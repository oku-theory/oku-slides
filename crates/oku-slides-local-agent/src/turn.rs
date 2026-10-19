//! Tour de l'agent local : routage par familles, boucle d'outils bornée, rapport final.
//!
//! Un tour finit toujours par l'outil `report` : celui du modèle, ou un rapport d'échec
//! localisé écrit par l'agent. Seule l'annulation finit un tour sans rapport.

use async_trait::async_trait;
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::compact::{ToolSpec, exposed};
use crate::engine::{Completion, Engine, EngineError, Message, Reply, Step};
use crate::families::{Family, parse_route};
use crate::prompt::{INSTRUCTIONS, Prompt};
use crate::tool_error::ToolError;

/// Nombre maximal d'étapes `Act` d'un tour.
pub(crate) const MAX_CALLS: usize = 8;

/// Outil qui termine chaque tour.
const REPORT: &str = "report";

/// Accès aux outils MCP de l'onglet.
#[async_trait]
pub(crate) trait ToolPort: Send + Sync {
    /// Les outils du serveur, avec description et schéma complets.
    async fn list(&self) -> Result<Vec<ToolSpec>, ToolError>;
    /// Appelle un outil ; un refus du serveur est réparable, une panne de transport ne l'est pas.
    async fn call(&self, name: &str, args: &Value) -> Result<String, ToolError>;
}

/// Progression du tour montrée à l'utilisateur.
pub(crate) trait Updates: Send + Sync {
    /// Un appel d'outil commence.
    fn tool_started(&self, id: &str, name: &str, args: &Value);
    /// Un appel d'outil finit, réussi ou non.
    fn tool_finished(&self, id: &str, ok: bool);
    /// Le modèle a écrit un texte intermédiaire.
    fn note(&self, text: &str);
}

/// Comment un tour s'est terminé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TurnEnd {
    /// Le modèle a appelé `report` avec succès.
    Reported,
    /// Le tour a été annulé, sans rapport.
    Cancelled,
    /// Le tour a échoué ; un rapport d'échec a été tenté.
    Failed(FailKind),
}

/// Raison d'un échec de tour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailKind {
    /// Aucune famille reconnue dans la réponse de routage.
    NoRoute,
    /// `MAX_CALLS` étapes sans rapport.
    ToolLimit,
    /// Deux appels invalides de suite.
    InvalidCall,
    /// Le moteur ou le serveur d'outils a échoué.
    Engine,
}

impl FailKind {
    /// Clé Fluent du titre du rapport d'échec.
    pub(crate) fn key(self) -> &'static str {
        match self {
            FailKind::NoRoute => "local-agent-no-route",
            FailKind::ToolLimit => "local-agent-tool-limit",
            FailKind::InvalidCall => "local-agent-invalid-call",
            FailKind::Engine => "local-agent-engine-failed",
        }
    }
}

/// Pourquoi une étape arrête le tour avant le rapport du modèle.
enum Stop {
    Cancelled,
    Fail(FailKind),
}

impl From<EngineError> for Stop {
    fn from(e: EngineError) -> Self {
        if e.is_cancelled() {
            return Stop::Cancelled;
        }
        tracing::warn!(error = %e, "échec du moteur de l'agent local");
        Stop::Fail(FailKind::Engine)
    }
}

/// Le contexte d'un tour et son compteur d'appels d'outils.
struct Turn<'a> {
    engine: &'a dyn Engine,
    tools: &'a dyn ToolPort,
    updates: &'a dyn Updates,
    prompt: &'a Prompt,
    cancel: &'a CancellationToken,
    calls: usize,
}

/// Joue un tour : routage, boucle d'outils bornée, puis rapport (sauf annulation).
pub(crate) async fn run_turn(
    engine: &dyn Engine,
    tools: &dyn ToolPort,
    updates: &dyn Updates,
    prompt: &Prompt,
    summaries: &[String],
    cancel: &CancellationToken,
) -> TurnEnd {
    let mut turn = Turn { engine, tools, updates, prompt, cancel, calls: 0 };
    let stop = match turn.play(summaries).await {
        Ok(()) => return TurnEnd::Reported,
        Err(stop) => stop,
    };
    match stop {
        Stop::Cancelled => TurnEnd::Cancelled,
        // Une annulation qui arrive pendant l'étape fautive l'emporte : pas de rapport.
        Stop::Fail(_) if cancel.is_cancelled() => TurnEnd::Cancelled,
        Stop::Fail(kind) => {
            turn.fail(kind).await;
            TurnEnd::Failed(kind)
        }
    }
}

impl Turn<'_> {
    async fn play(&mut self, summaries: &[String]) -> Result<(), Stop> {
        self.check_cancel()?;
        let all = self.tools.list().await.map_err(|e| {
            tracing::warn!(error = %snafu::Report::from_error(&e), "liste des outils de l'agent local impossible");
            Stop::Fail(FailKind::Engine)
        })?;
        let mut messages = Vec::new();
        if !summaries.is_empty() {
            messages.push(Message::User(summaries.join("\n")));
        }
        messages.push(Message::User(match &self.prompt.context {
            Some(context) => format!("{context}\n\n{}", self.prompt.request),
            None => self.prompt.request.clone(),
        }));
        let route = self.route(&messages).await?;
        self.act(exposed(&all, &route), messages).await
    }

    fn check_cancel(&self) -> Result<(), Stop> {
        if self.cancel.is_cancelled() { Err(Stop::Cancelled) } else { Ok(()) }
    }

    /// Demande une ou deux familles au moteur ; un essai de plus si la réponse n'en nomme aucune.
    async fn route(&self, messages: &[Message]) -> Result<Vec<Family>, Stop> {
        let families: Vec<String> = Family::ROUTABLE.iter().map(|f| format!("{}: {}", f.key(), f.summary())).collect();
        let completion = Completion {
            step: Step::Route,
            system: format!("{INSTRUCTIONS}\n{}", families.join("\n")),
            messages: messages.to_vec(),
            tools: vec![],
        };
        for _ in 0..2 {
            self.check_cancel()?;
            if let Reply::Text(text) = self.engine.complete(&completion, self.cancel).await? {
                let route = parse_route(&text);
                if !route.is_empty() {
                    return Ok(route);
                }
            }
        }
        Err(Stop::Fail(FailKind::NoRoute))
    }

    /// Boucle d'outils : au plus `MAX_CALLS` étapes, finie par un `report` réussi.
    async fn act(&mut self, tools: Vec<ToolSpec>, mut messages: Vec<Message>) -> Result<(), Stop> {
        let mut repairs = 0;
        for _ in 0..MAX_CALLS {
            self.check_cancel()?;
            let completion = Completion {
                step: Step::Act,
                system: INSTRUCTIONS.to_owned(),
                messages: messages.clone(),
                tools: tools.clone(),
            };
            let (name, result) = match self.engine.complete(&completion, self.cancel).await? {
                Reply::Text(text) => {
                    self.updates.note(&text);
                    messages.push(Message::Assistant(text));
                    continue;
                }
                Reply::Invalid(reason) => {
                    ("invalid".to_owned(), Err(ToolError::rejected(format!("invalid tool call: {reason}"))))
                }
                Reply::Call { name, args } => {
                    messages.push(Message::Call { name: name.clone(), args: args.clone() });
                    let result = if tools.iter().any(|t| t.name == name) {
                        self.call(&name, &args).await
                    } else {
                        Err(ToolError::rejected(format!("unknown tool {name}")))
                    };
                    (name, result)
                }
            };
            match result {
                Ok(_) if name == REPORT => return Ok(()),
                Ok(output) => {
                    repairs = 0;
                    messages.push(Message::ToolResult { name, output });
                }
                Err(error) if !error.is_rejected() => {
                    tracing::warn!(error = %snafu::Report::from_error(&error), "serveur d'outils de l'agent local injoignable");
                    return Err(Stop::Fail(FailKind::Engine));
                }
                Err(error) => {
                    repairs += 1;
                    // Une annulation qui arrive pendant l'appel fautif l'emporte sur l'échec.
                    self.check_cancel()?;
                    if repairs == 2 {
                        return Err(Stop::Fail(FailKind::InvalidCall));
                    }
                    messages.push(Message::ToolResult { name, output: format!("error: {error}") });
                }
            }
            self.check_cancel()?;
        }
        Err(Stop::Fail(FailKind::ToolLimit))
    }

    /// Un appel d'outil, encadré par sa progression `call-{n}`.
    async fn call(&mut self, name: &str, args: &Value) -> Result<String, ToolError> {
        self.calls += 1;
        let id = format!("call-{}", self.calls);
        self.updates.tool_started(&id, name, args);
        let result = self.tools.call(name, args).await;
        self.updates.tool_finished(&id, result.is_ok());
        result
    }

    /// Envoie le rapport d'échec localisé ; un échec de cet envoi n'est que journalisé.
    async fn fail(&mut self, kind: FailKind) {
        let localizer = Localizer::new(self.prompt.locale.as_deref());
        let title = match kind {
            FailKind::ToolLimit => localizer.tr_with(kind.key(), &[("n", MAX_CALLS.into())]),
            _ => localizer.tr(kind.key()),
        };
        // `report` exige 1 à 4 faits : l'unique fait est la marche à suivre.
        let args = json!({ "title": title, "facts": [localizer.tr("local-agent-retry")] });
        if let Err(error) = self.call(REPORT, &args).await {
            tracing::warn!(error = %snafu::Report::from_error(&error), "rapport d'échec de l'agent local refusé");
        }
    }
}

#[cfg(test)]
#[path = "turn_tests.rs"]
mod turn_tests;
