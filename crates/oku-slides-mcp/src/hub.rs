//! Surfaces ouvertes par l'agent et évènements vers l'interface.
//!
//! Deux familles de surfaces : les questions `ask_*` (oneshot, une réponse
//! puis fermeture) et les surfaces persistantes `show_*` (post-its de
//! paramètres, tiroir de variantes) qui restent ouvertes et appliquent leurs
//! actions directement. Les suites (`suggest_next`) sont persistantes aussi,
//! jusqu'au clic ou au tour suivant (`clear_suggestions`) ; l'interface
//! envoie elle-même la consigne choisie à l'agent.
//!
//! Ordre des verrous : jamais `state` tenu pendant un `.await`, ni en même
//! temps que la session. `prune` prend la session puis `state` ; `act`
//! relâche `state` avant d'appliquer quoi que ce soit sur la session.
//! `emit` prend `state` (liste des surfaces ouvertes) : jamais appelé
//! avec `state` déjà tenu.

use std::collections::HashMap;
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
}

/// Ferme la surface `id` à la sortie de `ask`, y compris si le futur qui
/// l'attend est abandonné (agent annulé, appel MCP interrompu) : c'est le
/// seul endroit qui émet `CloseSurface` pour une question.
struct CloseOnDrop<'a> {
    hub: &'a SurfaceHub,
    id: String,
}

impl Drop for CloseOnDrop<'_> {
    fn drop(&mut self) {
        self.hub.state().asks.retain(|a| a.id != self.id);
        self.hub.emit(McpEvent::CloseSurface { id: std::mem::take(&mut self.id) });
    }
}

/// Surface persistante et ce qu'il faut pour appliquer ses actions.
#[derive(Debug)]
enum Shown {
    Params(ShowParams),
    Variants(ShowVariants),
    /// Les suites proposées ; rien à appliquer, le clic part vers l'agent.
    Suggestions,
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
}

impl HubState {
    /// Retire la question `i` ; sa surface quitte aussitôt la liste des
    /// ouvertes (le `CloseSurface` de `CloseOnDrop` suit, sans effet de plus).
    fn take_ask(&mut self, i: usize) -> PendingAsk {
        let ask = self.asks.remove(i);
        self.open.retain(|s| s.id != ask.id);
        ask
    }

    fn take_last_ask(&mut self) -> Option<PendingAsk> {
        let last = self.asks.len().checked_sub(1)?;
        Some(self.take_ask(last))
    }

    fn take_asks(&mut self) -> Vec<PendingAsk> {
        let asks = std::mem::take(&mut self.asks);
        self.open.retain(|s| !asks.iter().any(|a| a.id == s.id));
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
            McpEvent::Surface { surface } => match state.open.iter_mut().find(|s| s.id == surface.id) {
                Some(open) => *open = surface.clone(),
                None => state.open.push(surface.clone()),
            },
            McpEvent::CloseSurface { id } => state.open.retain(|s| &s.id != id),
            McpEvent::Surfaces { list } => *list = state.open.clone(),
            McpEvent::Changed | McpEvent::Focus { .. } => {}
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
    pub async fn ask(&self, surface: Surface) -> Answer {
        let (tx, rx) = oneshot::channel();
        let id = surface.id.clone();
        {
            let mut state = self.state();
            if state.closed {
                return Answer::Cancelled;
            }
            state.asks.push(PendingAsk { id: id.clone(), tx });
        }
        self.emit(McpEvent::Surface { surface });
        let _guard = CloseOnDrop { hub: self, id };
        rx.await.unwrap_or(Answer::Cancelled)
    }

    /// Post-it de paramètres ; il reste jusqu'à sa fermeture.
    pub fn show_params(&self, surface: Surface, spec: ShowParams) {
        self.state().shows.insert(surface.id.clone(), Shown::Params(spec));
        self.emit(McpEvent::Surface { surface });
    }

    /// Tiroir de variantes ; remplace le précédent.
    pub fn show_variants(&self, surface: Surface, spec: ShowVariants) {
        self.show_alone(surface, Shown::Variants(spec), |s| matches!(s, Shown::Variants(_)));
    }

    /// Suites de la réponse ; remplacent les précédentes.
    pub fn show_suggestions(&self, surface: Surface) {
        self.show_alone(surface, Shown::Suggestions, |s| matches!(s, Shown::Suggestions));
    }

    /// Ferme les suites proposées : un nouveau tour les rend caduques.
    pub fn clear_suggestions(&self) {
        for id in self.take_shown(|s| matches!(s, Shown::Suggestions)) {
            self.emit(McpEvent::CloseSurface { id });
        }
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
            (_, "dismiss") => Ok(false),
            (Shown::Params(spec), "param") => self.apply_param(&action.surface, spec, &action.context).map(|()| true),
            (Shown::Variants(spec), "variant") => self.apply_variant(spec, &action.context).map(|()| false),
            (Shown::Suggestions, "suggest") => Ok(false),
            (_, other) => Err(format!("unexpected action {other}")),
        };
        match result {
            Ok(true) => {
                self.state().shows.insert(action.surface, shown);
                Ok(())
            }
            Ok(false) => {
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
                s.apply(&label, vec![op]).map_err(|e| e.one_line())?;
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
                s.apply(&label, variant.ops.clone()).map_err(|e| e.one_line())?;
                s.note(format!("variant picked: {:?} ({})", variant.label, spec.slide));
                Ok(())
            })
            .map_err(|e| e.one_line())??;
        self.emit(McpEvent::Changed);
        Ok(())
    }

    /// Ferme les post-its dont la slide n'existe plus ; sans post-it, ne
    /// prend pas la session (appelée après chaque requête qui change le deck).
    pub fn prune(&self) {
        if self.state().shows.is_empty() {
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
                        Shown::Suggestions => return false,
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
