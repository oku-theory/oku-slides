//! Questions en attente (`ask_*`, confirmations, permissions) : ouverture, réponse, relâche.

use oku_slides_session::agent::{PermissionChoice, Surface, forms};
use serde_json::{Map, Value};
use tokio::sync::oneshot;

use super::{Answer, Freeze, HubState, McpEvent, SurfaceHub};

#[derive(Debug)]
pub(super) struct PendingAsk {
    pub(super) id: String,
    pub(super) tx: oneshot::Sender<Answer>,
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
            // Sous le même verrou : deux questions parallèles entrent dans `open` (et partent vers
            // l'interface) dans l'ordre où elles sont enregistrées ; une question en attente est
            // toujours affichée.
            self.record_and_send(&mut state, McpEvent::Surface { surface });
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

    /// Une consigne (dictée ou tapée) en réponse à la question `id`, celle que l'interface
    /// désigne ; `false` si elle n'attend plus (répondue, annulée, inconnue) : aucune autre
    /// n'est répondue à sa place.
    pub fn answer(&self, id: &str, text: &str) -> bool {
        let Some(ask) = self.state().take_ask_id(id) else { return false };
        let mut context = Map::new();
        context.insert("text".into(), Value::String(text.to_owned()));
        ask.tx.send(Answer::Action { name: "answer".into(), context }).is_ok()
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

    /// Confirmation d'un outil non annulable ; `true` seulement sur « confirmer ».
    pub async fn confirm(&self, message: String, danger: bool) -> bool {
        let id = self.next_id("confirm");
        let surface =
            forms::confirm(&id, &oku_slides_session::agent::AskConfirm { message, danger }, &self.localizer.current());
        matches!(
            self.ask(surface).await,
            Answer::Action { name, context } if name == "confirm" && context.get("confirmed") == Some(&Value::Bool(true))
        )
    }

    /// Permission demandée par l'agent (ACP) : l'index de l'option choisie, dans
    /// l'ordre de l'agent. Seul un clic sur une option répond : Échap, une
    /// nouvelle conversation ou une consigne en réponse (`answer`, action nommée
    /// `answer`) donnent `None`, que la connexion transmet comme un refus.
    pub async fn permission(&self, title: &str, options: &[PermissionChoice]) -> Option<usize> {
        let id = self.next_id("permission");
        match self.ask(forms::permission(&id, title, options, &self.localizer.current())).await {
            Answer::Action { name, context } if name == "permission" => Self::index(&context, "index").ok(),
            _ => None,
        }
    }
}
