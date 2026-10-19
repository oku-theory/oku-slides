use crate::kinds::base;
use std::io::{Cursor, Write};

use speculoos::prelude::*;

use super::*;
use crate::MAX_SLOT_DEPTH;
use crate::error::{ReplicaErrorKind, SourcesErrorKind};
use oku_slides_i18n::Localize;
use zip::write::SimpleFileOptions;

const MAX: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26, compact_after_secs: 0 };

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

const MANIFEST: &[u8] = br#"{"format":"oku-slides-sources","version":11}"#;
/// L'ancien `deck.json` (sources v1) : ignoré depuis les sources v2.
const DECK_JSON: &[u8] = br#"{"slides":[]}"#;

/// Un `deck.loro` valide : le snapshot d'un deck vide.
fn deck_loro() -> Vec<u8> {
    let empty = Deck::from_json(std::str::from_utf8(DECK_JSON).unwrap()).unwrap();
    DeckReplica::new(&empty, crate::test_registry::shared()).unwrap().snapshot().unwrap()
}

/// `roadmap.json`, sans thème : la version de thème qu'il référence n'est
/// pas embarquée ici ; chaque test de thème pose la sienne.
fn roadmap() -> Deck {
    let mut deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
    deck.theme = None;
    deck
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

/// Sources v6 : refusées.
#[test]
fn sources_v6_zip_is_refused() {
    let manifest = br#"{"format":"oku-slides-sources","version":6}"#;
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
    assert_that!(za.file_names().any(|n| n.unwrap() == "deck.json")).is_false();
    assert_that!(za.file_names().any(|n| n.unwrap() == "deck.loro")).is_true();
}

#[test]
fn apply_then_undo_restores_view() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut next = deck.clone();
    next.theme = Some(crate::test_registry::theme_ref("b"));
    b.apply(next, "Thème", None).unwrap();
    assert_that!(b.deck().theme.clone()).is_equal_to(Some(crate::test_registry::theme_ref("b")));
    assert_that!(b.undo().unwrap()).is_true();
    assert_that!(b.deck()).is_equal_to(&deck);
}

#[test]
fn apply_silent_then_undo_returns_false() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut next = deck;
    next.theme = Some(crate::test_registry::theme_ref("b"));
    b.apply_silent(next).unwrap();
    assert_that!(b.undo().unwrap()).is_false();
    assert_that!(b.deck().theme.clone()).is_equal_to(Some(crate::test_registry::theme_ref("b")));
}

#[test]
fn undo_then_redo_restores_applied_view() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut next = deck;
    next.theme = Some(crate::test_registry::theme_ref("b"));
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
    let mut card = crate::Element::new("carte", base::CARD);
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
    a.apply(with_op(&deck, crate::Op::SetKind { target: card(), kind: base::TEXT.into() }), "A", None).unwrap();
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
    b.apply(with_op(&deck, crate::Op::SetKind { target: card(), kind: base::TEXT.into() }), "texte", None).unwrap();
    b.apply(deck.clone(), "carte", None).unwrap();
    let preview = b.history();
    let issues = b.restore(&preview, 1, "restaurer").unwrap();
    assert_that!(ids_of(&issues, "carte")).is_equal_to(vec!["issue-unknown-param"]);
}

#[test]
fn restore_of_a_version_with_a_missing_asset_returns_it() {
    let deck = roadmap();
    let mut image = crate::Element::new("img", base::IMAGE);
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
    let mut chart = crate::Element::new("g", base::CHART);
    chart.params.set("option", crate::ParamValue::Value(option));
    let slide = deck.slide_mut("s-constat").unwrap();
    slide.params.slot_mut("blocks").unwrap().push(chart);
    crate::canonicalize(&mut deck);
    let mut sources = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    sources.apply(deck, "graphique", None).unwrap();
    let before = sources.deck().to_json().unwrap();
    let mut other = sources.deck().clone();
    other.theme = Some(crate::test_registry::theme_ref("b"));
    sources.apply(other, "thème", None).unwrap();
    sources.undo().unwrap();
    assert_that!(sources.deck().to_json().unwrap()).is_equal_to(before);
}

/// Un deck d'une slide `blank` portant une chaîne de `n` `base/stack` imbriqués.
fn stack_chain(n: usize) -> Deck {
    let mut tree = serde_json::json!({ "id": format!("c{}", n - 1), "kind": "base/stack" });
    for i in (0..n - 1).rev() {
        tree = serde_json::json!({ "id": format!("c{i}"), "kind": "base/stack", "params": { "children": [tree] } });
    }
    let deck = serde_json::json!({ "slides": [{ "id": "s", "kind": "blank", "params": { "elements": [tree] } }] });
    serde_json::from_value(deck).unwrap_or_else(|e| panic!("deck de test : {e}"))
}

#[test]
fn new_with_elements_nested_too_deep_returns_too_deep() {
    let Err(err) = SourceBundle::new(stack_chain(20), crate::test_registry::shared()) else {
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
    let deck = stack_chain(MAX_SLOT_DEPTH);
    let bundle = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    assert_that!(bundle.deck().slides[0].walk().count()).is_equal_to(MAX_SLOT_DEPTH);
}

/// Un deck JSON au tableau `t` et au graphique `g`, sans identifiants de
/// lignes ni de colonnes.
fn deck_without_ids() -> Deck {
    let deck = serde_json::json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "t", "kind": "base/table", "params": { "data": {
            "dataset": { "source": [["Pays", "Ventes"], ["FR", 1], ["DE", 2]] } } } },
        { "id": "g", "kind": "base/chart", "params": { "option": {
            "dataset": { "source": [["Mois", "Ventes"], ["Jan", 10]] }, "series": [{ "type": "bar" }] } } }
    ] } }] });
    Deck::from_json(&deck.to_string()).unwrap()
}

/// Le `dataset` du paramètre `param` de l'élément `id` de la slide `s`.
fn dataset_of(deck: &Deck, id: &str, param: &str) -> serde_json::Value {
    let e = deck.slides[0].locate(id).unwrap().element;
    e.params.value(param).unwrap()["dataset"].clone()
}

fn set_cell(row: usize, col: usize, n: f64) -> crate::Op {
    crate::Op::SetData {
        target: crate::Target { slide: "s".into(), element: Some("t".into()) },
        param: "data".into(),
        change: oku_slides_chart::DataOp::SetCell { row, col, value: oku_slides_chart::Cell::Number(n) },
    }
}

#[test]
fn new_gives_ids_to_the_datasets_of_a_json_deck() {
    let bundle = SourceBundle::new(deck_without_ids(), crate::test_registry::shared()).unwrap();
    for (id, param, rows, cols) in [("t", "data", 3, 2), ("g", "option", 2, 2)] {
        let ds = dataset_of(bundle.deck(), id, param);
        assert_that!(ds["ids"].as_array().map(Vec::len)).is_equal_to(Some(rows));
        assert_that!(ds["columns"].as_array().map(Vec::len)).is_equal_to(Some(cols));
    }
    assert_that!(bundle.replica().deck().0).is_equal_to(bundle.deck());
}

#[test]
fn concurrent_cell_edits_of_a_json_deck_both_survive_the_merge() {
    let zip = SourceBundle::new(deck_without_ids(), crate::test_registry::shared()).unwrap().to_zip().unwrap();
    let (mut a, _) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    let (mut b, _) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    let base = a.deck().clone();
    a.apply(with_op(&base, set_cell(1, 1, 7.0)), "A", None).unwrap();
    b.apply(with_op(&base, set_cell(2, 1, 9.0)), "B", None).unwrap();
    a.merge(b).unwrap();
    let source = dataset_of(a.deck(), "t", "data")["source"].clone();
    assert_that!(source).is_equal_to(serde_json::json!([["Pays", "Ventes"], ["FR", 7.0], ["DE", 9.0]]));
}

/// Un deck au tableau `t`, dont `data` est écrit tel quel (sans identifiants).
fn raw_table_deck() -> Deck {
    let deck = serde_json::json!({ "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "t", "kind": "base/table", "params": { "data": { "dataset": {
            "source": [["Pays", "Ventes"], ["FR", 1]] } } } } ] } }] });
    Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck : {e}"))
}

/// Régression t-631 : restaurer normalise les données d'hôte comme l'aperçu,
/// jusque dans le replica (sinon le jeu est stocké opaque, fusion grossière).
/// En ligne (`sources_tests`) : le stockage n'est lisible que dans la crate.
#[test]
fn restore_of_a_version_without_host_ids_normalizes_it_like_the_preview() {
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    // `apply` n'normalise pas : la version est inscrite telle quelle.
    b.apply(raw_table_deck(), "tableau", None).unwrap();
    b.apply(roadmap(), "retrait", None).unwrap();
    let preview = b.history();
    b.restore(&preview, 1, "restaurer").unwrap();
    // Les identifiants sont tirés au hasard : on compare leur forme, pas leur valeur.
    let dataset = dataset_of(b.deck(), "t", "data");
    assert_that!(dataset["source"]).is_equal_to(serde_json::json!([["Pays", "Ventes"], ["FR", 1.0]]));
    assert_that!(dataset["ids"].as_array().map(Vec::len)).is_equal_to(Some(2));
    assert_that!(dataset["columns"].as_array().map(Vec::len)).is_equal_to(Some(2));
    // Le replica relit exactement la vue : le jeu n'est pas resté opaque.
    assert_that!(b.replica().deck().0).is_equal_to(b.deck().clone());
}

/// Sources v7 : refusées.
#[test]
fn sources_v7_zip_is_refused() {
    let manifest = br#"{"format":"oku-slides-sources","version":7}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

/// Sources v10 : refusées (un seul pack de modèles, dossier `packs/`).
#[test]
fn sources_v10_zip_is_refused() {
    let manifest = br#"{"format":"oku-slides-sources","version":10}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

/// Sources v9 : refusées (marques de suppression non datées, replica au format 3).
#[test]
fn sources_v9_zip_is_refused() {
    let manifest = br#"{"format":"oku-slides-sources","version":9}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

/// Sources v8 : refusées (arbre par slide, replica au format 2).
#[test]
fn sources_v8_zip_is_refused() {
    let manifest = br#"{"format":"oku-slides-sources","version":8}"#;
    let err = SourceBundle::from_zip(
        &zip(&[("manifest.json", manifest), ("deck.loro", &deck_loro())]),
        MAX,
        crate::test_registry::shared(),
    )
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::NotSources);
}

/// Le replica de `roadmap` dont la première slide porte une marque datée de 1 (1970).
fn deck_loro_with_an_old_mark() -> (Vec<u8>, loro::TreeID) {
    let r = DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap();
    let doc = loro::LoroDoc::new();
    doc.import(&r.snapshot().unwrap()).unwrap();
    let Some(loro::ValueOrContainer::Container(loro::Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("arbre du deck attendu")
    };
    let slide = tree.roots()[0];
    tree.get_meta(slide).unwrap().insert("@deleted/7", 1_i64).unwrap();
    doc.commit();
    (doc.export(loro::ExportMode::Snapshot).unwrap(), slide)
}

fn deleted_in(bundle: &SourceBundle, node: loro::TreeID) -> bool {
    let doc = loro::LoroDoc::new();
    doc.import(&bundle.replica().snapshot().unwrap()).unwrap();
    let Some(loro::ValueOrContainer::Container(loro::Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
        panic!("arbre du deck attendu")
    };
    tree.is_node_deleted(&node).unwrap()
}

#[test]
fn from_zip_compacts_a_node_marked_before_the_delay() {
    let (loro, slide) = deck_loro_with_an_old_mark();
    let bytes = zip(&[("manifest.json", MANIFEST), ("deck.loro", &loro)]);
    let limits = SourceLimits { compact_after_secs: 5 * 86_400, ..MAX };
    let (bundle, _) = SourceBundle::from_zip(&bytes, limits, crate::test_registry::shared()).unwrap();
    assert_that!(deleted_in(&bundle, slide)).is_true();
}

#[test]
fn from_zip_with_a_zero_delay_compacts_nothing() {
    let (loro, slide) = deck_loro_with_an_old_mark();
    let bytes = zip(&[("manifest.json", MANIFEST), ("deck.loro", &loro)]);
    let (bundle, _) = SourceBundle::from_zip(&bytes, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(deleted_in(&bundle, slide)).is_false();
}

#[path = "sources_packs_tests.rs"]
mod packs;
#[path = "sources_themes_tests.rs"]
mod themes;
