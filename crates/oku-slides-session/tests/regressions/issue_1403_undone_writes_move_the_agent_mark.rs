//! t-1403: the agent's widget mark followed the widget's presence, not its content.
//!
//! An undone agent write left the mark on the user's own content, and an undone
//! user edit gave the agent's content back without its mark.

use oku_slides_session::agent::AgentUi;
use oku_slides_session::{ESSENTIEL_PACK, Request, Session};
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::workspace_support::{on, open_pack, workspace};

const NOTE: &str = "user edited widget chapter after you";

/// Request `req` on the tab's session; its JSON answer.
fn handle(s: &mut Session, req: Request) -> String {
    s.handle(req).unwrap_or_else(|e| panic!("request: {}", e.one_line()))
}

/// The source of widget `chapter`.
fn chapter(s: &Session) -> String {
    s.widget_source(None, "chapter").unwrap_or_else(|e| panic!("{}", e.one_line()))
}

/// The agent writes `source` in `chapter`, in a turn of its own.
fn agent_write(s: &mut Session, source: String) {
    s.begin_turn(1, "chapter");
    s.set_widget_by_agent(None, "chapter", source).unwrap_or_else(|e| panic!("{}", e.one_line()));
    s.end_turn();
}

/// A user write of `chapter`, its source plus `tag`.
fn user_edit(s: &mut Session, source: &str, tag: &str) {
    handle(s, Request::SetWidget { pack: None, name: "chapter".into(), source: format!("{source}// {tag}\n") });
}

#[test]
fn issue_1403_undone_agent_write_drops_its_mark() {
    let data = data_dir();
    let ws = workspace(&data);
    let tab = open_pack(&ws, "t-1", ESSENTIEL_PACK);
    let notes = on(&ws, &tab, |s| {
        let user = chapter(s);
        agent_write(s, format!("{user}// agent\n"));
        handle(s, Request::Undo);
        user_edit(s, &user, "user");
        s.agent_context(&AgentUi::default())
    });
    assert_that!(notes.contains(NOTE)).is_false();
}

#[test]
fn issue_1403_undone_user_edit_gives_the_mark_back() {
    let data = data_dir();
    let ws = workspace(&data);
    let tab = open_pack(&ws, "t-1", ESSENTIEL_PACK);
    let notes = on(&ws, &tab, |s| {
        let agent = format!("{}// agent\n", chapter(s));
        agent_write(s, agent.clone());
        user_edit(s, &agent, "first");
        // Drains the first edit's note: the one asserted below comes from the second.
        s.agent_context(&AgentUi::default());
        handle(s, Request::Undo);
        user_edit(s, &agent, "second");
        s.agent_context(&AgentUi::default())
    });
    assert_that!(notes.contains(NOTE)).is_true();
}
