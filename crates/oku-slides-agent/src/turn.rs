//! One turn: its undo group, context, `session/prompt`, and what arrives
//! while it runs (a cancel, a new instruction).

use agent_client_protocol::schema::v1::{
    CancelNotification, ContentBlock, PromptRequest, SessionId, TextContent, Usage,
};
use agent_client_protocol::{Agent, ConnectionTo};
use oku_slides_session::DocumentKind;
use tokio::sync::mpsc;

use crate::events::as_text;
use crate::instructions::{IMPORT, INSTRUCTIONS, WIDGETS, for_kind};
use crate::tab_agent::{Command, Ctx, Instruction};
use crate::{AgentEvent, AgentState};

/// What happened during the turn, applied by the tab's agent once it ends.
///
/// That is a new conversation (once, even if an instruction follows) and/or the
/// last instruction received (a later `Cancel` cancels it, but not the new
/// conversation).
#[derive(Debug, Default)]
pub(crate) struct Pending {
    /// The conversation starts over after the turn.
    pub new_conversation: bool,
    /// The last instruction received during the turn, played next.
    pub prompt: Option<Instruction>,
}

/// Plays turn `turn` of `instruction`.
///
/// Returns what happened during the turn (handled next) and the reply's token usage, `None` when
/// the agent sent none.
///
/// The prompt: the instructions (the session's first turn: the core, the tab kind's
/// paragraph, then the widget text outside a theme tab), the import's (a fine-tuning
/// turn), the context, the pasted images, then the instruction's text, always last.
/// `see_images`: the agent gets image blocks. `prompting` is called just before `session/prompt` is sent.
#[expect(
    clippy::too_many_arguments,
    reason = "the parameters come from the ACP connection; grouping them would add a type with no other use"
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
) -> agent_client_protocol::Result<(Pending, Option<Usage>)> {
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
    // Without a session the turn has no context and no images: there is nothing to read them from.
    let (context, pasted) = match blocking {
        Ok(Ok(x)) => x,
        Ok(Err(e)) => {
            tracing::warn!(error = %e.one_line(), "agent context: tab session not found, empty context");
            (String::new(), Vec::new())
        }
        Err(e) => {
            tracing::warn!(error = %e, "agent context: blocking task lost, empty context");
            (String::new(), Vec::new())
        }
    };
    ctx.emit(AgentEvent::Status { state: AgentState::Working });
    tracing::debug!(tab = %ctx.tab.tab(), kind = ctx.kind.as_str(), turn, "agent turn starts");
    let mut blocks = Vec::with_capacity(5);
    if first {
        blocks.push(ContentBlock::Text(TextContent::new(INSTRUCTIONS)));
        blocks.push(ContentBlock::Text(TextContent::new(for_kind(ctx.kind))));
        // A theme tab writes no widget.
        if ctx.kind != DocumentKind::Theme {
            blocks.push(ContentBlock::Text(TextContent::new(WIDGETS)));
        }
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
                // Any command during a turn stops it, once. An instruction or a new
                // conversation is applied afterwards.
                if !cancel_sent {
                    ctx.hub.cancel_asks();
                    // Connection over: the turn will fail on its own.
                    if let Err(e) = cx.send_notification(CancelNotification::new(session.clone())) {
                        tracing::debug!(error = %e, "session/cancel not sent");
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
    let response = response?;
    ctx.emit(AgentEvent::TurnEnded { turn, stop: as_text(&response.stop_reason) });
    ctx.emit(AgentEvent::idle());
    Ok((pending, response.usage))
}
