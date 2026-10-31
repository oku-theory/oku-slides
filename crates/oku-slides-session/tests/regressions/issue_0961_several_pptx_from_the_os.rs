//! t-961: several `.pptx` opened at once from the OS offered only the last one.
//! The host kept a single import pick and each file replaced the previous one;
//! they now queue in order, and acknowledging one leaves the others.

use oku_slides_session::Workspace;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::workspace_support::{send, workspace};

#[test]
fn issue_0961_three_pptx_at_once_are_offered_one_after_another() {
    let data = data_dir();
    let ws = workspace(&data);
    let files: Vec<_> = ["a.pptx", "b.pptx", "c.pptx"]
        .iter()
        .map(|name| {
            let path = data.join(name);
            std::fs::write(&path, b"not read").unwrap_or_else(|e| panic!("write {name}: {e}"));
            path
        })
        .collect();
    Workspace::open_files(&ws, &files);
    let names = |view: &serde_json::Value| -> Vec<String> {
        let picks = view["import_picks"].as_array().cloned().unwrap_or_default();
        picks.iter().filter_map(|p| p["path"].as_str()?.rsplit('/').next().map(str::to_owned)).collect()
    };
    let view = send(&ws, json!({ "cmd": "view" }));
    assert_that!(names(&view)).is_equal_to(vec!["a.pptx".to_owned(), "b.pptx".into(), "c.pptx".into()]);
    let first = view["import_picks"][0]["id"].clone();
    let view = send(&ws, json!({ "cmd": "import_picked", "id": first }));
    assert_that!(names(&view)).is_equal_to(vec!["b.pptx".to_owned(), "c.pptx".into()]);
}
