//! Pending questions (`ask_*`, confirmations, permissions): opening, answering, release.

use std::time::Instant;

use oku_slides_session::agent::{PermissionChoice, Place, SpokenText, Surface, forms};
use serde_json::Value;
use tokio::sync::oneshot;

use super::{Answer, AnswerOutcome, Freeze, HubState, McpEvent, SurfaceHub};
use crate::{ConfirmAsk, ConfirmVerb};

/// What a pending question is: it says what may answer it.
#[derive(Debug, Clone)]
pub(super) enum AskKind {
    /// The agent's own question (`ask_*`): a typed or spoken reply answers it as text.
    Agent,
    /// A guarded confirmation: only a click or a host-routed utterance answers it.
    Confirm {
        /// The question, as shown.
        message: String,
        /// The verb a danger confirmation's button shows and the user must say.
        verb: Option<ConfirmVerb>,
    },
    /// An ACP permission: a click, or a host-routed utterance by option kind, never label.
    Permission {
        /// The title, as the agent gave it.
        title: String,
        /// The options in the agent's order.
        options: Vec<PermissionChoice>,
        /// The note shown under the title.
        note: Option<String>,
    },
}

#[derive(Debug)]
pub(super) struct PendingAsk {
    pub(super) id: String,
    pub(super) tx: oneshot::Sender<Answer>,
    /// A thread question frozen once answered (`ask_then`): it keeps its place in the list.
    frozen: bool,
    /// What it is, and so what may answer it.
    pub(super) kind: AskKind,
    /// Where the interface shows it: the voice target follows the same priority.
    pub(super) place: Place,
    /// The slide a sheet is anchored on: the page stages it only on that slide.
    pub(super) slide: Option<String>,
    /// When it became answerable by voice: when it was shown.
    pub(super) armed_from: Instant,
}

impl PendingAsk {
    /// Its surface again, with `line` under the buttons; `None` for the agent's own questions.
    pub(super) fn with_line(&self, line: &str, l: &oku_slides_i18n::Localizer) -> Option<Surface> {
        match &self.kind {
            AskKind::Agent => None,
            AskKind::Confirm { message, verb } => {
                Some(forms::confirm(&self.id, message, verb.map(|v| l.tr(v.label_key())), Some(line), l))
            }
            AskKind::Permission { title, options, note } => {
                Some(forms::permission(&self.id, title, options, note.as_deref(), Some(line), l))
            }
        }
    }
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

impl HubState {
    /// Retire la question `i` ; sa surface quitte aussitôt la liste des
    /// ouvertes (le `CloseSurface` de `CloseOnDrop` suit, sans effet de plus),
    /// sauf une question gelée : sa version figée la remplace à sa place.
    pub(super) fn take_ask(&mut self, i: usize) -> PendingAsk {
        let ask = self.asks.remove(i);
        if !ask.frozen {
            self.open.retain(|s| s.id != ask.id);
        }
        ask
    }

    /// Retire la question `id` si elle attend encore.
    pub(super) fn take_ask_id(&mut self, id: &str) -> Option<PendingAsk> {
        let i = self.asks.iter().position(|a| a.id == id)?;
        Some(self.take_ask(i))
    }

    fn take_asks_where(&mut self, which: impl Fn(&str) -> bool) -> Vec<PendingAsk> {
        let (taken, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.asks).into_iter().partition(|a| which(&a.id));
        self.asks = kept;
        self.open.retain(|s| !taken.iter().any(|a| !a.frozen && a.id == s.id));
        taken
    }

    fn take_asks(&mut self) -> Vec<PendingAsk> {
        self.take_asks_where(|_| true)
    }
}

impl SurfaceHub {
    /// Affiche `surface` et attend la réponse ; `dismiss` vaut `Cancelled`.
    /// Sûr à l'annulation : si ce futur est abandonné avant la réponse,
    /// `CloseOnDrop` retire la question et ferme la surface à sa place.
    /// Onglet fermé (`close`) : `Cancelled` tout de suite, sans rien afficher.
    /// Question hors du fil (fiche, fenêtre) : elle n'est rangée dans aucun tour.
    pub async fn ask(&self, surface: Surface) -> Answer {
        self.ask_with(None, surface, None, AskKind::Agent, None).await
    }

    /// Question du fil posée par un appel du tour `turn` (`None` : arrivé hors tour) ;
    /// `freeze` la remplace, figée, dans ce tour. Si `turn` n'est plus le tour ouvert (appel
    /// tardif d'un tour clos), `Cancelled` tout de suite, sans rien afficher.
    pub async fn ask_then(&self, turn: Option<u32>, surface: Surface, freeze: Freeze) -> Answer {
        self.ask_with(turn, surface, Some(freeze), AskKind::Agent, None).await
    }

    /// The agent's question read aloud as it opens: [`ask_then`](Self::ask_then) with `freeze`,
    /// else [`ask`](Self::ask) (`turn` is then `None`).
    pub async fn ask_aloud(
        &self,
        turn: Option<u32>,
        surface: Surface,
        freeze: Option<Freeze>,
        spoken: SpokenText,
    ) -> Answer {
        self.ask_with(turn, surface, freeze, AskKind::Agent, Some(spoken)).await
    }

    async fn ask_with(
        &self,
        turn: Option<u32>,
        mut surface: Surface,
        freeze: Option<Freeze>,
        kind: AskKind,
        spoken: Option<SpokenText>,
    ) -> Answer {
        let (tx, rx) = oneshot::channel();
        let id = surface.id.clone();
        {
            let mut state = self.state();
            if state.closed || turn.is_some_and(|t| state.turn != Some(t)) {
                return Answer::Cancelled;
            }
            surface.turn = turn;
            let ask = PendingAsk {
                id: id.clone(),
                tx,
                frozen: freeze.is_some(),
                kind,
                place: surface.place,
                slide: surface.anchor.as_ref().map(|a| a.slide.clone()),
                armed_from: Instant::now(),
            };
            state.asks.push(ask);
            // Sous le même verrou : deux questions parallèles entrent dans `open` (et partent vers
            // l'interface) dans l'ordre où elles sont enregistrées ; une question en attente est
            // toujours affichée.
            self.record_and_send(&mut state, McpEvent::surface_aloud(surface, spoken));
        }
        let mut guard = CloseOnDrop { hub: self, id, turn, freeze, answer: None };
        let answer = rx.await.unwrap_or(Answer::Cancelled);
        guard.answer = Some(answer.clone());
        answer
    }

    /// True if an `ask_*` question still waits for the user's answer.
    pub fn has_pending_ask(&self) -> bool {
        !self.state().asks.is_empty()
    }

    /// Answers a typed reply; never confirms a guard or picks a permission.
    ///
    /// The reply goes to the question `id` the interface names, never another one:
    /// `NotAQuestion` if it no longer waits (answered, cancelled, unknown). The agent's own
    /// questions take it as text; a guarded confirmation or a permission stays open and says
    /// "say it or click": typed text cannot be told from a draft the agent prefilled.
    pub fn answer(&self, id: &str, text: &str) -> AnswerOutcome {
        let l = self.localizer.current();
        let ask = {
            let mut state = self.state();
            let Some(i) = state.asks.iter().position(|a| a.id == id) else { return AnswerOutcome::NotAQuestion };
            if !matches!(state.asks[i].kind, AskKind::Agent) {
                self.show_line(&mut state, i, &l.tr("agent-surface-say-or-click"), &l);
                return AnswerOutcome::KeptOpen;
            }
            state.take_ask(i)
        };
        // Its waiting future may be gone (cancelled call): the reply is then not an answer.
        if ask.tx.send(Answer::text(text)).is_ok() { AnswerOutcome::Answered } else { AnswerOutcome::NotAQuestion }
    }

    /// Shows question `i` again with `line` under its buttons, under the state lock.
    pub(super) fn show_line(&self, state: &mut HubState, i: usize, line: &str, l: &oku_slides_i18n::Localizer) {
        if let Some(surface) = state.asks.get(i).and_then(|a| a.with_line(line, l)) {
            self.record_and_send(state, McpEvent::surface(surface));
        }
    }

    /// Libère toutes les questions en attente (Échap, nouveau tour, nouvelle conversation).
    pub fn cancel_asks(&self) {
        self.cancel_asks_where(|_| true);
    }

    /// Libère les questions en attente que `which` désigne.
    pub(crate) fn cancel_asks_where(&self, which: impl Fn(&str) -> bool) {
        // Le verrou d'un temporaire dans la tête d'un `for` vit jusqu'à la
        // fin de la boucle : on lie d'abord les questions pour ne pas le tenir
        // pendant l'envoi (qui peut réveiller le futur en attente).
        let asks = self.state().take_asks_where(which);
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

    /// Confirms a tool that cannot be undone; `true` only on "confirm".
    ///
    /// A danger `ask` shows its verb on the button, and a spoken yes must say it too;
    /// a plain one names its action aloud, and any yes approves it.
    pub async fn confirm(&self, message: String, ask: ConfirmAsk) -> bool {
        let id = self.next_id("confirm");
        let l = self.localizer.current();
        let danger = ask.verb();
        let verb = danger.map(|v| l.tr(v.label_key()));
        let surface = forms::confirm(&id, &message, verb.clone(), None, &l);
        // Read: the template and the verb or action only, never the message (it may name a tab or a path).
        let spoken = match ask {
            ConfirmAsk::Plain(action) => SpokenText::confirm_action(&l.tr(action.key()), &l),
            ConfirmAsk::Danger(_) => SpokenText::confirm(verb.as_deref(), &l),
        };
        matches!(
            self.ask_with(None, surface, None, AskKind::Confirm { message, verb: danger }, Some(spoken)).await,
            Answer::Action { name, context } if name == "confirm" && context.get("confirmed") == Some(&Value::Bool(true))
        )
    }

    /// An ACP permission asked by the agent: the chosen option's index, in the agent's order.
    ///
    /// A click on an option answers, or a yes or no said at the microphone (allow-once,
    /// reject-once: by kind, never by label). Escape or a new conversation give `None`, which
    /// the connection sends as a refusal; a typed reply leaves it open.
    pub async fn permission(&self, title: &str, options: &[PermissionChoice]) -> Option<usize> {
        let id = self.next_id("permission");
        let note = self.state().permission_note.clone();
        let l = self.localizer.current();
        let surface = forms::permission(&id, title, options, note.as_deref(), None, &l);
        let kind = AskKind::Permission { title: title.to_owned(), options: options.to_vec(), note };
        // Read: the template alone, never the agent's title nor its option labels.
        let spoken = SpokenText::permission(&l);
        match self.ask_with(None, surface, None, kind, Some(spoken)).await {
            Answer::Action { name, context } if name == "permission" => Self::index(&context, "index").ok(),
            _ => None,
        }
    }
}
