//! t-1383: after t-1206 (`UnknownTab`), other session errors still took a request's raw
//! strings into their message, which the servers log (`one_line`): a forged slide id or window
//! label put megabytes, bidi controls or a line break into a log line.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::agent::LABEL_MAX;
use oku_slides_session::{SessionErrorKind, Workspace, WorkspaceRequest};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{data_dir, session};
use crate::workspace_support::workspace;

fn forged() -> String {
    format!("s-\u{202E}\nforged{}", "9".repeat(10_000))
}

/// The message holds no bidi control nor line break and stays near one label's length.
fn assert_clean(message: &str) {
    assert_that!(message.chars().count()).is_less_than(LABEL_MAX + 60);
    assert_that!(message.contains('\u{202E}')).is_false();
    assert_that!(message.contains('\n')).is_false();
}

#[test]
fn issue_1383_no_slide_message_clips_the_requested_id() {
    let mut s = session();
    let err = s.svg(&forged()).err().unwrap_or_else(|| panic!("an unknown slide is refused"));
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoSlide);
    assert_clean(&err.one_line());
}

#[test]
fn issue_1383_no_slide_localized_message_clips_the_requested_id() {
    let mut s = session();
    let err = s.svg(&forged()).err().unwrap_or_else(|| panic!("an unknown slide is refused"));
    assert_clean(&err.localize(&Localizer::new(Some("fr"))));
}

#[test]
fn issue_1383_unknown_window_message_clips_the_requested_label() {
    let data = data_dir();
    let ws = workspace(&data);
    let req: WorkspaceRequest = serde_json::from_value(json!({ "cmd": "new_tab", "window": forged() }))
        .unwrap_or_else(|e| panic!("request: {e}"));
    let err = Workspace::apply(&ws, req).err().unwrap_or_else(|| panic!("an unknown window is refused"));
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
    assert_clean(&err.one_line());
}
