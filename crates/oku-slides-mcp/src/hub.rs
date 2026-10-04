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
//! avec `state` déjà tenu.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::{PermissionChoice, ShowParams, ShowVariants, Surface, TabSession, forms};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::{broadcast, oneshot};

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
    /// Fichier de widget écrit ou supprimé par l'agent (mode modèles) : l'éditeur le suit.
    WidgetTouched {
        name: String,
        removed: bool,
    },
}

/// Réponse de l'utilisateur à une surface `ask_*`.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Action { name: String, context: Map<String, Value> },
    Cancelled,
}

/// Action d'une surface, telle que l'interface l'envoie à l'hôte.
#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceAction {
    pub surface: String,
    pub name: String,
    #[serde(default)]
    pub context: Map<String, Value>,
}

#[derive(Debug)]
struct PendingAsk {
    id: String,
    tx: oneshot::Sender<Answer>,
    /// Question du fil figée une fois répondue (`ask_then`) : elle garde sa place dans la liste.
    frozen: bool,
}

/// Ferme (ou fige) la surface `id` à la sortie de `ask`, y compris si le futur
/// qui l'attend est abandonné (agent annulé, appel MCP interrompu) : c'est le
/// seul endroit qui émet `CloseSurface` pour une question.
struct CloseOnDrop<'a> {
    hub: &'a SurfaceHub,
    id: String,
    /// Le tour de la question, s'il est du fil.
    turn: Option<u32>,
    freeze: Option<Freeze>,
    /// La réponse reçue ; sans elle (futur abandonné), la question est figée « annulée ».
    answer: Option<Answer>,
}

impl Drop for CloseOnDrop<'_> {
    fn drop(&mut self) {
        self.hub.state().asks.retain(|a| a.id != self.id);
        let id = std::mem::take(&mut self.id);
        let shown = match (self.freeze.take(), self.turn) {
            (Some(freeze), Some(turn)) => {
                self.hub.show_frozen(turn, freeze(self.answer.as_ref().unwrap_or(&Answer::Cancelled)))
            }
            _ => false,
        };
        if !shown {
            // Pas de gel (question ordinaire), ou son tour a disparu / l'onglet est fermé : fermée.
            self.hub.emit(McpEvent::CloseSurface { id });
        }
    }
}

/// Ce qu'il advient d'une surface persistante après une action.
enum After {
    Keep,
    Close,
    Freeze,
}

/// Surface persistante et ce qu'il faut pour appliquer ses actions.
#[derive(Debug)]
enum Shown {
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

#[derive(Debug, Default)]
struct HubState {
    next: u64,
    /// Dans l'ordre d'ouverture : une réponse dictée va à la dernière.
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
}

impl HubState {
    /// Retire la question `i` ; sa surface quitte aussitôt la liste des
    /// ouvertes (le `CloseSurface` de `CloseOnDrop` suit, sans effet de plus),
    /// sauf une question gelée : sa version figée la remplace à sa place.
    fn take_ask(&mut self, i: usize) -> PendingAsk {
        let ask = self.asks.remove(i);
        if !ask.frozen {
            self.open.retain(|s| s.id != ask.id);
        }
        ask
    }

    fn take_last_ask(&mut self) -> Option<PendingAsk> {
        let last = self.asks.len().checked_sub(1)?;
        Some(self.take_ask(last))
    }

    fn take_asks(&mut self) -> Vec<PendingAsk> {
        let asks = std::mem::take(&mut self.asks);
        self.open.retain(|s| !asks.iter().any(|a| !a.frozen && a.id == s.id));
        asks
    }
}

/// Les surfaces de l'agent : ouvertes, attendues, fermées.
#[derive(Debug)]
pub struct SurfaceHub {
    pub(crate) session: TabSession,
    pub(crate) localizer: Arc<Localizer>,
    events: broadcast::Sender<McpEvent>,
    state: Mutex<HubState>,
}

impl SurfaceHub {
    pub fn new(session: TabSession, localizer: Arc<Localizer>) -> Arc<Self> {
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
    pub fn emit(&self, mut event: McpEvent) {
        let mut state = self.state();
        match &mut event {
            // Le relais, en retard, peut émettre la surface d'un tour effacé par une nouvelle conversation.
            McpEvent::Surface { surface } if surface.turn.is_some_and(|t| !state.live_turns.contains(&t)) => {
                tracing::debug!(surface = %surface.id, "surface d'un tour effacé : ignorée");
                return;
            }
            McpEvent::Surface { surface } => match state.open.iter_mut().find(|s| s.id == surface.id) {
                Some(open) => *open = surface.clone(),
                None => state.open.push(surface.clone()),
            },
            McpEvent::CloseSurface { id } => state.open.retain(|s| &s.id != id),
            McpEvent::Surfaces { list } => *list = state.open.clone(),
            McpEvent::SurfacePatch { id, messages } => match state.open.iter_mut().find(|s| &s.id == id) {
                Some(open) => open.apply_patch(messages),
                None => tracing::debug!(surface = %id, "mise à jour d'une surface fermée : ignorée"),
            },
            McpEvent::Changed | McpEvent::Focus { .. } | McpEvent::WidgetTouched { .. } => {}
        }
        let _ = self.events.send(event);
    }

    /// Les surfaces ouvertes, dans l'ordre d'ouverture.
    pub fn surfaces(&self) -> Vec<Surface> {
        self.state().open.clone()
    }

    /// Envoie la liste entière des surfaces ouvertes (`McpEvent::Surfaces`).
    pub fn resync(&self) {
        self.emit(McpEvent::Surfaces { list: Vec::new() });
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

    /// Affiche `surface` et attend la réponse ; `dismiss` vaut `Cancelled`.
    /// Sûr à l'annulation : si ce futur est abandonné avant la réponse,
    /// `CloseOnDrop` retire la question et ferme la surface à sa place.
    /// Onglet fermé (`close`) : `Cancelled` tout de suite, sans rien afficher.
    /// Question hors du fil (fiche, fenêtre) : elle n'est rangée dans aucun tour.
    pub async fn ask(&self, surface: Surface) -> Answer {
        self.ask_with(None, surface, None).await
    }

    /// Question du fil posée par un appel du tour `turn` (`None` : arrivé hors tour) ;
    /// `freeze` la remplace, figée, dans ce tour. Si `turn` n'est plus le tour ouvert (appel
    /// tardif d'un tour clos), `Cancelled` tout de suite, sans rien afficher.
    pub async fn ask_then(&self, turn: Option<u32>, surface: Surface, freeze: Freeze) -> Answer {
        self.ask_with(turn, surface, Some(freeze)).await
    }

    async fn ask_with(&self, turn: Option<u32>, mut surface: Surface, freeze: Option<Freeze>) -> Answer {
        let (tx, rx) = oneshot::channel();
        let id = surface.id.clone();
        {
            let mut state = self.state();
            if state.closed || turn.is_some_and(|t| state.turn != Some(t)) {
                return Answer::Cancelled;
            }
            surface.turn = turn;
            state.asks.push(PendingAsk { id: id.clone(), tx, frozen: freeze.is_some() });
        }
        self.emit(McpEvent::Surface { surface });
        let mut guard = CloseOnDrop { hub: self, id, turn, freeze, answer: None };
        let answer = rx.await.unwrap_or(Answer::Cancelled);
        guard.answer = Some(answer.clone());
        answer
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
        for id in old {
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

    pub fn has_pending_ask(&self) -> bool {
        !self.state().asks.is_empty()
    }

    /// Une réponse dictée pour la dernière question en attente.
    pub fn answer_pending(&self, text: &str) -> bool {
        let Some(ask) = self.state().take_last_ask() else { return false };
        let mut context = Map::new();
        context.insert("text".into(), Value::String(text.to_owned()));
        ask.tx.send(Answer::Action { name: "answer".into(), context }).is_ok()
    }

    /// Libère toutes les questions en attente (Échap, nouveau tour, nouvelle conversation).
    pub fn cancel_asks(&self) {
        // Le verrou d'un temporaire dans la tête d'un `for` vit jusqu'à la
        // fin de la boucle : on le lie d'abord pour ne pas le tenir pendant
        // l'envoi (qui peut réveiller le futur en attente).
        let asks = self.state().take_asks();
        Self::release(asks);
    }

    /// L'onglet est fermé : les questions en attente sont relâchées, et toute
    /// question ultérieure (`ask`, `confirm`, `permission`) vaut `Cancelled`.
    pub fn close(&self) {
        // Sous le même verrou qu'`ask` : aucune question ne s'ouvre après.
        let asks = {
            let mut state = self.state();
            state.closed = true;
            state.take_asks()
        };
        Self::release(asks);
    }

    fn release(asks: Vec<PendingAsk>) {
        for ask in asks {
            let _ = ask.tx.send(Answer::Cancelled);
        }
    }

    /// Une action de l'interface : réponse à une question, ou changement
    /// d'une surface persistante (appliqué ici, noté pour l'agent).
    pub fn act(&self, action: SurfaceAction) -> Result<(), String> {
        let ask = {
            let mut state = self.state();
            let i = state.asks.iter().position(|a| a.id == action.surface);
            i.map(|i| state.take_ask(i))
        };
        if let Some(ask) = ask {
            let answer = if action.name == "dismiss" {
                Answer::Cancelled
            } else {
                Answer::Action { name: action.name, context: action.context }
            };
            let _ = ask.tx.send(answer);
            return Ok(());
        }
        let shown =
            self.state().shows.remove(&action.surface).ok_or_else(|| format!("unknown surface {}", action.surface))?;
        // Un échec (valeur refusée, slide disparue) ne doit pas faire perdre
        // la surface : elle est ré-insérée dans tous les cas d'erreur, pour
        // que l'utilisateur puisse corriger ou la fermer explicitement.
        let result = match (&shown, action.name.as_str()) {
            (Shown::Params(_), "dismiss") => Ok(After::Freeze),
            (_, "dismiss") => Ok(After::Close),
            (Shown::Params(spec), "param") => {
                self.apply_param(&action.surface, spec, &action.context).map(|()| After::Keep)
            }
            (Shown::Variants(spec), "variant") => self.apply_variant(spec, &action.context).map(|()| After::Close),
            (Shown::Static { expired: false, .. }, "suggest") => Ok(After::Keep),
            (Shown::Static { expired: true, .. }, "suggest") => Err("the actions of a past turn have expired".into()),
            (_, other) => Err(format!("unexpected action {other}")),
        };
        match result {
            Ok(After::Keep) => {
                self.state().shows.insert(action.surface, shown);
                Ok(())
            }
            Ok(After::Freeze) => {
                self.freeze(&action.surface);
                Ok(())
            }
            Ok(After::Close) => {
                self.emit(McpEvent::CloseSurface { id: action.surface });
                Ok(())
            }
            Err(e) => {
                self.state().shows.insert(action.surface, shown);
                Err(e)
            }
        }
    }

    fn index(context: &Map<String, Value>, key: &str) -> Result<usize, String> {
        context
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|i| usize::try_from(i).ok())
            .ok_or_else(|| format!("missing {key}"))
    }

    fn apply_param(&self, id: &str, spec: &ShowParams, context: &Map<String, Value>) -> Result<(), String> {
        let i = Self::index(context, "field")?;
        let field = spec.fields.get(i).ok_or_else(|| format!("no field {i}"))?;
        let value = context.get("value").cloned().unwrap_or(Value::Null);
        self.session
            .with(|s| -> Result<(), String> {
                let op = s.param_op(field, &value).map_err(|e| e.one_line())?;
                let label = self.localizer.tr_with("agent-undo-param", &[("label", field.label.as_str().into())]);
                s.apply(&label, vec![op], None).map_err(|e| e.one_line())?;
                s.note(format!("parameter: {:?} = {value} ({})", field.label, spec.slide));
                Ok(())
            })
            .map_err(|e| e.one_line())??;
        // L'interface a déjà la valeur ; la copie du hub la garde pour une resynchronisation.
        if let Some(open) = self.state().open.iter_mut().find(|s| s.id == id) {
            open.set_value(&forms::param_key(i), value);
        }
        self.emit(McpEvent::Changed);
        Ok(())
    }

    fn apply_variant(&self, spec: &ShowVariants, context: &Map<String, Value>) -> Result<(), String> {
        let i = Self::index(context, "index")?;
        let variant = spec.variants.get(i).ok_or_else(|| format!("no variant {i}"))?;
        self.session
            .with(|s| -> Result<(), String> {
                let label = self.localizer.tr_with("agent-undo-variant", &[("label", variant.label.as_str().into())]);
                s.apply(&label, variant.ops.clone(), None).map_err(|e| e.one_line())?;
                s.note(format!("variant picked: {:?} ({})", variant.label, spec.slide));
                Ok(())
            })
            .map_err(|e| e.one_line())??;
        self.emit(McpEvent::Changed);
        Ok(())
    }

    /// Ferme les surfaces dont la slide n'existe plus ; sans réglage vivant
    /// (`Params`, `Variants`), ne prend pas la session (appelée après chaque
    /// requête qui change le deck).
    pub fn prune(&self) {
        if !self.state().shows.values().any(|s| matches!(s, Shown::Params(_) | Shown::Variants(_))) {
            return;
        }
        let Ok(gone) = self.session.with(|s| {
            let deck = s.deck();
            let mut state = self.state();
            let gone: Vec<String> = state
                .shows
                .iter()
                .filter(|(_, shown)| {
                    let slide = match shown {
                        Shown::Params(p) => &p.slide,
                        Shown::Variants(v) => &v.slide,
                        Shown::Static { .. } => return false,
                    };
                    deck.slide(slide).is_none()
                })
                .map(|(id, _)| id.clone())
                .collect();
            for id in &gone {
                state.shows.remove(id);
            }
            gone
        }) else {
            return;
        };
        for id in gone {
            self.emit(McpEvent::CloseSurface { id });
        }
    }

    /// Confirmation d'un outil non annulable ; `true` seulement sur « confirmer ».
    pub async fn confirm(&self, message: String, danger: bool) -> bool {
        let id = self.next_id("confirm");
        let surface = forms::confirm(&id, &oku_slides_session::agent::AskConfirm { message, danger }, &self.localizer);
        matches!(
            self.ask(surface).await,
            Answer::Action { name, context } if name == "confirm" && context.get("confirmed") == Some(&Value::Bool(true))
        )
    }

    /// Permission demandée par l'agent (ACP) : l'index de l'option choisie, dans
    /// l'ordre de l'agent. Seul un clic sur une option répond : Échap, une
    /// nouvelle conversation ou une réponse dictée (`answer_pending`, nommée
    /// `answer`) donnent `None`, que la connexion transmet comme un refus.
    pub async fn permission(&self, title: &str, options: &[PermissionChoice]) -> Option<usize> {
        let id = self.next_id("permission");
        match self.ask(forms::permission(&id, title, options, &self.localizer)).await {
            Answer::Action { name, context } if name == "permission" => Self::index(&context, "index").ok(),
            _ => None,
        }
    }
}

#[path = "hub_turns.rs"]
mod turns;
pub use turns::{Freeze, STATIC_PER_TURN};
