use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Deck, Element, Op, ParamValue, SlotRef, Target, apply_all};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn registry() -> Arc<WidgetRegistry> {
    crate::test_registry::shared()
}

/// Deux pairs partis du même replica de `deck`.
fn peers(deck: &Deck) -> (DeckReplica, DeckReplica) {
    let snap = DeckReplica::new(deck, registry()).unwrap().snapshot().unwrap();
    let open = || DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0;
    (open(), open())
}

/// `ops` appliquées à une copie de `deck`, sous forme canonique.
fn with_ops(deck: &Deck, ops: Vec<Op>) -> Deck {
    let mut d = deck.clone();
    apply_all(&mut d, ops, &crate::test_registry::registry()).unwrap();
    d
}

fn target(slide: &str, element: &str) -> Target {
    Target { slide: slide.into(), element: Some(element.into()) }
}

fn set_param(slide: &str, element: &str, param: &str, value: serde_json::Value) -> Op {
    Op::SetParam { target: target(slide, element), param: param.into(), value }
}

/// `roadmap` et, dans les blocs de `s-constat`, `e` en plus.
fn roadmap_with_block(e: Element) -> Deck {
    let deck = roadmap();
    let slot = SlotRef { owner: Target { slide: "s-constat".into(), element: None }, param: "blocks".into() };
    with_ops(&deck, vec![Op::AddElement { slot, index: None, element: e }])
}

/// L'élément `id` des blocs de `s-constat`.
fn block<'a>(deck: &'a Deck, id: &str) -> &'a Element {
    deck.slide("s-constat").unwrap().locate(id).unwrap().element
}

/// La carte `params` de l'élément `id`, direct dans les blocs de `s-constat`.
fn block_params(r: &DeckReplica, id: &str) -> loro::LoroMap {
    let as_map = |v: Option<ValueOrContainer>| match v {
        Some(ValueOrContainer::Container(loro::Container::Map(m))) => m,
        other => panic!("carte attendue : {other:?}"),
    };
    let as_list = |v: Option<ValueOrContainer>| match v {
        Some(ValueOrContainer::Container(loro::Container::MovableList(l))) => l,
        other => panic!("liste attendue : {other:?}"),
    };
    let slides = as_list(r.doc.get_map(ROOT).get("slides"));
    let slide = (0..slides.len())
        .map(|i| as_map(slides.get(i)))
        .find(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(v)) if v.as_string().is_some_and(|s| **s == "s-constat")))
        .unwrap();
    let blocks = as_list(as_map(slide.get("params")).get("blocks"));
    let el = (0..blocks.len())
        .map(|i| as_map(blocks.get(i)))
        .find(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(v)) if v.as_string().is_some_and(|s| **s == id)))
        .unwrap();
    as_map(el.get("params"))
}

#[test]
fn new_then_deck_returns_same_deck() {
    let deck = roadmap();
    let r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let (read, issues) = r.deck();
    assert_that!(read).is_equal_to(&deck);
    assert_that!(issues).is_empty();
}

#[test]
fn apply_same_deck_adds_no_op() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let before = r.len_ops();
    r.apply(&deck, "rien").unwrap();
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn snapshot_then_from_snapshot_returns_same_deck_and_uid() {
    let r = DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap();
    let back = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 20, crate::test_registry::shared()).unwrap().0;
    assert_that!(back.deck().0).is_equal_to(r.deck().0);
    assert_that!(back.deck_uid()).is_equal_to(r.deck_uid());
}

#[test]
fn apply_after_op_changes_deck() {
    let mut deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let id = deck.slides[2].id.clone();
    let target = Target { slide: id, element: None };
    let op = Op::SetParam { target, param: "title".into(), value: serde_json::json!("Nouveau") };
    apply_all(&mut deck, vec![op], &crate::test_registry::registry()).unwrap();
    r.apply(&deck, "Titre").unwrap();
    assert_that!(r.deck().0).is_equal_to(&deck);
}

#[test]
fn concurrent_title_edits_merge_both() {
    let deck = roadmap();
    let base = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) = (
        DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0,
        DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0,
    );
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[2].params.set("title", crate::ParamValue::Text("Contexte (révisé)".into()));
    db.slides[2].params.set("title", crate::ParamValue::Text("Brouillon : Contexte".into()));
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    assert_that!(a.deck().0.slides[2].params.text("title")).is_equal_to(Some("Brouillon : Contexte (révisé)"));
}

#[test]
fn concurrent_first_element_on_empty_slide_keeps_both() {
    let deck = roadmap();
    // `s-constat` (`content`) n'a pas d'élément libre : son slot `elements` est vide.
    let empty = deck.slide_index("s-constat").unwrap();
    let mut donor = crate::Element::new("x", "shape");
    donor.frame = Some(crate::Frame { x: 0.0, y: 0.0, w: 10.0, h: 10.0 });
    let (mut a, mut b) = peers(&deck);
    let (mut da, mut db) = (deck.clone(), deck.clone());
    slot_mut(&mut da.slides[empty], "elements").push(crate::Element { id: "e-a".into(), ..donor.clone() });
    slot_mut(&mut db.slides[empty], "elements").push(crate::Element { id: "e-b".into(), ..donor });
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    assert_that!(slot(&a.deck().0.slides[empty], "elements").len()).is_equal_to(2);
}

#[test]
fn concurrent_first_title_on_untitled_slide_keeps_both() {
    let deck = roadmap();
    let untitled = deck.slides.iter().position(|s| s.params.get("title").is_none()).unwrap();
    let base = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) = (
        DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0,
        DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0,
    );
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[untitled].params.set("title", crate::ParamValue::Text("A".into()));
    db.slides[untitled].params.set("title", crate::ParamValue::Text("B".into()));
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    let title = a.deck().0.slides[untitled].params.text("title").unwrap().to_owned();
    assert_that!(title.contains('A') && title.contains('B')).is_true();
}

#[test]
fn blank_title_reads_as_none() {
    let mut deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    deck.slides[2].params.remove("title");
    r.apply(&deck, "Sans titre").unwrap();
    assert_that!(r.deck().0.slides[2].params.get("title")).is_none();
}

fn assert_send<T: Send>() {}

#[test]
fn deck_replica_is_send() {
    assert_send::<DeckReplica>();
}

/// Slide vide (ni blocs ni éléments), pour y placer des éléments de test.
fn empty_slide_index(deck: &Deck) -> usize {
    deck.slides.iter().position(|s| slot(s, "elements").is_empty() && slot(s, "blocks").is_empty()).unwrap()
}

fn any_element(deck: &Deck) -> crate::Element {
    deck.slides.iter().flat_map(|s| slot(s, "elements").iter()).next().unwrap().clone()
}

#[test]
fn duplicate_element_ids_reconcile_and_read_back_both() {
    let mut deck = roadmap();
    let donor = any_element(&deck);
    let idx = empty_slide_index(&deck);
    *slot_mut(&mut deck.slides[idx], "elements") =
        vec![crate::Element { id: "x".into(), ..donor.clone() }, crate::Element { id: "x".into(), ..donor }];
    let r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    assert_that!(slot(&r.deck().0.slides[idx], "elements").len()).is_equal_to(2);
}

#[test]
fn shrinking_from_duplicate_ids_keeps_only_the_remaining_id() {
    let mut deck = roadmap();
    let donor = any_element(&deck);
    let idx = empty_slide_index(&deck);
    *slot_mut(&mut deck.slides[idx], "elements") = vec![
        crate::Element { id: "x".into(), ..donor.clone() },
        crate::Element { id: "x".into(), ..donor.clone() },
        crate::Element { id: "y".into(), ..donor.clone() },
    ];
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    *slot_mut(&mut deck.slides[idx], "elements") = vec![crate::Element { id: "x".into(), ..donor }];
    r.apply(&deck, "réduire").unwrap();
    let got = r.deck().0;
    assert_that!(slot(&got.slides[idx], "elements").len()).is_equal_to(1);
    assert_that!(slot(&got.slides[idx], "elements")[0].id.as_str()).is_equal_to("x");
}

#[test]
fn empty_element_ids_reconcile_without_error() {
    let mut deck = roadmap();
    let donor = any_element(&deck);
    let idx = empty_slide_index(&deck);
    *slot_mut(&mut deck.slides[idx], "elements") =
        vec![crate::Element { id: String::new(), ..donor.clone() }, crate::Element { id: String::new(), ..donor }];
    let r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    assert_that!(slot(&r.deck().0.slides[idx], "elements").len()).is_equal_to(2);
}

#[test]
fn moving_first_slide_to_last_and_last_to_first_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let mut moved = deck.clone();
    let n = moved.slides.len();
    moved.slides.swap(0, n - 1);
    r.apply(&moved, "réordonner").unwrap();
    assert_that!(r.deck().0).is_equal_to(&moved);
}

#[test]
fn deleting_a_slide_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let mut trimmed = deck.clone();
    trimmed.slides.remove(3);
    r.apply(&trimmed, "supprimer une slide").unwrap();
    assert_that!(r.deck().0).is_equal_to(&trimmed);
}

#[test]
fn deleting_an_element_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let mut trimmed = deck.clone();
    let idx = trimmed.slides.iter().position(|s| !slot(s, "elements").is_empty()).unwrap();
    slot_mut(&mut trimmed.slides[idx], "elements").remove(0);
    r.apply(&trimmed, "supprimer un élément").unwrap();
    assert_that!(r.deck().0).is_equal_to(&trimmed);
}

#[test]
fn removing_frame_and_style_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let mut edited = deck.clone();
    let slide = edited.slides.iter_mut().find(|s| s.id == "s-libre").unwrap();
    let el = slot_mut(slide, "elements").iter_mut().find(|e| e.id == "e-fond").unwrap();
    el.frame = None;
    el.style = crate::Style::default();
    r.apply(&edited, "retirer cadre et style").unwrap();
    assert_that!(r.deck().0).is_equal_to(&edited);
}

#[test]
fn changing_element_kind_from_card_to_text_loses_title_and_kicker() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let mut edited = deck.clone();
    let slide = edited.slides.iter_mut().find(|s| s.id == "s-priorites").unwrap();
    let el = slot_mut(slide, "blocks").iter_mut().find(|e| e.id == "c1").unwrap();
    el.kind = "text".into();
    el.params.remove("title");
    el.params.remove("kicker");
    r.apply(&edited, "carte en texte").unwrap();
    assert_that!(r.deck().0).is_equal_to(&edited);
}

#[test]
fn concurrent_first_paragraph_on_empty_text_keeps_both() {
    let mut deck = roadmap();
    let idx = empty_slide_index(&deck);
    *slot_mut(&mut deck.slides[idx], "elements") = vec![crate::Element::text("e-vide", vec![])];
    let base = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) = (
        DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0,
        DeckReplica::from_snapshot(&snap, 1 << 20, registry()).unwrap().0,
    );
    let (mut da, mut db) = (deck.clone(), deck.clone());
    slot_mut(&mut da.slides[idx], "elements")[0] =
        crate::Element::text("e-vide", vec![crate::Paragraph::plain("Alpha")]);
    slot_mut(&mut db.slides[idx], "elements")[0] =
        crate::Element::text("e-vide", vec![crate::Paragraph::plain("Bravo")]);
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0;
    let all: String = slot(&merged.slides[idx], "elements")[0]
        .params
        .rich("body")
        .unwrap()
        .iter()
        .flat_map(|p| p.runs.iter().map(|r| r.t.as_str()))
        .collect();
    assert_that!(all.contains("Alpha") && all.contains("Bravo")).is_true();
}

#[test]
fn apply_same_deck_with_marked_runs_adds_no_op() {
    let mut deck = roadmap();
    let idx = empty_slide_index(&deck);
    let runs = vec![
        crate::Run { t: "gras".into(), bold: true, ..Default::default() },
        crate::Run { t: " et ".into(), ..Default::default() },
        crate::Run {
            t: "lien".into(),
            link: Some("https://oku.test".into()),
            highlight: Some(2),
            ..Default::default()
        },
    ];
    *slot_mut(&mut deck.slides[idx], "elements") =
        vec![crate::Element::text("e-riche", vec![crate::Paragraph { runs, list: Some(crate::ListKind::Number) }])];
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    assert_that!(r.deck().0).is_equal_to(&deck);
    let before = r.len_ops();
    r.apply(&deck, "rien").unwrap();
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn removing_card_text_empties_it_and_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let mut edited = deck.clone();
    let slide = edited.slides.iter_mut().find(|s| s.id == "s-priorites").unwrap();
    let el = slot_mut(slide, "blocks").iter_mut().find(|e| e.id == "c1").unwrap();
    el.params.remove("body");
    r.apply(&edited, "vider la carte").unwrap();
    assert_that!(r.deck().0).is_equal_to(&edited);
    // Vider n'a pas retiré la clé : une seconde réconciliation n'ajoute rien.
    let before = r.len_ops();
    r.apply(&edited, "rien").unwrap();
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn object_valued_var_round_trips_unchanged() {
    let mut deck = roadmap();
    deck.vars.insert("client".into(), serde_json::json!({"name": "Acme", "since": 2020}));
    let r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    assert_that!(r.deck().0.vars).is_equal_to(deck.vars);
}

#[test]
fn array_of_foreign_objects_round_trips_unchanged() {
    // Rien ne se perd : un tableau d'objets qui ne sont pas des paragraphes,
    // sur un paramètre inconnu comme sur un paramètre `rich`, reste tel quel.
    let deck: Deck = serde_json::from_value(json!({ "slides": [{ "id": "s", "kind": "content", "params": {
        "blocks": [{ "id": "t", "kind": "text", "params": { "body": [{ "x": 1 }], "data": [{ "x": 1 }] } }]
    } }] }))
    .unwrap();
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let t = r.deck().0.slides[0].locate("t").unwrap().element.clone();
    assert_that!(t.params.get("body")).is_equal_to(Some(&ParamValue::Value(json!([{ "x": 1 }]))));
    assert_that!(t.params.get("data")).is_equal_to(Some(&ParamValue::Value(json!([{ "x": 1 }]))));
}

#[test]
fn merge_unrelated_returns_unrelated() {
    let mut a = DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap();
    let b = DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap();
    assert_that!(a.merge(&b).unwrap_err().kind()).is_equal_to(crate::ReplicaErrorKind::Unrelated);
}

/// Profondeur d'imbrication d'un JSON (0 pour une feuille).
fn json_depth(v: &serde_json::Value) -> usize {
    match v {
        serde_json::Value::Object(m) => 1 + m.values().map(json_depth).max().unwrap_or(0),
        serde_json::Value::Array(a) => 1 + a.iter().map(json_depth).max().unwrap_or(0),
        _ => 0,
    }
}

/// Profondeur hostile pour une imbrication conteneur par conteneur
/// (`LoroMap` dans `LoroMap`) : Loro décode son propre historique
/// d'opérations sans récursion notable ici, donc `import`/`export`
/// tiennent largement au-delà de `MAX_DEPTH` (64) ; c'est bien notre borne
/// de lecture qui doit arrêter la descente.
const HOSTILE_CONTAINER_DEPTH: usize = 4000;

/// Profondeur hostile pour une valeur plate imbriquée sans conteneur
/// (`LoroValue::Map` dans `LoroValue::Map`, par exemple une entrée de
/// `vars`) : `doc.import` n'y touche pas (import paresseux), mais au
/// premier accès Loro décode la valeur récursivement (postcard + serde), et
/// le drop du doc la libère récursivement ; sur une pile de 256 Kio, l'un
/// ou l'autre déborde vers 1000 niveaux (2000 et 1000 débordent, 700
/// tient). On reste sous ce plafond, très au-delà de `MAX_DEPTH` (64), pour
/// que ce soit notre borne qui décide, pas la pile. Limite côté Loro : t-223.
const HOSTILE_PLAIN_VALUE_DEPTH: usize = 700;

/// Relit un snapshot dans un fil à petite pile (256 Kio) : sans la borne de
/// profondeur, la récursion de lecture déborderait à coup sûr sur un
/// replica imbriqué à `HOSTILE_CONTAINER_DEPTH` ou `HOSTILE_PLAIN_VALUE_DEPTH`
/// niveaux. `std::thread::Result` capture un panic ; un débordement de pile
/// réel abat le processus (SIGSEGV/SIGABRT) et ne peut pas être capturé ici
/// — c'est justement ce que la borne évite.
fn read_tokens_depth_in_small_stack(bytes: Vec<u8>) -> std::thread::Result<usize> {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let r = DeckReplica::from_snapshot(&bytes, 64 << 20, crate::test_registry::shared()).unwrap().0;
            let (deck, _issues) = r.deck();
            json_depth(&deck.tokens)
        })
        .unwrap()
        .join()
}

/// `deck.tokens` imbriqué conteneur par conteneur (`LoroMap` dans
/// `LoroMap`) bien au-delà de la borne : la lecture s'arrête à la borne, et
/// ne déborde pas la pile même très contrainte.
#[test]
fn deck_on_deeply_nested_container_tokens_stops_at_the_depth_bound() {
    let deck = roadmap();
    let r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let root = r.doc.get_map(ROOT);
    let mut current: loro::LoroMap = root.insert_container("tokens", loro::LoroMap::new()).unwrap();
    for _ in 0..HOSTILE_CONTAINER_DEPTH {
        current = current.insert_container("n", loro::LoroMap::new()).unwrap();
    }
    r.doc.commit();
    let bytes = r.snapshot().unwrap();
    let depth = read_tokens_depth_in_small_stack(bytes).expect("la lecture n'a pas dû paniquer ni déborder");
    assert_that!(depth).is_less_than_or_equal_to(super::read::MAX_DEPTH as usize);
}

/// Une valeur `deck.tokens` plate (`LoroValue::Map` imbriquée sans passer
/// par des conteneurs) bien au-delà de la borne : même chose, sans conteneur
/// cette fois (c'est `loro_to_json`, pas `read_map`, qui doit s'arrêter).
#[test]
fn deck_on_deeply_nested_plain_value_tokens_stops_at_the_depth_bound() {
    let deck = roadmap();
    let r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let mut value = loro::LoroValue::Null;
    for _ in 0..HOSTILE_PLAIN_VALUE_DEPTH {
        let mut m = std::collections::HashMap::new();
        m.insert("n".to_string(), value);
        value = loro::LoroValue::from(m);
    }
    r.doc.get_map(ROOT).insert("tokens", value).unwrap();
    r.doc.commit();
    let bytes = r.snapshot().unwrap();
    let depth = read_tokens_depth_in_small_stack(bytes).expect("la lecture n'a pas dû paniquer ni déborder");
    assert_that!(depth).is_less_than_or_equal_to(super::read::MAX_DEPTH as usize);
}

/// Messages des changes Loro, du plus ancien au plus récent.
fn messages(r: &DeckReplica) -> Vec<String> {
    let heads: Vec<loro::ID> = r.doc.oplog_frontiers().iter().collect();
    let mut all = Vec::new();
    let _ = r.doc.travel_change_ancestors(&heads, &mut |m| {
        all.push((m.lamport, m.message.map(|s| s.to_string()).unwrap_or_default()));
        std::ops::ControlFlow::Continue(())
    });
    all.sort();
    all.into_iter().map(|(_, m)| m).collect()
}

#[test]
fn apply_reorder_marks_intermediate_commits_as_parts() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let before = messages(&r).len();
    let mut swapped = deck.clone();
    swapped.slides.swap(0, 3);
    r.apply(&swapped, "Échanger").unwrap();
    let got = messages(&r).split_off(before);
    // Trois `mov` (0↔3 déplace aussi les voisines) : deux commits intermédiaires, puis le dernier.
    assert_that!(got).is_equal_to(vec![format!("{PART}Échanger"), format!("{PART}Échanger"), "Échanger".to_owned()]);
}

fn slot<'a>(s: &'a crate::Slide, name: &str) -> &'a [crate::Element] {
    s.params.slot(name).unwrap_or_default()
}

fn slot_mut<'a>(s: &'a mut crate::Slide, name: &str) -> &'a mut Vec<crate::Element> {
    s.params.slot_mut(name).unwrap()
}

#[test]
fn reconcile_new_card_creates_its_text_params_before_any_value() {
    let r = DeckReplica::new(&roadmap_with_block(Element::new("carte", "card")), registry()).unwrap();
    let params = block_params(&r, "carte");
    for key in ["kicker", "title"] {
        assert_that!(matches!(params.get(key), Some(ValueOrContainer::Container(loro::Container::Text(_))))).is_true();
    }
    let body = match params.get("body") {
        Some(ValueOrContainer::Container(loro::Container::Map(m))) => m,
        other => panic!("carte `rich` attendue : {other:?}"),
    };
    assert_that!(body.keys().map(|k| k.to_string()).collect::<Vec<_>>()).is_equal_to(vec!["rich".to_owned()]);
    assert_that!(matches!(body.get("rich"), Some(ValueOrContainer::Container(loro::Container::Text(_))))).is_true();
    // Rien ne se relit : les paramètres créés d'avance sont vides.
    assert_that!(block(&r.deck().0, "carte").params.is_empty()).is_true();
}

#[test]
fn concurrent_first_edit_of_card_title_merges_both() {
    let deck = roadmap_with_block(Element::new("carte", "card"));
    let (mut a, mut b) = peers(&deck);
    a.apply(&with_ops(&deck, vec![set_param("s-constat", "carte", "title", json!("Alpha"))]), "A").unwrap();
    b.apply(&with_ops(&deck, vec![set_param("s-constat", "carte", "title", json!("Bravo"))]), "B").unwrap();
    a.merge(&b).unwrap();
    let title = block(&a.deck().0, "carte").params.text("title").unwrap().to_owned();
    assert_that!(title.contains("Alpha") && title.contains("Bravo")).is_true();
}

#[test]
fn concurrent_edit_of_title_and_body_of_one_card_keeps_both() {
    let deck = roadmap_with_block(Element::new("carte", "card"));
    let (mut a, mut b) = peers(&deck);
    let body = json!([{ "runs": [{ "t": "Corps" }] }]);
    a.apply(&with_ops(&deck, vec![set_param("s-constat", "carte", "title", json!("Titre"))]), "A").unwrap();
    b.apply(&with_ops(&deck, vec![set_param("s-constat", "carte", "body", body)]), "B").unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0;
    let card = block(&merged, "carte");
    assert_that!(card.params.text("title")).is_equal_to(Some("Titre"));
    assert_that!(card.params.rich("body")).is_equal_to(Some(&[crate::Paragraph::plain("Corps")][..]));
}

#[test]
fn read_rich_param_without_registry_gives_paragraphs() {
    let mut deck = roadmap();
    crate::canonicalize(&mut deck);
    let snap = DeckReplica::new(&deck, registry()).unwrap().snapshot().unwrap();
    let empty = Arc::new(WidgetRegistry::from_probe(json!({ "base": [] })).unwrap());
    let (_, (read, issues)) = DeckReplica::from_snapshot(&snap, 1 << 20, empty).unwrap();
    let card = read.slide("s-priorites").unwrap().locate("c1").unwrap().element;
    assert_that!(card.params.rich("body").map(<[crate::Paragraph]>::len)).is_equal_to(Some(1));
    assert_that!(card.params.text("title")).is_equal_to(Some("Fiabilité"));
    assert_that!(read).is_equal_to(&deck);
    assert_that!(issues).is_empty();
}

#[test]
fn concurrent_set_kind_and_param_edit_keep_the_param() {
    let mut card = Element::new("carte", "card");
    card.params.set("title", ParamValue::Text("Titre".into()));
    let deck = roadmap_with_block(card);
    let (mut a, mut b) = peers(&deck);
    let to_text = Op::SetKind { target: target("s-constat", "carte"), kind: "text".into() };
    a.apply(&with_ops(&deck, vec![to_text]), "A").unwrap();
    b.apply(&with_ops(&deck, vec![set_param("s-constat", "carte", "title", json!("Titre révisé"))]), "B").unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0;
    let el = block(&merged, "carte");
    assert_that!(el.kind.as_str()).is_equal_to("text");
    assert_that!(el.params.text("title")).is_equal_to(Some("Titre révisé"));
    let issues = crate::validate(&merged, &crate::test_registry::registry(), None);
    assert_that!(issues.iter().any(|i| i.message.id() == "issue-unknown-param")).is_true();
}

/// Variante sans titre de départ : le `title` créé d'avance (vide) n'est pas
/// retiré par le changement de widget, la première édition concurrente survit.
#[test]
fn concurrent_set_kind_and_first_param_edit_keep_the_param() {
    let deck = roadmap_with_block(Element::new("carte", "card"));
    let (mut a, mut b) = peers(&deck);
    let to_text = Op::SetKind { target: target("s-constat", "carte"), kind: "text".into() };
    a.apply(&with_ops(&deck, vec![to_text]), "A").unwrap();
    b.apply(&with_ops(&deck, vec![set_param("s-constat", "carte", "title", json!("Titre"))]), "B").unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0;
    assert_that!(block(&merged, "carte").kind.as_str()).is_equal_to("text");
    assert_that!(block(&merged, "carte").params.text("title")).is_equal_to(Some("Titre"));
}

#[test]
fn round_trip_roadmap_is_identical() {
    let mut deck = roadmap();
    crate::canonicalize(&mut deck);
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let (back, issues) = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 20, registry()).unwrap().1;
    assert_that!(back).is_equal_to(&deck);
    assert_that!(issues).is_empty();
}

#[test]
fn concurrent_edit_of_image_fit_keeps_one_whole_value() {
    let mut image = Element::new("img", "image");
    image.params.set("fit", ParamValue::Text("cover".into()));
    let deck = roadmap_with_block(image);
    let (mut a, mut b) = peers(&deck);
    a.apply(&with_ops(&deck, vec![set_param("s-constat", "img", "fit", json!("contain"))]), "A").unwrap();
    b.apply(&with_ops(&deck, vec![set_param("s-constat", "img", "fit", json!("stretch"))]), "B").unwrap();
    a.merge(&b).unwrap();
    b.merge(&a).unwrap();
    let fit = block(&a.deck().0, "img").params.text("fit").unwrap().to_owned();
    assert_that!(["contain", "stretch"].contains(&fit.as_str())).is_true();
    assert_that!(block(&b.deck().0, "img").params.text("fit")).is_equal_to(Some(fit.as_str()));
}

#[test]
fn concurrent_add_element_into_one_slot_keeps_both() {
    let deck = roadmap();
    let (mut a, mut b) = peers(&deck);
    let add = |id: &str| {
        let slot = SlotRef { owner: Target { slide: "s-constat".into(), element: None }, param: "blocks".into() };
        Op::AddElement { slot, index: None, element: Element::text(id, vec![crate::Paragraph::plain(id)]) }
    };
    a.apply(&with_ops(&deck, vec![add("de-a")]), "A").unwrap();
    b.apply(&with_ops(&deck, vec![add("de-b")]), "B").unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0;
    let ids: Vec<&str> = merged.slide("s-constat").unwrap().element_ids().collect();
    assert_that!(ids.contains(&"de-a") && ids.contains(&"de-b")).is_true();
}

#[test]
fn concurrent_add_element_and_set_kind_of_owner_keeps_the_child() {
    let deck = roadmap_with_block(Element::new("pile", "stack"));
    let (mut a, mut b) = peers(&deck);
    let slot = SlotRef { owner: target("s-constat", "pile"), param: "children".into() };
    let add = Op::AddElement { slot, index: None, element: Element::text("enfant", vec![]) };
    a.apply(&with_ops(&deck, vec![add]), "A").unwrap();
    let to_canvas = Op::SetKind { target: target("s-constat", "pile"), kind: "canvas".into() };
    b.apply(&with_ops(&deck, vec![to_canvas]), "B").unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0;
    let pile = block(&merged, "pile");
    assert_that!(pile.kind.as_str()).is_equal_to("canvas");
    let children: Vec<&str> = pile.params.slot("children").unwrap_or_default().iter().map(|e| e.id.as_str()).collect();
    assert_that!(children).is_equal_to(vec!["enfant"]);
}

/// Profondeur hostile en slots : bien au-delà de `MAX_DEPTH` (64), deux
/// niveaux de lecture par slot.
const HOSTILE_SLOT_DEPTH: usize = 200;

/// Plus grande profondeur d'élément (`Located::depth`) d'un deck.
fn max_slot_depth(deck: &Deck) -> Option<usize> {
    deck.slides.iter().flat_map(|s| s.walk()).map(|l| l.depth).max()
}

/// Un replica forgé (sans passer par les ops, qui bornent la profondeur)
/// imbrique 200 `stack` : la lecture s'arrête à la borne des ops
/// (`MAX_SLOT_DEPTH`), la signale, et ne déborde pas la pile même très
/// contrainte.
#[test]
fn read_deeply_nested_slots_stops_at_the_depth_bound() {
    let deck = roadmap_with_block(Element::new("pile-0", "stack"));
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let mut params = block_params(&r, "pile-0");
    for i in 1..HOSTILE_SLOT_DEPTH {
        let children = params.insert_container("children", loro::LoroMovableList::new()).unwrap();
        let child = children.insert_container(0, loro::LoroMap::new()).unwrap();
        child.insert("id", format!("pile-{i}")).unwrap();
        child.insert("kind", "stack").unwrap();
        params = child.insert_container("params", loro::LoroMap::new()).unwrap();
    }
    r.doc.commit();
    let bytes = r.snapshot().unwrap();
    let (depth, issues) = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let (_, (deck, issues)) = DeckReplica::from_snapshot(&bytes, 64 << 20, registry()).unwrap();
            let ids: Vec<&'static str> = issues.iter().map(|i| i.message.id()).collect();
            (max_slot_depth(&deck), ids)
        })
        .unwrap()
        .join()
        .expect("la lecture n'a pas dû paniquer ni déborder");
    assert_that!(depth).is_equal_to(Some(crate::MAX_SLOT_DEPTH - 1));
    assert_that!(issues).is_equal_to(vec!["issue-slot-too-deep"]);
}

/// La profondeur que les ops permettent se relit en entier.
#[test]
fn round_trip_elements_nested_to_the_slot_bound_is_identical() {
    let mut deck = roadmap_with_block(Element::new("pile-0", "stack"));
    for i in 1..crate::MAX_SLOT_DEPTH {
        let slot = SlotRef { owner: target("s-constat", &format!("pile-{}", i - 1)), param: "children".into() };
        deck = with_ops(
            &deck,
            vec![Op::AddElement { slot, index: None, element: Element::new(format!("pile-{i}"), "stack") }],
        );
    }
    let r = DeckReplica::new(&deck, registry()).unwrap();
    let (back, issues) = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 20, registry()).unwrap().1;
    assert_that!(max_slot_depth(&back)).is_equal_to(Some(crate::MAX_SLOT_DEPTH - 1));
    assert_that!(back).is_equal_to(&deck);
    assert_that!(issues).is_empty();
}

/// Un replica dont le deck est d'une version `version`.
fn replica_of_version(version: i64) -> Vec<u8> {
    let r = DeckReplica::new(&roadmap(), registry()).unwrap();
    r.doc.get_map(ROOT).insert("version", version).unwrap();
    r.doc.commit();
    r.snapshot().unwrap()
}

#[test]
fn from_snapshot_of_a_version_1_deck_returns_unsupported_version() {
    let err = DeckReplica::from_snapshot(&replica_of_version(1), 1 << 20, registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn merge_of_a_version_1_deck_returns_unsupported_version_and_keeps_self() {
    let (mut a, b) = peers(&roadmap());
    b.doc.get_map(ROOT).insert("version", 1).unwrap();
    b.doc.commit();
    let err = a.merge(&b).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
    assert_that!(a.deck().1).is_empty();
}

#[test]
fn new_with_a_version_1_deck_returns_unsupported_version() {
    let mut deck = roadmap();
    deck.version = 1;
    let err = DeckReplica::new(&deck, registry()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::ReplicaErrorKind::UnsupportedVersion);
}

#[test]
fn unsupported_version_error_localizes_with_its_own_message() {
    let err = DeckReplica::from_snapshot(&replica_of_version(1), 1 << 20, registry()).unwrap_err();
    let text = oku_slides_i18n::Localize::localize(&err, oku_slides_i18n::english());
    assert_that!(text).contains("deck format");
}
