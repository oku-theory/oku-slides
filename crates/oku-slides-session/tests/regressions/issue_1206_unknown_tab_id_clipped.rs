//! t-1206: `UnknownTab` took the request's raw tab id into its message, which the servers log
//! (`one_line`): a forged id put megabytes, bidi controls or a line break into a log line.

use oku_slides_session::agent::LABEL_MAX;
use oku_slides_session::{SessionErrorKind, Workspace, WorkspaceRequest};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::workspace_support::workspace;

#[test]
fn issue_1206_unknown_tab_message_clips_the_requested_id() {
    let data = data_dir();
    let ws = workspace(&data);
    let forged = format!("t-\u{202E}\nforged{}", "9".repeat(10_000));
    let req =
        json!({ "cmd": "carry_assets", "from": forged, "to": forged, "element": { "id": "e", "kind": "base/text" } });
    let req: WorkspaceRequest = serde_json::from_value(req).unwrap_or_else(|e| panic!("request: {e}"));
    let err = Workspace::apply(&ws, req).err().unwrap_or_else(|| panic!("an unknown tab is refused"));
    let message = err.one_line();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTab);
    assert_that!(message.chars().count()).is_less_than(LABEL_MAX + 40);
    assert_that!(message.contains('\u{202E}')).is_false();
    assert_that!(message.contains('\n')).is_false();
}
