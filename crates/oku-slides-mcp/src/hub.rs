//! Surfaces ouvertes par l'agent et évènements vers l'interface.
//!
//! Deux familles de surfaces : les questions `ask_*` (oneshot, une réponse
//! puis fermeture) et les surfaces persistantes `show_*` (surfaces de
//! paramètres, tiroir de variantes) qui restent ouvertes et appliquent leurs
//! actions directement. Les suites (`report`, `render`) sont des surfaces
//! figées dont le bouton `suggest` renvoie sa consigne à l'agent par l'interface.
//!
//! Ordre des verrous : jamais `state` tenu pendant un `.await`, ni en même
//! temps que la session. `prune` prend la session puis `state` ; `act`
//! relâche `state` avant d'appliquer quoi que ce soit sur la session.
//! `emit` prend `state` (liste des surfaces ouvertes) : jamais appelé
//! avec `state` déjà tenu (sous le verrou : `record_and_send`).
//!
//! Une consigne qui répond à une question (`answer`) nomme cette question :
//! l'interface la désigne, la même qui reçoit ses touches 1–4 (`quickTarget`).
//! Le hub ne devine pas laquelle, il ne voit pas la slide affichée.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use oku_slides_i18n::LiveLocalizer;
use oku_slides_session::agent::{ShowParams, ShowVariants, Surface, TabSession};
use serde::Serialize;
use serde_json::{Map, Value};
use tokio::sync::broadcast;

use crate::VoicePort;

/// Events sent to the interface (Tauri `emit`, SSE).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpEvent {
    /// The deck or the session changed: read the snapshot again.
    Changed,
    /// A surface opens, or replaces the one with the same id.
    Surface {
        /// The surface to show.
        surface: Surface,
    },
    /// A surface closes: the interface removes it.
    CloseSurface {
        /// Id of the closed surface.
        id: String,
    },
    /// Show a slide (and an element).
    Focus {
        /// Id of the slide to show.
        slide: String,
        /// Id of the element to select, if any.
        element: Option<String>,
    },
    /// Every open surface, in opening order.
    ///
    /// The interface replaces its list (after a lag, a reconnection, a subscription).
    /// `emit` fills `list` itself: the list sent is always the hub's.
    Surfaces {
        /// The open surfaces, in opening order.
        list: Vec<Surface>,
    },
    /// Messages that extend an open surface (progress of a turn).
    SurfacePatch {
        /// Id of the extended surface.
        id: String,
        /// A2UI messages to apply, in order.
        messages: Vec<oku_slides_a2ui::ServerMessage>,
    },
    /// Widget file written or deleted by the agent (pack mode): the editor follows it.
    WidgetTouched {
        /// Name of the touched widget file.
        name: String,
        /// True if the file was deleted, false if it was written.
        removed: bool,
    },
    /// Start the presentation in the tab.
    Present {
        /// Screen or window.
        mode: crate::PresentMode,
    },
    /// Open the history as its own command would: input committed, save sent first.
    OpenHistory,
    /// Change the interface (scheme, language, grid, guides) as its own commands would.
    Interface {
        /// The requested changes; absent means unchanged.
        change: crate::InterfaceChange,
    },
}

/// The user's answer to an `ask_*` surface.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// The user chose one of the surface's actions.
    Action {
        /// Name of the chosen action.
        name: String,
        /// Form values at the time of the choice.
        context: Map<String, Value>,
    },
    /// Surface abandoned: Escape, new turn, tab or conversation closed, or its turn already over.
    Cancelled,
}

/// Surface persistante et ce qu'il faut pour appliquer ses actions.
#[derive(Debug)]
enum Shown {
    /// Sélecteur de thème de l'hôte : ses cartes rendues et l'actif montré.
    Themes(ThemePicker),
    /// Panneau des packs du deck : la dernière surface montrée.
    Packs(PacksShown),
    /// Boîte des réglages de la dictée et de l'agent : le dernier panneau envoyé.
    Settings(SettingsShown),
    Params(ShowParams),
    Variants(ShowVariants),
    /// Figée (question répondue, composition `render`) : seules ses suites (`suggest`) partent
    /// encore, et seulement tant que son tour est le dernier. `plain` : sa version sans actions,
    /// affichée quand elles expirent.
    Static {
        turn: Option<u32>,
        plain: Option<Surface>,
        expired: bool,
        report: bool,
    },
}

/// Le sélecteur de thème montré : un choix ou `prune` le refait sans rien rendre.
#[derive(Debug, Clone)]
struct ThemePicker {
    cards: Vec<oku_slides_session::ThemeCard>,
    active: oku_slides_core::Namespace,
}

/// Le panneau des packs montré : `prune` ne le renvoie que s'il change.
#[derive(Debug, Clone)]
struct PacksShown {
    /// La dernière surface envoyée.
    last: Surface,
}

/// La boîte des réglages montrée : un rafraîchissement ne part que si elle change.
#[derive(Debug, Clone)]
struct SettingsShown {
    /// Le dernier panneau envoyé.
    last: oku_slides_session::VoicePanel,
}

#[derive(Debug, Default)]
struct HubState {
    next: u64,
    /// Questions en attente, dans l'ordre d'ouverture.
    asks: Vec<PendingAsk>,
    shows: HashMap<String, Shown>,
    /// Surfaces ouvertes, dans l'ordre d'ouverture. Ouverture et fermeture
    /// passent par `emit` ; une question répondue en sort dès la réponse, un
    /// réglage changé y est recopié (`act`).
    open: Vec<Surface>,
    /// L'onglet est fermé (`SurfaceHub::close`) : plus aucune question ne s'ouvre.
    closed: bool,
    /// Le tour en cours ; `None` entre deux tours (une surface du fil y est alors refusée).
    turn: Option<u32>,
    /// Le dernier numéro de tour donné.
    turns: u32,
    /// Turns of the tab's thread (open, neither evicted nor cleared).
    ///
    /// Only their surfaces are shown.
    live_turns: BTreeSet<u32>,
    /// Tours qui ont leur reçu (`show_report`) : pas de reçu de repli pour eux.
    reported: HashSet<u32>,
    /// The one new pack tab a turn may open, once claimed (`reserve_pack_tab`).
    pack_tab: Option<turns::PackTabClaim>,
    /// The turn whose `edit_pack` focused another tab (`note_elsewhere`).
    elsewhere: Option<u32>,
}

/// Les surfaces de l'agent : ouvertes, attendues, fermées.
#[derive(Debug)]
pub struct SurfaceHub {
    pub(crate) session: TabSession,
    /// Langue courante de l'interface, relue à chaque texte.
    pub(crate) localizer: LiveLocalizer,
    /// La dictée et l'agent de l'app : la boîte des réglages.
    voice: Arc<dyn VoicePort>,
    events: broadcast::Sender<McpEvent>,
    state: Mutex<HubState>,
}

impl SurfaceHub {
    /// Le hub de l'onglet de `session` ; `voice` fournit la boîte des réglages.
    pub fn new(session: TabSession, localizer: LiveLocalizer, voice: Arc<dyn VoicePort>) -> Arc<Self> {
        let (events, _) = broadcast::channel(64);
        Arc::new(Self { session, localizer, voice, events, state: Mutex::new(HubState::default()) })
    }

    /// Identifiant de l'onglet dont ce hub sert les surfaces.
    pub fn tab(&self) -> &str {
        self.session.tab()
    }

    /// Le port de la dictée et de l'agent de l'app, pour les outils.
    pub(crate) fn voice(&self) -> &Arc<dyn VoicePort> {
        &self.voice
    }

    /// One more subscriber to the events sent to the interface.
    pub fn subscribe(&self) -> broadcast::Receiver<McpEvent> {
        self.events.subscribe()
    }

    /// Envoie un évènement à l'interface ; personne à l'écoute n'est pas une erreur.
    ///
    /// Tient aussi la liste des surfaces ouvertes, et remplit celle d'un
    /// `Surfaces` : liste et évènement sont faits sous le même verrou, dans
    /// le même ordre.
    pub fn emit(&self, event: McpEvent) {
        self.record_and_send(&mut self.state(), event);
    }

    /// [`emit`](Self::emit), verrou déjà pris : la liste des ouvertes et l'évènement suivent
    /// l'ordre des sections critiques.
    fn record_and_send(&self, state: &mut HubState, mut event: McpEvent) {
        let send = match &mut event {
            // Le relais, en retard, peut émettre la surface d'un tour effacé par une nouvelle conversation.
            McpEvent::Surface { surface } if surface.turn.is_some_and(|t| !state.live_turns.contains(&t)) => {
                tracing::debug!(surface = %surface.id, "surface d'un tour effacé : ignorée");
                return;
            }
            McpEvent::Surface { surface } => {
                match state.open.iter_mut().find(|s| s.id == surface.id) {
                    Some(open) => *open = surface.clone(),
                    None => state.open.push(surface.clone()),
                }
                true
            }
            McpEvent::CloseSurface { id } => {
                state.open.retain(|s| &s.id != id);
                true
            }
            McpEvent::Surfaces { list } => {
                *list = Self::visible(&*state);
                true
            }
            McpEvent::SurfacePatch { id, messages } => {
                if let Some(open) = state.open.iter_mut().find(|s| &s.id == id) {
                    open.apply_patch(messages);
                } else {
                    tracing::debug!(surface = %id, "mise à jour d'une surface fermée : ignorée");
                }
                true
            }
            McpEvent::Changed
            | McpEvent::Focus { .. }
            | McpEvent::Present { .. }
            | McpEvent::OpenHistory
            | McpEvent::Interface { .. }
            | McpEvent::WidgetTouched { .. } => true,
        };
        if send {
            let _ = self.events.send(event);
        }
    }

    /// Les surfaces ouvertes, dans l'ordre d'ouverture.
    pub fn surfaces(&self) -> Vec<Surface> {
        self.state().open.clone()
    }

    /// Renvoie la liste entière des surfaces montrées (`McpEvent::Surfaces`) ; piloté par le gardien.
    pub fn resync(&self) {
        self.record_and_send(&mut self.state(), McpEvent::Surfaces { list: Vec::new() });
    }

    /// Identifiant de surface neuf : `ask-3`, `show-4`…
    pub fn next_id(&self, prefix: &str) -> String {
        let mut state = self.state();
        state.next += 1;
        format!("{prefix}-{}", state.next)
    }

    fn state(&self) -> std::sync::MutexGuard<'_, HubState> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Réglages du fil, posés par un appel du tour `turn` (`None` : arrivé hors tour) ; ils
    /// restent vivants jusqu'à ce qu'on les fige, et figent les précédents.
    ///
    /// Refusés (`false`) si `turn` n'est plus le tour ouvert (appel tardif d'un tour clos).
    pub fn show_params(&self, turn: Option<u32>, mut surface: Surface, spec: ShowParams) -> bool {
        let previous: Vec<String> = {
            let mut state = self.state();
            if turn.is_some_and(|t| state.turn != Some(t)) {
                return false;
            }
            surface.turn = turn;
            let previous =
                state.shows.iter().filter(|(_, s)| matches!(s, Shown::Params(_))).map(|(id, _)| id.clone()).collect();
            state.shows.insert(surface.id.clone(), Shown::Params(spec));
            previous
        };
        for id in previous {
            self.freeze(&id);
        }
        self.emit(McpEvent::Surface { surface });
        true
    }

    /// Tiroir de variantes ; remplace le précédent.
    pub fn show_variants(&self, surface: Surface, spec: ShowVariants) {
        self.show_alone(surface, Shown::Variants(spec), |s| matches!(s, Shown::Variants(_)));
    }

    /// Affiche `surface` à la place des surfaces de la même famille (`same`).
    fn show_alone(&self, surface: Surface, shown: Shown, same: impl Fn(&Shown) -> bool) {
        let old = self.take_shown(same);
        self.state().shows.insert(surface.id.clone(), shown);
        for id in old.into_iter().filter(|id| *id != surface.id) {
            self.emit(McpEvent::CloseSurface { id });
        }
        self.emit(McpEvent::Surface { surface });
    }

    /// Retire des surfaces persistantes celles que `which` désigne ; rend leurs identifiants.
    fn take_shown(&self, which: impl Fn(&Shown) -> bool) -> Vec<String> {
        let mut state = self.state();
        let ids: Vec<String> = state.shows.iter().filter(|(_, s)| which(s)).map(|(id, _)| id.clone()).collect();
        for id in &ids {
            state.shows.remove(id);
        }
        ids
    }
}

mod actions;
pub use actions::SurfaceAction;

mod asks;
use asks::PendingAsk;

mod themes;

mod packs;

mod settings;

mod error;
pub use error::{HubError, HubErrorKind};

#[path = "hub_turns.rs"]
mod turns;
pub(crate) use turns::Reserve;
pub use turns::{Freeze, STATIC_PER_TURN};

#[path = "hub_thread.rs"]
mod thread;
pub use thread::THREAD_BYTES;

#[cfg(test)]
#[path = "hub_tests.rs"]
mod tests;
