use std::io::Write;

use speculoos::prelude::*;

use super::*;
use crate::MAX_SLOT_DEPTH;
use crate::error::{ReplicaErrorKind, SourcesErrorKind};
use crate::{PackFiles, PackRef};
use oku_slides_i18n::Localize;

const MAX: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26 };

/// Un zip fait à la main, entrée par entrée.
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        for (name, data) in entries {
            zw.start_file(*name, SimpleFileOptions::default()).unwrap();
            zw.write_all(data).unwrap();
        }
        zw.finish().unwrap();
    }
    buf.into_inner()
}

const MANIFEST: &[u8] = br#"{"format":"oku-slides-sources","version":5}"#;
/// L'ancien `deck.json` (sources v1) : ignoré depuis les sources v2.
const DECK_JSON: &[u8] = br#"{"slides":[]}"#;

/// Un `deck.loro` valide : le snapshot d'un deck vide.
fn deck_loro() -> Vec<u8> {
    let empty = Deck::from_json(std::str::from_utf8(DECK_JSON).unwrap()).unwrap();
    DeckReplica::new(&empty, crate::test_registry::shared()).unwrap().snapshot().unwrap()
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap()
}

#[test]
fn from_zip_not_a_zip_returns_unreadable() {
    let err = SourceBundle::from_zip(b"pas un zip", MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Unreadable);
}

#[test]
fn from_zip_without_manifest_returns_not_sources() {
    let err =
        SourceBundle::from_zip(&zip(&[("deck.loro", &deck_loro())]), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_other_format_returns_not_sources() {
    let manifest = br#"{"format":"slider-sources","version":2}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_newer_version_returns_unsupported_version() {
    let manifest = br#"{"format":"oku-slides-sources","version":99}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::UnsupportedVersion);
}

/// Sources v4 : un pack à un seul `widgets.typ`, qui ne se relit pas en v5.
#[test]
fn from_zip_with_sources_version_4_returns_not_sources() {
    let manifest = br#"{"format":"oku-slides-sources","version":4}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_version_1_returns_not_sources() {
    let manifest = br#"{"format":"oku-slides-sources","version":1}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.json", DECK_JSON)]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

#[test]
fn from_zip_without_deck_loro_returns_missing_deck() {
    let err =
        SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST)]), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::MissingDeck);
}

#[test]
fn from_zip_deck_json_only_returns_missing_deck() {
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", MANIFEST), ("deck.json", DECK_JSON)]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::MissingDeck);
}

#[test]
fn from_zip_invalid_deck_loro_returns_replica_corrupted() {
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", MANIFEST), ("deck.loro", b"pas loro")]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Replica(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_zip_deck_loro_over_max_returns_replica_too_large() {
    let loro = deck_loro();
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", MANIFEST), ("deck.loro", &loro)]),
        SourceLimits { replica_bytes: loro.len() - 1, ..MAX },
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Replica(ReplicaErrorKind::TooLarge));
}

#[test]
fn from_zip_hidden_font_name_returns_bad_name() {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("fonts/.cache", b"x")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_unknown_entries_are_ignored() {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("history.jsonl", b"{}")];
    assert_that!(SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared())).is_ok();
}

#[test]
fn to_zip_then_from_zip_returns_same_bundle() {
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    b.add_asset(b"png".to_vec());
    let (back, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&b);
    assert_that!(back.replica().deck_uid()).is_equal_to(b.replica().deck_uid());
}

#[test]
fn to_zip_has_no_deck_json() {
    let zip = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap().to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    assert_that!(za.file_names().any(|n| n == "deck.json")).is_false();
    assert_that!(za.file_names().any(|n| n == "deck.loro")).is_true();
}

#[test]
fn apply_then_undo_restores_view() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut next = deck.clone();
    next.theme = Some("pro".into());
    b.apply(next, "Thème", None).unwrap();
    assert_that!(b.deck().theme.as_deref()).is_equal_to(Some("pro"));
    assert_that!(b.undo().unwrap()).is_true();
    assert_that!(b.deck()).is_equal_to(&deck);
}

#[test]
fn apply_silent_then_undo_returns_false() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut next = deck;
    next.theme = Some("pro".into());
    b.apply_silent(next).unwrap();
    assert_that!(b.undo().unwrap()).is_false();
    assert_that!(b.deck().theme.as_deref()).is_equal_to(Some("pro"));
}

#[test]
fn undo_then_redo_restores_applied_view() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut next = deck;
    next.theme = Some("pro".into());
    b.apply(next.clone(), "Thème", None).unwrap();
    b.undo().unwrap();
    assert_that!(b.redo().unwrap()).is_true();
    assert_that!(b.deck()).is_equal_to(&next);
}

#[test]
fn from_zip_assets_over_total_limit_returns_too_large() {
    let loro = deck_loro();
    let big = vec![1u8; 4096];
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("assets/aa", &big), ("assets/bb", &big)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + big.len() + 100, ..MAX };
    let err = SourceBundle::from_zip(&zip(entries), limits, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to("Les sources de ce fichier sont trop volumineuses pour être ouvertes".to_owned());
}

#[test]
fn from_zip_assets_within_total_limit_are_read() {
    let loro = deck_loro();
    let big = vec![1u8; 4096];
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("assets/aa", &big)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + big.len(), ..MAX };
    let (b, _) = SourceBundle::from_zip(&zip(entries), limits, crate::test_registry::shared()).unwrap();
    assert_that!(b.assets.get("sha256:aa")).is_equal_to(Some(&big));
}

/// Bombe de décompression : quelques Kio compressés, 64 Mio une fois lus.
#[test]
fn from_zip_deflate_bomb_returns_too_large() {
    let zeros = vec![0u8; 64 << 20];
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), ("fonts/f.ttf", &zeros)];
    let bomb = zip(entries);
    assert_that!(bomb.len()).is_less_than(1 << 20);
    let err =
        SourceBundle::from_zip(&bomb, SourceLimits { total_bytes: 1 << 20, ..MAX }, crate::test_registry::shared())
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
}

#[test]
fn sources_error_localizes_by_kind() {
    let err =
        SourceBundle::from_zip(&zip(&[("manifest.json", MANIFEST)]), MAX, crate::test_registry::shared()).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to("Le deck manque dans les sources".to_owned());
}

#[test]
fn sources_error_replica_localizes_as_replica_error() {
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", MANIFEST), ("deck.loro", b"x")]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let expected = oku_slides_i18n::Localize::localize(
        &DeckReplica::from_snapshot(b"x", MAX.replica_bytes, crate::test_registry::shared()).unwrap_err(),
        &fr,
    );
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr)).is_equal_to(expected);
}

#[test]
fn merge_unions_assets_and_keeps_our_font_on_name_clash() {
    let deck = roadmap();
    let mut a = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let snap = a.to_zip().unwrap();
    let mut b = SourceBundle::from_zip(&snap, MAX, crate::test_registry::shared()).unwrap().0;
    a.fonts.insert("f.ttf".into(), b"n\xc3\xb4tre".to_vec());
    b.fonts.insert("f.ttf".into(), b"sienne".to_vec());
    let id = b.add_asset(b"png".to_vec());
    a.merge(b).unwrap();
    assert_that!(a.assets.contains_key(&id)).is_true();
    assert_that!(a.fonts.get("f.ttf").map(Vec::as_slice)).is_equal_to(Some(&b"n\xc3\xb4tre"[..]));
}

#[test]
fn merge_unrelated_bundles_returns_replica_unrelated() {
    let mut a = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let err = a.merge(b).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::Replica(ReplicaErrorKind::Unrelated));
}

/// `roadmap` et, dans les blocs de `s-constat`, une carte `carte` titrée.
fn roadmap_with_card() -> Deck {
    let mut deck = roadmap();
    let mut card = crate::Element::new("carte", "card");
    card.params.set("title", crate::ParamValue::Text("Titre".into()));
    let slot =
        crate::SlotRef { owner: crate::Target { slide: "s-constat".into(), element: None }, param: "blocks".into() };
    crate::apply_all(
        &mut deck,
        vec![crate::Op::AddElement { slot, index: None, defaults: false, element: card }],
        &crate::test_registry::registry(),
    )
    .unwrap();
    deck
}

fn with_op(deck: &Deck, op: crate::Op) -> Deck {
    let mut d = deck.clone();
    crate::apply_all(&mut d, vec![op], &crate::test_registry::registry()).unwrap();
    d
}

fn card() -> crate::Target {
    crate::Target { slide: "s-constat".into(), element: Some("carte".into()) }
}

/// Les problèmes de l'élément `element`.
fn ids_of(issues: &[Issue], element: &str) -> Vec<&'static str> {
    issues.iter().filter(|i| i.element.as_deref() == Some(element)).map(|i| i.message.id()).collect()
}

#[test]
fn merge_of_concurrent_set_kind_and_param_edit_returns_unknown_param() {
    let deck = roadmap_with_card();
    let zip = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap().to_zip().unwrap();
    let (mut a, _) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    let (mut b, _) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    a.apply(with_op(&deck, crate::Op::SetKind { target: card(), kind: "text".into() }), "A", None).unwrap();
    let edit = crate::Op::SetParam { target: card(), param: "title".into(), value: serde_json::json!("Révisé") };
    b.apply(with_op(&deck, edit), "B", None).unwrap();
    let (_, issues) = a.merge(b).unwrap();
    assert_that!(ids_of(&issues, "carte")).is_equal_to(vec!["issue-unknown-param"]);
}

#[test]
fn restore_of_a_version_with_an_unknown_param_returns_it() {
    let deck = roadmap_with_card();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    // `SetKind` garde le titre, inconnu du widget `text`.
    b.apply(with_op(&deck, crate::Op::SetKind { target: card(), kind: "text".into() }), "texte", None).unwrap();
    b.apply(deck.clone(), "carte", None).unwrap();
    let preview = b.history();
    let issues = b.restore(&preview, 1, "restaurer").unwrap();
    assert_that!(ids_of(&issues, "carte")).is_equal_to(vec!["issue-unknown-param"]);
}

#[test]
fn restore_of_a_version_with_a_missing_asset_returns_it() {
    let deck = roadmap();
    let mut image = crate::Element::new("img", "image");
    image.params.set("asset", crate::ParamValue::Text("sha256:00".into()));
    let slot =
        crate::SlotRef { owner: crate::Target { slide: "s-constat".into(), element: None }, param: "blocks".into() };
    let with_image = with_op(&deck, crate::Op::AddElement { slot, index: None, defaults: false, element: image });
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    b.apply(with_image, "image", None).unwrap();
    let preview = b.history();
    let issues = b.restore(&preview, 1, "restaurer").unwrap();
    assert_that!(ids_of(&issues, "img")).is_equal_to(vec!["issue-missing-asset"]);
}

/// Un paramètre `value` (option de graphique) relu après annuler s'écrit
/// comme la vue d'avant : mêmes clés, même ordre (le rendu hache ce JSON).
#[test]
fn undo_rereads_value_params_with_the_same_json_as_the_view() {
    let mut deck = roadmap();
    let option: serde_json::Value = serde_json::from_str(
        r#"{"xAxis":{"type":"category"},"dataset":{"source":[["a","b"],["x",1.0]]},"series":[{"type":"bar"}]}"#,
    )
    .unwrap();
    let mut chart = crate::Element::new("g", "chart");
    chart.params.set("option", crate::ParamValue::Value(option));
    let slide = deck.slide_mut("s-constat").unwrap();
    slide.params.slot_mut("blocks").unwrap().push(chart);
    crate::canonicalize(&mut deck);
    let mut sources = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    sources.apply(deck, "graphique", None).unwrap();
    let before = sources.deck().to_json().unwrap();
    let mut other = sources.deck().clone();
    other.theme = Some("pro".into());
    sources.apply(other, "thème", None).unwrap();
    sources.undo().unwrap();
    assert_that!(sources.deck().to_json().unwrap()).is_equal_to(before);
}

/// Un deck d'une slide `blank` portant une chaîne de `n` canvas imbriqués.
fn canvas_chain(n: usize) -> Deck {
    let mut tree = serde_json::json!({ "id": format!("c{}", n - 1), "kind": "canvas" });
    for i in (0..n - 1).rev() {
        tree = serde_json::json!({ "id": format!("c{i}"), "kind": "canvas", "params": { "children": [tree] } });
    }
    let deck = serde_json::json!({ "slides": [{ "id": "s", "kind": "blank", "params": { "elements": [tree] } }] });
    serde_json::from_value(deck).unwrap_or_else(|e| panic!("deck de test : {e}"))
}

#[test]
fn new_with_elements_nested_too_deep_returns_too_deep() {
    let Err(err) = SourceBundle::new(canvas_chain(20), crate::test_registry::shared()) else {
        panic!("un deck trop profond devait être refusé")
    };
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooDeep);
    let en = err.localize(oku_slides_i18n::english());
    assert_that!(en).is_equal_to("This deck nests elements too deeply".to_owned());
    let fr = err.localize(&oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(fr).is_equal_to("Ce deck imbrique des éléments trop profondément".to_owned());
}

#[test]
fn new_with_elements_nested_to_the_slot_bound_opens() {
    let deck = canvas_chain(MAX_SLOT_DEPTH);
    let bundle = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    assert_that!(bundle.deck().slides[0].walk().count()).is_equal_to(MAX_SLOT_DEPTH);
}

/// Un pack de test : `pack.json` minimal et ses fichiers de widgets.
fn pack(name: &str, widgets: &[(&str, &str)]) -> PackFiles {
    let manifest = serde_json::json!({ "id": "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10", "name": name });
    let widgets = widgets.iter().map(|(n, s)| (crate::WidgetFile::parse(n).unwrap(), (*s).to_owned())).collect();
    PackFiles::new(manifest.as_object().cloned().unwrap(), widgets)
}

/// `roadmap` qui référence `files`, et son bundle qui l'embarque.
fn bundle_with(files: &PackFiles) -> SourceBundle {
    let mut deck = roadmap();
    deck.templates = Some(PackRef { id: uuid::Uuid::from_u128(1), version: files.version() });
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    b.add_pack(files.clone());
    b
}

#[test]
fn to_zip_then_from_zip_keeps_packs_and_each_widget_file() {
    let files = pack("P", &[("kpi", "#let widget = none\n"), ("_commun", "#let x = 1\n")]);
    let b = bundle_with(&files);
    let zip = b.to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip.clone())).unwrap();
    let hex = files.version().hex().to_owned();
    for name in ["pack.json", "widgets/kpi.typ", "widgets/_commun.typ"] {
        assert_that!(za.file_names().any(|n| n == format!("packs/{hex}/{name}"))).named(name).is_true();
    }
    let (back, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(back.packs.get(&files.version())).is_equal_to(Some(&files));
}

#[test]
fn to_zip_of_a_pack_without_widgets_writes_no_widgets_folder() {
    let files = pack("P", &[]);
    let zip = bundle_with(&files).to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    assert_that!(za.file_names().any(|n| n.contains("/widgets"))).is_false();
}

#[test]
fn from_zip_pack_whose_content_differs_from_its_hash_is_discarded_and_reported() {
    let loro = deck_loro();
    let forged = "f".repeat(64);
    let name = format!("packs/{forged}/pack.json");
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), br#"{"name":"P"}"#)];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-mismatch"]);
}

#[rstest::rstest]
#[case("packs/abc/pack.json")]
#[case("packs/FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF/pack.json")]
#[case("packs/../pack.json")]
fn from_zip_pack_with_an_invalid_path_returns_bad_name(#[case] name: &str) {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, b"{}")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_deck_referencing_an_absent_pack_reports_it() {
    let files = pack("P", &[]);
    let mut b = bundle_with(&files);
    b.packs.clear();
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-missing"]);
}

#[test]
fn from_zip_packs_count_in_the_total_limit() {
    let loro = deck_loro();
    let files = pack(&"x".repeat(4096), &[]);
    let name = format!("packs/{}/pack.json", files.version().hex());
    let json = files.pack_json().unwrap();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), &json)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + 100, ..MAX };
    let err = SourceBundle::from_zip(&zip(entries), limits, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
}

#[test]
fn merge_unions_the_packs_of_both_bundles() {
    let (p, q) = (pack("P", &[]), pack("Q", &[("kpi", "#let registry = ()")]));
    let mut a = bundle_with(&p);
    let mut b = SourceBundle::from_zip(&a.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap().0;
    b.add_pack(q.clone());
    a.merge(b).unwrap();
    assert_that!(a.packs.keys().cloned().collect::<Vec<_>>()).is_equal_to({
        let mut v = vec![p.version(), q.version()];
        v.sort();
        v
    });
}

/// Le mode historique restaure aussi le pack d'une version passée : toute
/// version que le replica a référencée reste dans le fichier.
#[test]
fn versions_referenced_by_the_history_are_kept_on_save() {
    let (p, q) = (pack("P", &[]), pack("Q", &[]));
    let mut b = bundle_with(&p);
    b.add_pack(q.clone());
    let mut next = b.deck().clone();
    next.templates = Some(PackRef { id: uuid::Uuid::from_u128(1), version: q.version() });
    b.apply(next, "pack", None).unwrap();
    let (back, _) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(back.packs.contains_key(&p.version()) && back.packs.contains_key(&q.version())).is_true();
    let preview = back.history();
    let mut restored = back;
    restored.restore(&preview, 0, "restaurer").unwrap();
    assert_that!(restored.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(p.version()));
}

#[test]
fn from_zip_pack_with_widgets_but_no_pack_json_is_discarded_and_reported() {
    let loro = deck_loro();
    let name = format!("packs/{}/widgets/kpi.typ", "a".repeat(64));
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), b"#let registry = ()")];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-mismatch"]);
}

/// t-328 : les entrées de répertoire `packs/` et `packs/<hex>/` sont ignorées.
#[rstest::rstest]
#[case("packs/")]
#[case(&format!("packs/{}/", "a".repeat(64)))]
#[case(&format!("packs/{}/widgets/", "a".repeat(64)))]
#[case(&format!("packs/{}/widgets.typ", "a".repeat(64)))]
fn from_zip_bare_packs_directory_entries_are_ignored(#[case] name: &str) {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, b"")];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    assert_that!(issues.is_empty()).is_true();
}

/// t-329 : un nom invalide sous `packs/<hex>/widgets/` refuse le zip.
#[rstest::rstest]
#[case("widgets/Kpi.typ")]
#[case("widgets/_autre.typ")]
#[case("widgets/kpi")]
#[case("widgets/a/b.typ")]
#[case("autre/x.typ")]
fn from_zip_pack_with_an_invalid_widget_path_returns_bad_name(#[case] file: &str) {
    let loro = deck_loro();
    let name = format!("packs/{}/{file}", "a".repeat(64));
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), b"x")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_pack_whose_widget_file_was_altered_is_discarded_and_reported() {
    let files = pack("P", &[("kpi", "#let widget = none\n")]);
    let bytes = bundle_with(&files).to_zip().unwrap();
    // Réécrit le zip avec un `kpi.typ` modifié : l'empreinte ne correspond plus.
    let mut za = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..za.len() {
        let mut f = za.by_index(i).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data).unwrap();
        if f.name().ends_with("widgets/kpi.typ") {
            data.extend_from_slice(b"// altere\n");
        }
        entries.push((f.name().to_owned(), data));
    }
    let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    let (b, issues) = SourceBundle::from_zip(&zip(&refs), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).contains("issue-pack-version-mismatch");
}
