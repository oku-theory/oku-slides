//! Un tour : groupe d'annulation, contexte, `session/prompt`, et ce qui
//! arrive pendant qu'il tourne (annulation, nouvelle consigne).

use agent_client_protocol::schema::v1::{CancelNotification, ContentBlock, PromptRequest, SessionId, TextContent};
use agent_client_protocol::{Agent, ConnectionTo};
use oku_slides_session::agent::AgentUi;
use tokio::sync::mpsc;

use crate::events::as_text;
use crate::tab_agent::{Command, Ctx};
use crate::{AgentEvent, AgentState};

/// Ce qui est arrivé pendant le tour, à appliquer par l'agent de l'onglet
/// une fois le tour fini : une nouvelle conversation (déclenchée une fois,
/// même si une consigne suit) et/ou la dernière consigne reçue (une `Cancel`
/// après coup l'annule, mais pas la nouvelle conversation).
#[derive(Debug, Default)]
pub(crate) struct Pending {
    pub new_conversation: bool,
    pub prompt: Option<(String, AgentUi)>,
}

/// Joue un tour ; renvoie ce qui est arrivé pendant le tour (à traiter ensuite).
#[expect(
    clippy::too_many_arguments,
    reason = "les paramètres viennent de la connexion ACP ; les regrouper ajouterait un type sans autre usage"
)]
pub(crate) async fn run(
    ctx: &Ctx,
    cx: &ConnectionTo<Agent>,
    rx: &mut mpsc::UnboundedReceiver<Command>,
    session: SessionId,
    text: String,
    ui: AgentUi,
    first: bool,
) -> agent_client_protocol::Result<Pending> {
    let tab = ctx.tab.clone();
    let text_for_begin = text.clone();
    let blocking = tokio::task::spawn_blocking(move || {
        tab.with(|s| {
            s.begin_turn(&text_for_begin);
            s.agent_context(&ui)
        })
    })
    .await;
    let context = match blocking {
        Ok(Ok(context)) => context,
        Ok(Err(e)) => {
            tracing::warn!(error = %e.one_line(), "contexte de l'agent : session de l'onglet introuvable, contexte vide");
            String::new()
        }
        Err(e) => {
            tracing::warn!(error = %e, "contexte de l'agent : tâche bloquante perdue, contexte vide");
            String::new()
        }
    };
    ctx.emit(AgentEvent::Status { state: AgentState::Working });
    let mut blocks = Vec::with_capacity(4);
    if first {
        blocks.push(ContentBlock::Text(TextContent::new(ctx.localizer.tr("agent-instructions"))));
        blocks.push(ContentBlock::Text(TextContent::new(ctx.localizer.tr("agent-instructions-widgets"))));
    }
    blocks.push(ContentBlock::Text(TextContent::new(context)));
    blocks.push(ContentBlock::Text(TextContent::new(text)));
    let request = cx.send_request(PromptRequest::new(session.clone(), blocks)).block_task();
    tokio::pin!(request);
    let mut pending = Pending::default();
    let mut cancel_sent = false;
    let mut open = true;
    let response = loop {
        tokio::select! {
            r = &mut request => break r,
            cmd = rx.recv(), if open => {
                // Toute commande pendant un tour l'arrête, une seule fois ; une
                // consigne ou une nouvelle conversation est appliquée ensuite.
                if !cancel_sent {
                    ctx.hub.cancel_asks();
                    // Connexion finie : le tour échouera de lui-même.
                    if let Err(e) = cx.send_notification(CancelNotification::new(session.clone())) {
                        tracing::debug!(error = %e, "session/cancel non envoyé");
                    }
                    cancel_sent = true;
                }
                match cmd {
                    None => open = false,
                    Some(Command::Cancel) => pending.prompt = None,
                    Some(Command::NewConversation) => pending.new_conversation = true,
                    Some(Command::Prompt { text, ui }) => pending.prompt = Some((text, ui)),
                }
            }
        }
    };
    ctx.end_turn().await;
    let stop = as_text(&response?.stop_reason);
    ctx.emit(AgentEvent::TurnEnded { stop });
    ctx.emit(AgentEvent::Status { state: AgentState::Idle });
    Ok(pending)
}
