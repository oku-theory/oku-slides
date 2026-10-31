//! t-981 (and t-959): a received pack's name went as is into localized sentences. The bundles add
//! no isolation marks, so a name carrying an override (U+202E) or zero-width characters could
//! make a sentence read otherwise. Every interpolated value now loses these characters.

use std::collections::BTreeMap;

use oku_slides_core::{PackKind, PackScope, WidgetFile};
use oku_slides_session::{PackLibrary, PackManifest, Storage};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, no_user_themes, ns};
use crate::workspace_support::{on, workspace};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("quote", params: (body: p.plain()), (args, ctx) => [#args.body])
"#;

/// Adds the widget pack `acme`, named `name`, to the library under `data`.
fn add_pack(data: &std::path::Path, name: &str) {
    let storage = Storage::local().unwrap_or_else(|e| panic!("storage: {e}"));
    let library = PackLibrary::new(Some(data.join("widgets")), None, no_user_themes(), storage);
    let manifest = PackManifest {
        id: Uuid::new_v4(),
        name: name.to_owned(),
        namespace: ns("acme"),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-31T10:00:00Z".to_owned(),
        scope: PackScope::Library,
        profiles: vec![],
        examples: BTreeMap::default(),
        styles: BTreeMap::default(),
        tokens: vec![],
    };
    let quote = WidgetFile::parse("quote").unwrap_or_else(|| panic!("widget name"));
    let files = manifest
        .to_files(PackKind::Widgets, BTreeMap::from([(quote, QUOTE.to_owned())]))
        .unwrap_or_else(|e| panic!("pack: {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("pack: {}", e.one_line()));
}

#[test]
fn issue_0981_pack_name_in_a_sentence_loses_its_bidi_and_invisible_characters() {
    let data = data_dir();
    add_pack(data.as_ref(), "Ré\u{202E}cent\u{2066}e\u{200B}");
    let ws = workspace(&data);
    let label = on(&ws, "t-1", |s| {
        s.add_library_pack(PackKind::Widgets, &ns("acme")).unwrap_or_else(|e| panic!("add: {}", e.one_line()));
        s.snapshot().undo_label
    });
    let label = label.unwrap_or_else(|| panic!("an undo label"));
    assert_that!(label.chars().any(oku_slides_i18n::is_hidden_control)).is_false();
    assert_that!(label).contains("Récente");
}
