//! t-329 : un widget qui ne compile pas n'empêche plus les autres widgets
//! du même pack de se rendre dans le deck.
//!
//! Avant t-329, un pack était un seul `widgets.typ` : une erreur de Typst
//! écartait le pack entier, et le deck retombait sur les widgets de base.

use std::collections::BTreeMap;
use std::path::Path;

use oku_slides_core::{Namespace, Op, PackPin, WidgetFile};
use oku_slides_session::{PackLibrary, PackManifest, Request, Storage};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, fixture_path, session_in};

/// Le fichier `quote` : un widget d'élément valide.
const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Le fichier `casse` : ne compile pas.
const BROKEN: &str = "#let widget = (";

/// Un pack « Mixte » (`quote` valide, `casse` cassé) ajouté à la bibliothèque de `data`.
fn mixed_pack(data: &Path) -> PackPin {
    let id = Uuid::new_v4();
    let manifest = PackManifest {
        id,
        name: "Mixte".into(),
        namespace: oku_slides_core::Namespace::parse("mixte").unwrap_or_else(|| panic!("espace de noms")),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-02T10:00:00Z".into(),
        scope: oku_slides_core::PackScope::Library,
        templates: Vec::new(),
        examples: Default::default(),
    };
    let name = |n: &str| WidgetFile::parse(n).unwrap_or_else(|| panic!("nom invalide : {n}"));
    let widgets = BTreeMap::from([(name("quote"), QUOTE.to_owned()), (name("casse"), BROKEN.to_owned())]);
    let files = manifest.to_files(widgets).unwrap_or_else(|e| panic!("{e}"));
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    PackLibrary::new(Some(data.join("widgets")), storage).add_version(id, &files).unwrap_or_else(|e| panic!("{e}"));
    PackPin { id, version: files.version(), requires: Vec::new() }
}

#[test]
fn deck_with_one_broken_widget_still_renders_the_other_widgets_of_the_pack() {
    let data = data_dir();
    let r = mixed_pack(&data);
    let mut s = session_in(&data);
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let quote: Op = serde_json::from_value(serde_json::json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": {
            "id": "q",
            "kind": "mixte/quote",
            "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "Héraclite" }
        }
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    let namespace = Namespace::parse("mixte").unwrap_or_else(|| panic!("espace de noms"));
    s.apply("Pack", vec![Op::AddPack { namespace, pin: r }, quote], None)
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    let widgets: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(widgets["widgets"].get("mixte/quote").is_some()).is_true();
    assert_that!(s.deck().slide("s-constat").is_some_and(|slide| slide.locate("q").is_some())).is_true();
    let snapshot: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot["notice"].as_str().unwrap_or_default()).contains("casse");
}
