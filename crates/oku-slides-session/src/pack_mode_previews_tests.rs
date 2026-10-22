use speculoos::prelude::*;

use crate::test_support::{editing_layout_pack, fresh_dir, session};
use crate::{LayoutsView, Request};

/// La réponse de `Request::Layouts`, relue.
fn layouts(s: &mut crate::Session) -> serde_json::Value {
    serde_json::from_str(&s.handle(Request::Layouts).unwrap()).unwrap()
}

#[test]
fn request_layouts_lists_essentiel_layouts_with_previews() {
    let mut s = session();
    let view: serde_json::Value = layouts(&mut s);
    let packs = view["packs"].as_array().cloned().unwrap_or_default();
    let shown: Vec<&serde_json::Value> = packs[0]["layouts"].as_array().into_iter().flatten().collect();
    let previews = shown.iter().all(|l| l["svg"].as_str().is_some_and(|svg| svg.starts_with("<svg")));
    assert_that!((packs.len(), shown.len(), previews)).is_equal_to((1, 6, true));
}

#[test]
fn layouts_view_names_each_layout_in_the_user_language() {
    let mut s = session();
    let view: LayoutsView = s.layouts_view().unwrap();
    let content = view.packs[0].layouts.iter().find(|l| l.kind == oku_slides_core::kinds::essentiel::CONTENT);
    assert_that!(content.map(|l| l.label.as_str())).is_equal_to(Some("Contenu"));
}

#[test]
fn layouts_view_slide_of_a_layout_is_its_example() {
    let mut s = session();
    let view = s.layouts_view().unwrap();
    let content = view.packs[0].layouts.iter().find(|l| l.kind == oku_slides_core::kinds::essentiel::CONTENT);
    let slide = content.map(|l| (l.slide.id.clone(), l.slide.params.text("title").map(str::to_owned)));
    assert_that!(slide).is_equal_to(Some(("layout-content".to_owned(), Some("Titre de la slide".to_owned()))));
}

#[test]
fn layouts_view_in_pack_mode_lists_the_edited_layout_pack() {
    let data = fresh_dir("oku-slides-pack-previews-");
    let (mut s, _) = editing_layout_pack(data.path(), "acme", &["one"]);
    let view = s.layouts_view().unwrap();
    let names: Vec<&str> = view.packs.iter().map(|p| p.namespace.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["acme"]);
}

/// Le deck de `s` épingle, à côté d'`essentiel`, le pack de layouts `acme` (layout `one`) ; son épinglage.
fn pin_acme(s: &mut crate::Session) -> oku_slides_core::PackPin {
    let m = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), "acme", &["base"]);
    let kind = oku_slides_core::PackKind::Layouts;
    let source = crate::test_support::slide_widget("one");
    let pin = crate::test_support::add_version_of(&s.library, kind, &m, &[("one", source.as_str())]);
    let op = oku_slides_core::Op::AddPack { kind, namespace: m.namespace.clone(), pin: pin.clone() };
    s.apply("Pack", vec![op], None).unwrap();
    pin
}

/// Les espaces de noms que liste `Request::Layouts`.
fn listed(s: &mut crate::Session) -> Vec<String> {
    let view = layouts(s);
    let packs = view["packs"].as_array().cloned().unwrap_or_default();
    packs.iter().filter_map(|p| p["namespace"].as_str().map(str::to_owned)).collect()
}

#[test]
fn request_layouts_leaves_a_disabled_layout_pack_out() {
    let mut s = session();
    pin_acme(&mut s);
    // Une épingle dont les besoins diffèrent du manifeste : à la resynchronisation, le pack est écarté.
    let mut deck = s.deck().clone();
    deck.layouts.get_mut(&crate::test_support::ns("acme")).unwrap().requires.clear();
    s.doc.sources.apply_silent(deck).unwrap();
    s.sync_widgets().unwrap();
    assert_that!(s.widgets_disabled.contains_key(&crate::test_support::ns("acme"))).is_true();
    assert_that!(listed(&mut s)).is_equal_to(vec!["essentiel".to_owned()]);
}

#[test]
fn request_layouts_leaves_a_layout_pack_missing_from_the_sources_out() {
    let mut s = session();
    let pin = pin_acme(&mut s);
    s.doc.sources.packs.remove(&pin.version);
    assert_that!(listed(&mut s)).is_equal_to(vec!["essentiel".to_owned()]);
}
