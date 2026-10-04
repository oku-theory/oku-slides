use speculoos::prelude::*;

use super::*;
use crate::test_support::registry;
use crate::views::CodePlace;
use crate::views::PileSection;
use oku_slides_core::{Deck, Op, SourceBundle};
use oku_slides_platform::storage::StorageErrorKind;

/// Réglages de test : 10 étapes d'annulation, sans auteur réglé.
fn defaults() -> DocDefaults {
    DocDefaults { undo_limit: 10, author: None }
}

fn temp_dir() -> PathBuf {
    crate::test_support::fresh_dir("oku-slides-resume-").keep()
}

fn resume() -> Resume {
    Resume::new(Storage::local().unwrap(), temp_dir())
}

fn deck() -> Deck {
    Deck::from_json(r#"{"theme":"papier","slides":[]}"#).unwrap()
}

fn document() -> Document {
    let mut sources = SourceBundle::new(deck(), registry()).unwrap();
    sources.add_asset(b"image".to_vec());
    sources.fonts.insert("Inter.ttf".into(), b"police".to_vec());
    let mut doc = Document::new(sources, Some(PathBuf::from("/tmp/essai.oku-slides.pdf")), &defaults());
    let mut next = doc.sources.deck().clone();
    next.theme = Some("pro".into());
    doc.sources.apply(next, "Thème", None).unwrap();
    doc.dirty = true;
    doc.saved_hash = Some("sha256:00".into());
    doc
}

#[test]
fn write_snapshot_then_read_doc_round_trips_sources_uid_and_meta() {
    let r = resume();
    let doc = document();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources).is_equal_to(&doc.sources);
    assert_that!(back.sources.replica().deck_uid()).is_equal_to(doc.sources.replica().deck_uid());
    assert_that!(back.path).is_equal_to(&doc.path);
    assert_that!(back.dirty).is_true();
    assert_that!(back.saved_hash).is_equal_to(&doc.saved_hash);
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_snapshot_removes_orphan_assets() {
    let r = resume();
    let mut doc = document();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    doc.sources.assets.clear();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    assert_that!(r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap().sources.assets.len()).is_equal_to(0);
    std::fs::remove_dir_all(&r.dir).ok();
}

/// Un `deck.loro` abîmé rend le document illisible (l'onglet est mis de côté).
#[test]
fn read_doc_with_corrupt_deck_loro_returns_error() {
    let r = resume();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&document(), &mut Vec::new()).unwrap()).unwrap();
    std::fs::write(r.dir.join(DECK_DIR).join("deck.loro"), b"illisible").unwrap();
    assert_that!(r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).is_err()).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

/// Un `deck.loro` au-delà du maximum n'est pas relu.
#[test]
fn read_doc_with_deck_loro_over_the_maximum_returns_error() {
    let r = resume();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&document(), &mut Vec::new()).unwrap()).unwrap();
    assert_that!(r.read_doc(DECK_DIR, &defaults(), 16, &registry()).is_err()).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_state_missing_returns_none() {
    let r = resume();
    assert_that!(r.read_state().unwrap()).is_none();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_state_then_read_state_round_trips_mode_and_ui() {
    let r = resume();
    let v = crate::library::builtin_versions()[0].version.clone();
    let edit = TemplateMode {
        set: crate::DEFAULT_PACK,
        name: "Essentiel".into(),
        builtin: true,
        base: v.clone(),
        parent: Some(v),
        session_version: None,
    };
    let ui = UiState {
        slide: Some("s-1".into()),
        grid: false,
        ghosts: true,
        pile: Some(200),
        notes: Some(520),
        widgets: vec!["_commun".into(), "kpi".into()],
        widget: Some("kpi".into()),
        code: true,
        code_place: CodePlace::Right,
        code_height: Some(480),
        code_width: Some(640),
        folded: vec![PileSection::Widgets],
        case: [("kpi".to_owned(), "Long".to_owned())].into(),
        board: true,
    };
    r.write_state(&State { version: VERSION, mode: SavedMode::Templates(edit), ui: ui.clone(), help_theme: None })
        .unwrap();
    let back = r.read_state().unwrap().unwrap();
    assert_that!(matches!(back.mode, SavedMode::Templates(ref m) if m.set == crate::DEFAULT_PACK)).is_true();
    assert_that!(back.ui).is_equal_to(ui);
    std::fs::remove_dir_all(&r.dir).ok();
}

/// Un `state.json` écrit avant les fantômes et les colonnes se relit, fantômes masqués, largeurs par défaut.
#[test]
fn read_state_without_ghosts_columns_or_tabs_returns_defaults() {
    let r = resume();
    std::fs::create_dir_all(&r.dir).unwrap();
    let state = format!(r#"{{"version":{VERSION},"mode":"deck","ui":{{"slide":"s-1","grid":false}}}}"#);
    std::fs::write(r.dir.join("state.json"), state).unwrap();
    let back = r.read_state().unwrap().unwrap();
    assert_that!(back.ui).is_equal_to(UiState {
        slide: Some("s-1".into()),
        grid: false,
        ghosts: false,
        pile: None,
        notes: None,
        widgets: Vec::new(),
        widget: None,
        code: false,
        code_place: CodePlace::Below,
        code_height: None,
        code_width: None,
        folded: Vec::new(),
        case: std::collections::BTreeMap::new(),
        board: false,
    });
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_state_unknown_version_returns_error() {
    let r = resume();
    std::fs::create_dir_all(&r.dir).unwrap();
    std::fs::write(r.dir.join("state.json"), br#"{"version":99,"mode":"deck"}"#).unwrap();
    assert_that!(matches!(r.read_state(), Err(ResumeError::Version { version: 99, .. }))).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

/// Un `state.json` valide mais démesuré n'est pas lu en entier : illisible.
#[test]
fn read_state_oversized_returns_access_error_too_large() {
    let r = resume();
    let mut bytes = br#"{"version":1,"mode":"deck"}"#.to_vec();
    bytes.resize(usize::try_from(MAX_JSON_BYTES).unwrap() + 1, b' ');
    std::fs::create_dir_all(&r.dir).unwrap();
    std::fs::write(r.dir.join("state.json"), bytes).unwrap();
    let err = r.read_state().unwrap_err();
    let too_large = matches!(&err, ResumeError::Access { source, .. } if source.kind() == StorageErrorKind::TooLarge);
    assert_that!(too_large).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_state_future_version_with_other_shape_returns_version_error() {
    let r = resume();
    std::fs::create_dir_all(&r.dir).unwrap();
    std::fs::write(r.dir.join("state.json"), br#"{"version":2,"mode":{"nouveau":1}}"#).unwrap();
    assert_that!(matches!(r.read_state(), Err(ResumeError::Version { version: 2, .. }))).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_snapshot_font_changed_under_same_name_reads_new_bytes() {
    let r = resume();
    let mut doc = document();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    doc.sources.fonts.insert("Inter.ttf".into(), b"nouvelle police".to_vec());
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources.fonts.get("Inter.ttf").cloned()).is_equal_to(Some(b"nouvelle police".to_vec()));
    let fonts_dir = r.dir.join(DECK_DIR).join("fonts");
    assert_that!(std::fs::read_dir(&fonts_dir).unwrap().count()).is_equal_to(1);
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_snapshot_unsafe_names_writes_nothing_outside_dir() {
    let r = resume();
    let mut doc = document();
    doc.sources.assets.insert("sha256:../x".into(), b"evil".to_vec());
    doc.sources.fonts.insert("../evil.ttf".into(), b"evil".to_vec());
    doc.sources.fonts.insert(".hidden.ttf".into(), b"evil".to_vec());
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    assert_that!(r.dir.join(DECK_DIR).join("assets").join("x").exists()).is_false();
    assert_that!(r.dir.join(DECK_DIR).join("x").exists()).is_false();
    assert_that!(r.dir.parent().unwrap().join("x").exists()).is_false();
    assert_that!(r.dir.join(DECK_DIR).join("fonts").join("evil.ttf").exists()).is_false();
    assert_that!(r.dir.join(DECK_DIR).join("evil.ttf").exists()).is_false();
    assert_that!(r.dir.parent().unwrap().join("evil.ttf").exists()).is_false();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources.assets.contains_key("sha256:../x")).is_false();
    assert_that!(back.sources.fonts.contains_key("evil.ttf")).is_false();
    assert_that!(back.sources.fonts.contains_key(".hidden.ttf")).is_false();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_doc_with_stored_path_not_oku_slides_drops_path() {
    let r = resume();
    let mut doc = document();
    doc.path = Some(PathBuf::from("/tmp/notes.txt"));
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    assert_that!(r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap().path).is_none();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_doc_with_stored_relative_path_drops_path() {
    let r = resume();
    let mut doc = document();
    doc.path = Some(PathBuf::from("essai.oku-slides.pdf"));
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    assert_that!(r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap().path).is_none();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_snapshot_then_read_doc_keeps_the_blank_flag() {
    let r = resume();
    let mut doc = document();
    doc.blank = true;
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    assert_that!(r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap().blank).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_snapshot_then_read_doc_keeps_a_non_blank_flag() {
    let r = resume();
    let doc = document();
    assert_that!(doc.blank).is_false();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    assert_that!(r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap().blank).is_false();
    std::fs::remove_dir_all(&r.dir).ok();
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

/// t-143 : au-delà de `keep`, les plus anciens partent ; l'ordre est celui
/// de l'horodatage puis du suffixe, en nombres (pas alphabétique).
#[test]
fn failed_to_prune_beyond_keep_returns_the_oldest_in_numeric_order() {
    let found = names(&[
        "resume.failed-100",
        "resume.failed-99",
        "resume.failed-100-2",
        "resume.failed-100-10",
        "resume.failed-1000",
    ]);
    let gone = failed_to_prune(&found, |b| b == "resume", 2);
    assert_that!(gone).is_equal_to(names(&["resume.failed-99", "resume.failed-100", "resume.failed-100-2"]));
}

#[test]
fn failed_to_prune_within_keep_returns_nothing() {
    let found = names(&["resume.failed-1", "resume.failed-2"]);
    assert_that!(failed_to_prune(&found, |b| b == "resume", 3)).is_empty();
}

/// Seuls les dossiers au motif exact sont candidats : jamais un nom voisin.
#[test]
fn failed_to_prune_ignores_names_outside_the_pattern() {
    let found = names(&[
        "resume",
        "resume.failed-",
        "resume.failed-12x",
        "resume.failed-1-",
        "resume.failed-1-2-3",
        "autre.failed-5",
        "resume.failed-7",
    ]);
    assert_that!(failed_to_prune(&found, |b| b == "resume", 0)).is_equal_to(names(&["resume.failed-7"]));
}

/// Onglets : un seul ensemble, tous identifiants confondus.
#[test]
fn failed_to_prune_across_tab_folders_keeps_the_most_recent_overall() {
    let found = names(&["t-1.failed-10", "t-2.failed-30", "t-3.failed-20", "t-2", "t-x.failed-40"]);
    let is_tab = |b: &str| b.strip_prefix("t-").is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()));
    assert_that!(failed_to_prune(&found, is_tab, 2)).is_equal_to(names(&["t-1.failed-10"]));
}

/// t-224 : l'agent modifie la session directement (`Session::apply`), sans
/// passer par `handle` : l'op doit être écrite pour la reprise.
#[test]
fn apply_without_handle_writes_the_op_for_resume() {
    let dir = temp_dir();
    let mut s = crate::test_support::session();
    assert_that!(s.resume(dir.clone())).is_true();
    s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }], None).unwrap();
    s.flush_resume();
    let mut back = crate::test_support::session();
    assert_that!(back.resume(dir.clone())).is_true();
    assert_that!(back.deck().theme.as_deref()).is_equal_to(Some("pro"));
    std::fs::remove_dir_all(&dir).ok();
}

/// t-224 : `Session::new_deck` appelé directement (sans `handle`) doit lui
/// aussi être écrit : la reprise retrouve un deck vierge.
#[test]
fn new_deck_without_handle_resumes_to_a_blank_deck() {
    let dir = temp_dir();
    let mut s = crate::test_support::session();
    assert_that!(s.resume(dir.clone())).is_true();
    s.new_deck().unwrap();
    s.flush_resume();
    let mut back = crate::test_support::session();
    assert_that!(back.resume(dir.clone())).is_true();
    assert_that!(back.tab_info().pristine).is_true();
    std::fs::remove_dir_all(&dir).ok();
}

/// Un pack de l'utilisateur, avec ses fichiers `widgets`.
fn pack_files(name: &str, widgets: &[(&str, &str)]) -> oku_slides_core::PackFiles {
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), name, None, "2026-10-01T10:00:00Z");
    m.to_files(crate::test_support::widget_files(widgets)).unwrap()
}

/// Les fichiers `<hex>.*.typ` de `packs`.
fn widget_files_of(packs: &std::path::Path, hex: &str) -> Vec<String> {
    let prefix = format!("{hex}.");
    std::fs::read_dir(packs)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with(&prefix) && n.ends_with(".typ"))
        .collect()
}

#[test]
fn write_snapshot_then_read_doc_keeps_the_packs_and_their_widgets() {
    let r = resume();
    let mut doc = document();
    let with = doc.sources.add_pack(pack_files("Avec", &[("kpi", "K"), ("_commun", "C")]));
    let without = doc.sources.add_pack(pack_files("Sans", &[]));
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    let packs = r.dir.join(DECK_DIR).join("packs");
    assert_that!(packs.join(format!("{}.kpi.typ", with.hex())).exists()).is_true();
    assert_that!(packs.join(format!("{}._commun.typ", with.hex())).exists()).is_true();
    assert_that!(widget_files_of(&packs, without.hex())).is_empty();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources.packs == doc.sources.packs).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_doc_ignores_a_pack_whose_content_differs_from_its_name() {
    let r = resume();
    let mut doc = document();
    let v = doc.sources.add_pack(pack_files("Avec", &[("kpi", "K")]));
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    let typ = r.dir.join(DECK_DIR).join("packs").join(format!("{}.kpi.typ", v.hex()));
    std::fs::write(&typ, "autre").unwrap();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources.packs.is_empty()).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_snapshot_removes_orphan_packs() {
    let r = resume();
    let mut doc = document();
    let v = doc.sources.add_pack(pack_files("Avec", &[("kpi", "K")]));
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    doc.sources.packs.clear();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    let packs = r.dir.join(DECK_DIR).join("packs");
    assert_that!(packs.join(format!("{}.json", v.hex())).exists()).is_false();
    assert_that!(packs.join(format!("{}.kpi.typ", v.hex())).exists()).is_false();
    std::fs::remove_dir_all(&r.dir).ok();
}

/// Un cas d'exemple est dans le pack : son écriture écrit les documents de reprise.
#[test]
fn set_example_value_writes_the_resume_documents() {
    let data = crate::test_support::fresh_dir("oku-slides-example-");
    let dir = temp_dir();
    let mut s = crate::test_support::opened_on(data.path());
    assert_that!(s.resume(dir.clone())).is_true();
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    let source = "#import \"/oku-slides/lib.typ\": widget, params as p\n\
                  #let widget = widget(\"essai\", params: (title: p.plain()), (args, ctx) => args.title)\n";
    s.handle(crate::Request::SetWidget { name: "essai".into(), source: source.into() }).unwrap();
    s.flush_resume();
    let docs = [dir.join(DECK_DIR).join("document.json"), dir.join(TEMPLATES_DIR).join("document.json")];
    for d in &docs {
        std::fs::remove_file(d).unwrap();
    }
    let req = crate::Request::SetExampleValue {
        name: "essai".into(),
        example: None,
        param: "title".into(),
        value: "Météo".into(),
    };
    s.handle(req).unwrap();
    s.flush_resume();
    assert_that!(docs.iter().filter(|d| d.exists()).count()).is_equal_to(2);
    std::fs::remove_dir_all(&dir).ok();
}
