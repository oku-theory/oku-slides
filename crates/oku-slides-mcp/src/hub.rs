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
use oku_slides_session::agent::{AgentScope, ShowParams, ShowVariants, Surface, TabSession};
use serde::Serialize;
use serde_json::{Map, Value};
use tokio::sync::broadcast;

/// Évènements vers l'interface (Tauri `emit`, SSE).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpEvent {
    /// Le deck ou la session a changé : relire l'instantané.
    Changed,
    Surface {
        surface: Surface,
    },
    CloseSurface {
        id: String,
    },
    /// Montrer une slide (et un élément).
    Focus {
        slide: String,
        element: Option<String>,
    },
    /// Toutes les surfaces ouvertes, dans l'ordre d'ouverture : l'interface
    /// remplace sa liste (après un retard, une reconnexion, un abonnement).
    /// `emit` remplit `list` lui-même : la liste envoyée est toujours celle du hub.
    Surfaces {
        list: Vec<Surface>,
    },
    /// Messages qui prolongent une surface ouverte (progression d'un tour).
    SurfacePatch {
        id: String,
        messages: Vec<oku_slides_a2ui::ServerMessage>,
    },
    /// Fichier de widget écrit ou supprimé par l'agent (mode pack) : l'éditeur le suit.
    WidgetTouched {
        name: String,
        removed: bool,
    },
    /// Lancer la présentation dans l'onglet.
    Present {
        /// Écran ou fenêtre.
        mode: crate::PresentMode,
    },
    /// Ouvrir l'historique comme sa propre commande : saisie validée, enregistrement parti d'abord.
    OpenHistory,
    /// Changer l'interface (schéma, langue, grille, repères) comme ses propres commandes.
    Interface {
        /// Les changements demandés ; absent : inchangé.
        change: crate::InterfaceChange,
    },
    /// La portée dont le fil est montré.
    ///
    /// Une autre vue du même onglet (la fenêtre de l'agent) la suit.
    Viewing {
        /// La portée montrée.
        scope: AgentScope,
    },
}

/// Réponse de l'utilisateur à une surface `ask_*`.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Action { name: String, context: Map<String, Value> },
    Cancelled,
}

/// Surface persistante et ce qu'il faut pour appliquer ses actions.
#[derive(Debug)]
enum Shown {
    /// Sélecteur de thème de l'hôte : ses cartes rendues et l'actif montré.
    Themes(ThemePicker),
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
    active: uuid::Uuid,
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
    /// Tours dont les surfaces sont encore dans le fil (ouverts, ni évincés ni effacés).
    live_turns: BTreeSet<u32>,
    /// Tours qui ont leur reçu (`show_report`) : pas de reçu de repli pour eux.
    reported: HashSet<u32>,
    /// Portée de chaque tour encore dans le fil.
    turn_scopes: HashMap<u32, AgentScope>,
    /// La portée que l'interface montre : seul son fil lui est envoyé.
    viewing: AgentScope,
}

/// Les surfaces de l'agent : ouvertes, attendues, fermées.
#[derive(Debug)]
pub struct SurfaceHub {
    pub(crate) session: TabSession,
    /// Langue courante de l'interface, relue à chaque texte.
    pub(crate) localizer: LiveLocalizer,
    events: broadcast::Sender<McpEvent>,
    state: Mutex<HubState>,
}

impl SurfaceHub {
    pub fn new(session: TabSession, localizer: LiveLocalizer) -> Arc<Self> {
        let (events, _) = broadcast::channel(64);
        Arc::new(Self { session, localizer, events, state: Mutex::new(HubState::default()) })
    }

    /// Identifiant de l'onglet dont ce hub sert les surfaces.
    pub fn tab(&self) -> &str {
        self.session.tab()
    }

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
                let was = state.open.iter().find(|s| s.id == surface.id).map(|s| (s.live, s.turn));
                let was = was.is_some_and(|(live, turn)| Self::shown(&*state, live, turn));
                match state.open.iter_mut().find(|s| s.id == surface.id) {
                    Some(open) => *open = surface.clone(),
                    None => state.open.push(surface.clone()),
                }
                let now = Self::shown(&*state, surface.live, surface.turn);
                if was && !now {
                    // Montrée, maintenant cachée (question d'une autre portée figée) : l'interface la ferme.
                    event = McpEvent::CloseSurface { id: surface.id.clone() };
                    true
                } else {
                    now
                }
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
                let seen = state.open.iter_mut().find(|s| &s.id == id).map(|open| {
                    open.apply_patch(messages);
                    (open.live, open.turn)
                });
                match seen {
                    Some((live, turn)) => Self::shown(&*state, live, turn),
                    None => {
                        tracing::debug!(surface = %id, "mise à jour d'une surface fermée : ignorée");
                        true
                    }
                }
            }
            McpEvent::Changed
            | McpEvent::Focus { .. }
            | McpEvent::Present { .. }
            | McpEvent::OpenHistory
            | McpEvent::Interface { .. }
            | McpEvent::WidgetTouched { .. }
            | McpEvent::Viewing { .. } => true,
        };
        if send {
            let _ = self.events.send(event);
        }
    }

    /// Les surfaces ouvertes, dans l'ordre d'ouverture.
    pub fn surfaces(&self) -> Vec<Surface> {
        self.state().open.clone()
    }

    /// Montre le fil de `scope`, piloté par le gardien de l'onglet.
    ///
    /// Annonce la portée (`McpEvent::Viewing`), puis envoie la liste entière (`McpEvent::Surfaces`).
    pub fn resync(&self, scope: AgentScope) {
        let mut state = self.state();
        state.viewing = scope.clone();
        self.record_and_send(&mut state, McpEvent::Viewing { scope });
        self.record_and_send(&mut state, McpEvent::Surfaces { list: Vec::new() });
    }

    /// La portée dont le fil est montré.
    pub fn viewing(&self) -> AgentScope {
        self.state().viewing.clone()
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

mod error;
pub use error::{HubError, HubErrorKind};

#[path = "hub_turns.rs"]
mod turns;
pub use turns::{Freeze, STATIC_PER_TURN};

#[path = "hub_scopes.rs"]
mod scopes;
pub use scopes::THREAD_BYTES;

#[cfg(test)]
#[path = "hub_tests.rs"]
mod tests;
