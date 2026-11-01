//! t-1385: a `.pptx` given by the OS while 32 picks already waited was dropped
//! with only a log line, so the user never learned the file was ignored. The
//! active tab now shows a localized notice counting the dropped files and naming
//! the first three, cleaned like any request string.

use oku_slides_session::{Request, Workspace};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::workspace_support::workspace;

#[test]
fn issue_1385_pptx_beyond_the_full_queue_raise_one_notice_naming_them() {
    let data = data_dir();
    let ws = workspace(&data);
    let files: Vec<_> = (0..37)
        .map(|i| {
            let path = data.join(format!("{i}.pptx"));
            std::fs::write(&path, b"not read").unwrap_or_else(|e| panic!("write {i}.pptx: {e}"));
            path
        })
        .collect();
    Workspace::open_files(&ws, &files);
    let req: Request = serde_json::from_value(json!({"cmd": "snapshot"})).unwrap_or_else(|e| panic!("request: {e}"));
    let reply = Workspace::request(&ws, "t-1", req).0.unwrap_or_else(|e| panic!("refused: {}", e.one_line()));
    let snap: serde_json::Value = serde_json::from_str(&reply).unwrap_or_else(|e| panic!("reply: {e}"));
    let notice = snap["notice"].as_str().unwrap_or_default().to_owned();
    // The workspace speaks French: 32 can wait, 5 were dropped, 3 are named.
    assert_that!(notice.as_str()).starts_with("Trop de fichiers PowerPoint");
    assert_that!(notice.as_str()).contains("Non ajoutés (5) : 32.pptx, 33.pptx, 34.pptx et 2 autres.");
}
