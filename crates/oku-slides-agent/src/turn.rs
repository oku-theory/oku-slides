//! Un tour : groupe d'annulation, contexte, `session/prompt`, et ce qui
//! arrive pendant qu'il tourne (annulation, nouvelle consigne).

use agent_client_protocol::schema::v1::{CancelNotification, ContentBlock, PromptRequest, SessionId, TextContent};
use agent_client_protocol::{Agent, ConnectionTo};
use tokio::sync::mpsc;

use crate::events::as_text;
use crate::instructions::{IMPORT, INSTRUCTIONS, WIDGETS};
use crate::tab_agent::{Command, Ctx, Instruction};
use crate::{AgentEvent, AgentState};

/// Ce qui est arrivé pendant le tour, à appliquer par l'agent de l'onglet
/// une fois le tour fini : une nouvelle conversation (une fois, même si une
/// consigne suit) et/ou la dernière consigne reçue (une `Cancel` après coup
/// l'annule, mais pas la nouvelle conversation).
#[derive(Debug, Default)]
pub(crate) struct Pending {
    /// La conversation repart de zéro après le tour.
    pub new_conversation: bool,
    /// La dernière consigne reçue pendant le tour, à jouer ensuite.
    pub prompt: Option<Instruction>,
}

/// Joue le tour `turn` de `instruction`.
///
/// Renvoie ce qui est arrivé pendant le tour (à traiter ensuite).
///
/// Le prompt : les consignes (premier tour de la session), celles de l'import (tour de
/// fine-tuning), le contexte, les images collées, puis le texte de la consigne, toujours en dernier.
/// `see_images` : l'agent reçoit des blocs image. `prompting` est appelé juste avant l'envoi de `session/prompt`.
#[expect(
    clippy::too_many_arguments,
    reason = "les paramètres viennent de la connexion ACP ; les regrouper ajouterait un type sans autre usage"
)]
pub(crate) async fn run(
    ctx: &Ctx,
    cx: &ConnectionTo<Agent>,
    rx: &mut mpsc::UnboundedReceiver<Command>,
    turn: u32,
    session: SessionId,
    instruction: Instruction,
    first: bool,
    see_images: bool,
    prompting: impl FnOnce(),
) -> agent_client_protocol::Result<Pending> {
    let Instruction { text, ui, tune, images } = instruction;
    let tab = ctx.tab.clone();
    let text_for_begin = text.clone();
    let blocking = tokio::task::spawn_blocking(move || {
        tab.with(|s| {
            s.begin_turn(turn, &text_for_begin);
            let pasted: Vec<_> = images
                .into_iter()
                .map(|r| {
                    let bytes = s.pasted_image(&r.id).map(|(_, b)| b);
                    (r, bytes)
                })
                .collect();
            (s.agent_context(&ui), pasted)
        })
    })
    .await;
    // Sans session, le tour n'a ni contexte ni images : il n'y a rien où les lire.
    let (context, pasted) = match blocking {
        Ok(Ok(x)) => x,
        Ok(Err(e)) => {
            tracing::warn!(error = %e.one_line(), "contexte de l'agent : session de l'onglet introuvable, contexte vide");
            (String::new(), Vec::new())
        }
        Err(e) => {
            tracing::warn!(error = %e, "contexte de l'agent : tâche bloquante perdue, contexte vide");
            (String::new(), Vec::new())
        }
    };
    ctx.emit(AgentEvent::Status { state: AgentState::Working });
    let mut blocks = Vec::with_capacity(5);
    if first {
        blocks.push(ContentBlock::Text(TextContent::new(INSTRUCTIONS)));
        blocks.push(ContentBlock::Text(TextContent::new(WIDGETS)));
    }
    if tune {
        blocks.push(ContentBlock::Text(TextContent::new(IMPORT)));
    }
    blocks.push(ContentBlock::Text(TextContent::new(context)));
    blocks.extend(crate::prompt_images::image_blocks(&pasted, see_images, ctx.prompt_limits));
    blocks.push(ContentBlock::Text(TextContent::new(text)));
    prompting();
    let request = cx.send_request(PromptRequest::new(session.clone(), blocks)).block_task();
    tokio::pin!(request);
    let mut pending = Pending::default();
    let mut cancel_sent = false;
    let mut open = true;
    let response = loop {
        tokio::select! {
            r = &mut request => break r,
            cmd = rx.recv(), if open => {
                // Toute commande pendant un tour l'arrête, une seule fois. Une consigne ou
                // une nouvelle conversation est appliquée ensuite.
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
                    Some(Command::Prompt(next)) => pending.prompt = Some(next),
                }
            }
        }
    };
    ctx.end_turn().await;
    let stop = as_text(&response?.stop_reason);
    ctx.emit(AgentEvent::TurnEnded { turn, stop });
    ctx.emit(AgentEvent::idle());
    Ok(pending)
}
