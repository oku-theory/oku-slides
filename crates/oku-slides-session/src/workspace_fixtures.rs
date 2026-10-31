//! Shared helpers of the workspace tests: a data dir, the view, a saved deck.

use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::json;

use super::{Fonts, Session, WindowView, Workspace, WorkspaceView};
use crate::Request;
use crate::test_support::workspace_platform;

/// A fresh data dir, deleted when the test ends.
pub(super) struct Data(pub(super) PathBuf);

impl Data {
    pub(super) fn new() -> Self {
        Self(crate::test_support::fresh_dir("oku-slides-workspace-").keep())
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// The workspace's view.
pub(super) fn view(ws: &Mutex<Workspace>) -> WorkspaceView {
    Workspace::lock(ws).view()
}

/// The tab ids of window `w`, in order.
pub(super) fn ids(w: &WindowView) -> Vec<&str> {
    w.tabs.iter().map(|t| t.id.as_str()).collect()
}

/// An edit: the `pro` theme.
pub(super) fn set_theme() -> Request {
    serde_json::from_value(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": "pro"}]}))
        .unwrap()
}

/// Un vrai `.oku-slides.pdf` : un deck JSON n'a pas de fichier associé.
pub(super) fn saved_pdf(data: &Data, name: &str) -> PathBuf {
    let fixture = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    let mut s = Session::new(Fonts::default(), &workspace_platform(&data.0)).unwrap();
    s.open(&fixture).unwrap();
    let path = data.0.join(format!("{name}.oku-slides.pdf"));
    s.save(Some(&path)).unwrap();
    path
}
