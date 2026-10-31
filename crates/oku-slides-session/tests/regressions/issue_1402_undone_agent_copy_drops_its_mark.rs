//! t-1402: an undone agent copy kept the agent's mark on its name.
//!
//! The user's own item of that name then told the agent "user edited widget X
//! after you" on its first edit, though the agent never wrote it.

use oku_slides_session::agent::AgentUi;
use oku_slides_session::{ESSENTIEL_PACK, Request, Session};
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::workspace_support::{on, open_pack, workspace};

/// Request `req` on the tab's session; its JSON answer.
fn handle(s: &mut Session, req: Request) -> String {
    s.handle(req).unwrap_or_else(|e| panic!("request: {}", e.one_line()))
}

#[test]
fn issue_1402_undone_agent_copy_drops_its_mark() {
    let data = data_dir();
    let ws = workspace(&data);
    let tab = open_pack(&ws, "t-1", ESSENTIEL_PACK);
    let notes = on(&ws, &tab, |s| {
        s.duplicate_widget_by_agent(None, "chapter", Some("copie")).unwrap_or_else(|e| panic!("{}", e.one_line()));
        handle(s, Request::Undo);
        handle(s, Request::DuplicateWidget { pack: None, from: "chapter".into(), to: Some("copie".into()) });
        let source = s.widget_source(None, "copie").unwrap_or_else(|e| panic!("{}", e.one_line()));
        handle(s, Request::SetWidget { pack: None, name: "copie".into(), source: format!("{source}// v2\n") });
        s.agent_context(&AgentUi::default())
    });
    assert_that!(notes.contains("user edited widget copie after you")).is_false();
}
