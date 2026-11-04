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
//! A typed reply (`answer`) names its question: the interface picks it, the one that takes
//! its keys 1–4 (`quickTarget`). A host-routed utterance (`answer_voice`) goes to the hub's own
//! target, with the same priority: modal, sheet, dock question, then the variant drawer.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use oku_slides_core::GuideKind;
use oku_slides_i18n::LiveLocalizer;
use oku_slides_session::agent::{ShowParams, ShowVariants, SpokenText, Surface, TabSession, Trace, TurnOrigin};
use serde::Serialize;
use serde_json::{Map, Value};
use tokio::sync::{broadcast, watch};

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
        /// What the read-aloud says as it opens; host-only, never sent to the page.
        #[serde(skip)]
        spoken: Option<SpokenText>,
    },
    /// A surface closes: the interface removes it.
    CloseSurface {
        /// Id of the closed surface.
        id: String,
    },
    /// Show a slide, the agent never selects (the user's selection stays); no slide: deselect,
    /// the shown slide stays.
    Focus {
        /// Id of the slide to show; `None` keeps the shown one.
        slide: Option<String>,
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
    /// Start or steer the presentation in the tab.
    Present {
        /// What the presentation does.
        command: crate::PresentCommand,
    },
    /// Open the history as its own command would: input committed, save sent first.
    OpenHistory,
    /// Change the interface (scheme, language, guides) as its own commands would.
    Interface {
        /// The requested changes; absent means unchanged.
        change: crate::InterfaceChange,
    },
    /// Close a tab or a window through the page's own flows; sent live, never kept nor replayed.
    Close {
        /// What closes.
        target: CloseTarget,
        /// The id the page redeems once (`SurfaceHub::take_close_decision`) for its discard question.
        decision: String,
    },
}

impl McpEvent {
    /// A surface opening silently, or replacing the one with the same id.
    #[must_use]
    pub fn surface(surface: Surface) -> Self {
        Self::Surface { surface, spoken: None }
    }

    /// A surface opening with what the read-aloud says of it.
    #[must_use]
    pub fn surface_aloud(surface: Surface, spoken: Option<SpokenText>) -> Self {
        Self::Surface { surface, spoken }
    }
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

impl Answer {
    /// A reply given as text (`answer {text}`): typed, or spoken to the agent's own question.
    pub(crate) fn text(text: &str) -> Self {
        let mut context = Map::new();
        context.insert("text".into(), Value::String(text.to_owned()));
        Self::Action { name: "answer".into(), context }
    }
}

/// What asking the hub to show one of its own boxes did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowOutcome {
    /// The box is shown.
    Shown,
    /// Not shown: the tab is not in the box's mode (deck mode for themes and packs).
    OutOfMode,
    /// Not shown: the box's turn left the thread (evicted or cleared).
    TurnOver,
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

/// The shown theme picker: a pick or `prune` rebuilds it without rendering anything.
#[derive(Debug, Clone)]
struct ThemePicker {
    cards: Vec<oku_slides_session::ThemeCard>,
    active: oku_slides_core::Namespace,
    /// The agent turn that opened it (its chip); `None`: opened by the user.
    turn: Option<u32>,
}

/// The shown packs panel: `prune` resends it only when it changes.
#[derive(Debug, Clone)]
struct PacksShown {
    /// The last surface sent.
    last: Surface,
    /// The agent turn that opened it (its chip); `None`: opened by the user.
    turn: Option<u32>,
}

/// The shown settings box: a refresh goes out only when it changes.
#[derive(Debug, Clone)]
struct SettingsShown {
    /// The last panel sent.
    last: oku_slides_session::VoicePanel,
    /// The agent turn that opened it (its chip); `None`: opened by the user.
    turn: Option<u32>,
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
    /// The receipt lines of the agent's guide writes, by turn (`note_guide_write`).
    guide_writes: HashMap<u32, Vec<(GuideKind, String)>>,
    /// How many of a turn's guide writes a receipt or card already lists.
    guide_listed: HashMap<u32, usize>,
    /// The traces of an ended turn, for its fallback receipt (`note_turn_traces`).
    traces: HashMap<u32, Vec<Trace>>,
    /// The one tab move a turn may make, once claimed (`reserve_tab`).
    tab_claim: Option<turns::TabClaim>,
    /// The turn that moved to another tab (`note_elsewhere`).
    elsewhere: Option<u32>,
    /// Shown with every permission request of the turn (it carries a received guide).
    permission_note: Option<String>,
    /// The last turn begun and how it began (`begin_turn_from`).
    origin: Option<(u32, TurnOrigin)>,
    /// Where the tab's page presents, from its reports, with the host's number of the last one.
    presenting: Option<(u64, Option<crate::PresentMode>)>,
    /// What runs once a turn ends, recorded by its tools (`after_turn`), with that turn.
    after_turn: Option<(u32, Vec<turns::AfterTurn>)>,
    /// Confirmed closes, oldest first, until their page redeems them (`hub/close`).
    closes: Vec<close::StoredClose>,
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
    /// The last turn whose end is done, receipt included (`end_turn`); 0 before any.
    ended: watch::Sender<u32>,
}

impl SurfaceHub {
    /// Le hub de l'onglet de `session` ; `voice` fournit la boîte des réglages.
    pub fn new(session: TabSession, localizer: LiveLocalizer, voice: Arc<dyn VoicePort>) -> Arc<Self> {
        let (events, _) = broadcast::channel(64);
        let ended = watch::Sender::new(0);
        Arc::new(Self { session, localizer, voice, events, state: Mutex::new(HubState::default()), ended })
    }

    /// Waits until turn `turn` has ended, its receipt shown, `within` at most; false on timeout.
    pub async fn turn_ended(&self, turn: u32, within: std::time::Duration) -> bool {
        let mut rx = self.ended.subscribe();
        matches!(tokio::time::timeout(within, rx.wait_for(|last| *last >= turn)).await, Ok(Ok(_)))
    }

    /// Notes that turn `turn` has ended, receipt included; an older turn changes nothing.
    pub(crate) fn note_ended(&self, turn: u32) {
        self.ended.send_if_modified(|last| {
            let newer = turn > *last;
            if newer {
                *last = turn;
            }
            newer
        });
    }

    /// Sets the note every permission request shows, `None` clears it (turn start and end).
    pub fn set_permission_note(&self, note: Option<String>) {
        self.state().permission_note = note;
    }

    /// Records where the tab's page presents (`None`: it does not), unless `seq` is not newer.
    ///
    /// The host numbers these writes after ordering the page's reports by (page, seq) itself;
    /// a write that lost a race to a newer one is ignored.
    pub fn set_presenting(&self, mode: Option<crate::PresentMode>, seq: u64) {
        let mut state = self.state();
        if state.presenting.is_some_and(|(last, _)| seq <= last) {
            return;
        }
        state.presenting = Some((seq, mode));
    }

    /// Where the tab's page last said it presents; `None` when it does not.
    #[must_use]
    pub fn presenting(&self) -> Option<crate::PresentMode> {
        self.state().presenting.and_then(|(_, mode)| mode)
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
            McpEvent::Surface { surface, .. } if surface.turn.is_some_and(|t| !state.live_turns.contains(&t)) => {
                tracing::debug!(surface = %surface.id, "surface d'un tour effacé : ignorée");
                return;
            }
            McpEvent::Surface { surface, .. } => {
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
            | McpEvent::Close { .. }
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
        self.emit(McpEvent::surface(surface));
        true
    }

    /// True while a variant drawer is shown: a spoken pick may answer it.
    #[must_use]
    pub fn has_variants(&self) -> bool {
        self.state().shows.values().any(|s| matches!(s, Shown::Variants(_)))
    }

    /// Tiroir de variantes ; remplace le précédent.
    pub fn show_variants(&self, surface: Surface, spec: ShowVariants) {
        self.show_alone(surface, Shown::Variants(spec), |s| matches!(s, Shown::Variants(_)));
    }

    /// The turn an agent-opened box is filed in: the call's turn, else the thread's last one.
    ///
    /// `turn` is the one read when the call arrived ([`turn`](Self::turn)): a late call between
    /// turns still shows as a chip, in the last turn of the thread. `None`: no turn at all.
    pub fn agent_turn(&self, turn: Option<u32>) -> Option<u32> {
        turn.or_else(|| self.state().live_turns.last().copied())
    }

    /// Shows `surface` in place of the surfaces of the same family (`same`).
    ///
    /// `TurnOver`, and nothing shown, when the surface's turn is no longer in the thread.
    fn show_alone(&self, surface: Surface, shown: Shown, same: impl Fn(&Shown) -> bool) -> ShowOutcome {
        let old = {
            let mut state = self.state();
            if surface.turn.is_some_and(|t| !state.live_turns.contains(&t)) {
                return ShowOutcome::TurnOver;
            }
            let ids: Vec<String> = state.shows.iter().filter(|(_, s)| same(s)).map(|(id, _)| id.clone()).collect();
            for id in &ids {
                state.shows.remove(id);
            }
            state.shows.insert(surface.id.clone(), shown);
            ids
        };
        for id in old.into_iter().filter(|id| *id != surface.id) {
            self.emit(McpEvent::CloseSurface { id });
        }
        self.emit(McpEvent::surface(surface));
        ShowOutcome::Shown
    }
}

mod actions;
pub use actions::SurfaceAction;

mod asks;
use asks::PendingAsk;

mod voice_answer;
pub use voice_answer::{ARMING, AnswerOutcome};

mod themes;

mod close;
pub use close::{CLOSE_DECISION_TTL, CloseDecision, CloseTarget};

mod packs;

mod settings;

mod error;
pub use error::{HubError, HubErrorKind};

#[path = "hub_turns.rs"]
mod turns;
pub use turns::{AfterTurn, Freeze, STATIC_PER_TURN};
pub(crate) use turns::{Reserve, TabTarget};

#[path = "hub_thread.rs"]
mod thread;
pub use thread::THREAD_BYTES;

#[cfg(test)]
#[path = "hub_tests.rs"]
pub(crate) mod tests;
