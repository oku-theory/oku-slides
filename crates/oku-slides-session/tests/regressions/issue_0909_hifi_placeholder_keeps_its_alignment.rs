//! t-909: the calibration corpus showed every centered placeholder of a hi-fi import drawn left.
//!
//! The generated `champ` set the layout's alignment, then placed its box with
//! `place(top + left, …)`, which imposes `left` on what it places: a centered title
//! (`poi-themes.pptx`, slide 1, the Office theme's `ctrTitle`) came out flush left.

use std::path::{Path, PathBuf};

use oku_slides_session::import::{ImportRequest, ImportTarget, PackLookup, import};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use speculoos::prelude::*;

use crate::common::data_dir;

/// A session on `data` holding the hi-fi import of the corpus deck `name`, without a witness.
fn imported(data: &Path, name: &str) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let pptx = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/corpus")).join(name);
    let request = ImportRequest {
        pptx,
        target: ImportTarget::Hifi { witness: None },
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let imported = import(&platform, &Fonts::default(), &request).unwrap_or_else(|e| panic!("import: {e}"));
    let mut s = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session: {}", e.one_line()));
    s.load_import(imported, name).unwrap_or_else(|e| panic!("load: {}", e.one_line()));
    s
}

#[test]
fn hifi_import_of_a_centered_title_placeholder_renders_it_centered() {
    let data = data_dir();
    let s = imported(data.as_ref(), "poi-themes.pptx");
    let view = s.slide("s1").unwrap_or_else(|e| panic!("slide: {}", e.one_line()));
    let title = view.fields.iter().find(|f| f.owner == "s1" && f.field == "titre");
    let align = title.and_then(|f| f.style).and_then(|st| st.align.clone());
    assert_that!(align.as_deref()).is_equal_to(Some("center"));
}
