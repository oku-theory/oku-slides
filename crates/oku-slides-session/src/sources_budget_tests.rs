use oku_slides_core::Op;
use oku_slides_i18n::{Localize, Localizer};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::import::{ImportErrorKind, ImportRequest, ImportTarget, PackLookup, import};
use crate::test_support::{fresh_dir, opened_on, png, pptx_fixture, session, session_on, settings_on};
use crate::{Fonts, Platform, SessionErrorKind};

/// Sets `s`'s sources budget one byte above what its sources weigh now.
fn tight(s: &mut Session) {
    let total = source_bytes(&s.doc.sources).unwrap();
    s.source_limits.total_bytes = usize::try_from(total).unwrap() + 1;
}

/// The op that gives `asset` to the image `e-image` of `s-libre` (roadmap.json).
fn set_image(asset: &str) -> Op {
    serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "asset", "value": asset
    }))
    .unwrap()
}

#[test]
fn ensure_sources_room_within_budget_accepts() {
    let mut s = session();
    tight(&mut s);
    assert_that!(s.ensure_sources_room(1)).is_ok();
}

#[test]
fn ensure_sources_room_over_budget_refuses_with_kind() {
    let mut s = session();
    tight(&mut s);
    let e = s.ensure_sources_room(2).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::SourcesOverBudget);
}

#[test]
fn embed_op_assets_over_budget_adds_nothing() {
    let mut s = session();
    let pasted = s.paste_image(png(4, 4, 21), &[]).unwrap();
    tight(&mut s);
    let deck = s.doc.sources.deck().clone();
    let e = s.apply("image", vec![set_image(&pasted.id)], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::SourcesOverBudget);
    assert_that!(s.doc.sources.assets.contains_key(&pasted.id)).is_false();
    assert_that!(s.doc.sources.deck()).is_equal_to(&deck);
}

#[test]
fn embed_op_assets_known_asset_costs_nothing() {
    let mut s = session();
    let id = s.doc.sources.add_asset(png(4, 4, 22));
    tight(&mut s);
    assert_that!(s.apply("image", vec![set_image(&id)], None)).is_ok();
}

#[test]
fn embed_op_themes_over_budget_refuses() {
    let mut s = session();
    tight(&mut s);
    let (deck, themes) = (s.doc.sources.deck().clone(), s.doc.sources.themes.len());
    let op = Op::SetTheme {
        theme: Some(crate::test_support::ns(crate::theme::PRO_NAMESPACE)),
        themes: std::collections::BTreeMap::new(),
    };
    let e = s.apply("Thème", vec![op], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::SourcesOverBudget);
    assert_that!(s.doc.sources.themes.len()).is_equal_to(themes);
    assert_that!(s.doc.sources.deck()).is_equal_to(&deck);
}

#[test]
fn embed_from_library_over_budget_refuses() {
    use crate::test_support::{add_pack, add_version, library_of, manifest};
    let data = fresh_dir("oku-slides-budget-pack-");
    let mut s = session_on(data.path());
    let m = manifest(uuid::Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let r = add_version(library_of(&s), &m, &[]);
    tight(&mut s);
    let deck = s.doc.sources.deck().clone();
    let e = s.apply("Pack", vec![add_pack(&m, &r)], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::SourcesOverBudget);
    assert_that!(s.doc.sources.packs.contains_key(&r.version)).is_false();
    assert_that!(s.doc.sources.deck()).is_equal_to(&deck);
}

#[test]
fn merge_over_budget_refuses() {
    let data = fresh_dir("oku-slides-budget-merge-");
    let mut ours = opened_on(data.path());
    let path = data.path().join("deck.oku-slides.pdf");
    ours.save(Some(&path)).unwrap();
    // Their copy of the same deck brings an image ours does not hold.
    let mut theirs = session_on(data.path());
    theirs.open(&path).unwrap();
    let pasted = theirs.paste_image(png(4, 4, 23), &[]).unwrap();
    theirs.apply("image", vec![set_image(&pasted.id)], None).unwrap();
    let copy = data.path().join("copie.oku-slides.pdf");
    theirs.save(Some(&copy)).unwrap();
    // Ours holds a heavier asset theirs lacks: their copy alone reads within the budget.
    ours.doc.sources.add_asset(vec![7; 20_000]);
    tight(&mut ours);
    let e = ours.merge_file(&copy).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::SourcesOverBudget);
    assert_that!(ours.doc.sources.assets.contains_key(&pasted.id)).is_false();
}

/// A grey PNG of `side` × `side` pixels that deflate cannot shrink.
fn noise_png(side: u32) -> Vec<u8> {
    let mut state = 0x9e37_79b9_u32;
    let pixels: Vec<u8> = (0..side * side)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()[0]
        })
        .collect();
    let mut out = std::io::Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, side, side);
    e.set_color(png::ColorType::Grayscale);
    let mut writer = e.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    drop(writer);
    out.into_inner()
}

/// The `kitchen` fixture in `dir`, its image replaced by `image`.
fn kitchen_with(dir: &std::path::Path, image: &[u8]) -> std::path::PathBuf {
    use std::io::{Read, Write};
    let path = pptx_fixture(dir, "kitchen");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        let name = entry.name().unwrap().into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        w.start_file(name.as_str(), zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(if name == "ppt/media/image1.png" { image } else { &bytes }).unwrap();
    }
    std::fs::write(&path, w.finish().unwrap().into_inner()).unwrap();
    path
}

#[test]
fn import_over_budget_refuses() {
    let data = fresh_dir("oku-slides-budget-import-");
    let mut settings = settings_on(data.path());
    settings.sources.max_sources_mib = 1;
    let platform = Platform::with_settings(settings).unwrap();
    // Read within the budget (the plan's estimate counts its image), past it once the packs join.
    let image = noise_png(1000);
    let request = ImportRequest {
        pptx: kitchen_with(data.path(), &image),
        target: ImportTarget::Retrofit { layouts: Vec::new(), theme: None },
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let e = import(&platform, &Fonts::default(), &request).unwrap_err();
    assert_that!(e.kind()).is_equal_to(ImportErrorKind::Library);
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr_with("session-sources-over-budget", &[("limit", 1.into())]));
}

#[test]
fn sources_over_budget_localizes_in_fr() {
    let mut s = session();
    s.source_limits.total_bytes = 3 * 1024 * 1024;
    let e = s.ensure_sources_room(4 * 1024 * 1024).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    let text = e.localize(&fr);
    assert_that!(text.as_str()).is_equal_to(fr.tr_with("session-sources-over-budget", &[("limit", 3.into())]).as_str());
    assert_that!(text.as_str()).contains("3 Mio");
}
