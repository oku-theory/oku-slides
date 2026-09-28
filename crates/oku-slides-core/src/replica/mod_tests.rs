use speculoos::prelude::*;

use super::*;
use crate::{Deck, Op, SlideField, apply_all};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

#[test]
fn new_then_deck_returns_same_deck() {
    let deck = roadmap();
    let r = DeckReplica::new(&deck).unwrap();
    let (read, issues) = r.deck();
    assert_that!(read).is_equal_to(&deck);
    assert_that!(issues).is_empty();
}

#[test]
fn apply_same_deck_adds_no_op() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let before = r.len_ops();
    r.apply(&deck, "rien").unwrap();
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn snapshot_then_from_snapshot_returns_same_deck_and_uid() {
    let r = DeckReplica::new(&roadmap()).unwrap();
    let back = DeckReplica::from_snapshot(&r.snapshot().unwrap(), 1 << 20).unwrap().0;
    assert_that!(back.deck().0).is_equal_to(r.deck().0);
    assert_that!(back.deck_uid()).is_equal_to(r.deck_uid());
}

#[test]
fn apply_after_op_changes_deck() {
    let mut deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let id = deck.slides[2].id.clone();
    apply_all(
        &mut deck,
        vec![Op::SetSlideField { slide: id, field: SlideField::Title, value: serde_json::json!("Nouveau") }],
    )
    .unwrap();
    r.apply(&deck, "Titre").unwrap();
    assert_that!(r.deck().0).is_equal_to(&deck);
}

#[test]
fn concurrent_title_edits_merge_both() {
    let deck = roadmap();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) =
        (DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0, DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0);
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[2].title = Some("Contexte (révisé)".into());
    db.slides[2].title = Some("Brouillon : Contexte".into());
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    assert_that!(a.deck().0.slides[2].title.as_deref()).is_equal_to(Some("Brouillon : Contexte (révisé)"));
}

#[test]
fn concurrent_first_element_on_empty_slide_keeps_both() {
    let mut deck = roadmap();
    let empty = deck.slides.iter().position(|s| s.elements.is_empty() && s.blocks.is_empty()).unwrap();
    let donor = deck.slides.iter().flat_map(|s| s.elements.iter()).next().unwrap().clone();
    deck.slides[empty].elements.clear();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) =
        (DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0, DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0);
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[empty].elements.push(crate::Element { id: "e-a".into(), ..donor.clone() });
    db.slides[empty].elements.push(crate::Element { id: "e-b".into(), ..donor });
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    assert_that!(a.deck().0.slides[empty].elements).has_length(2);
}

#[test]
fn concurrent_first_title_on_untitled_slide_keeps_both() {
    let deck = roadmap();
    let untitled = deck.slides.iter().position(|s| s.title.is_none()).unwrap();
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) =
        (DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0, DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0);
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[untitled].title = Some("A".into());
    db.slides[untitled].title = Some("B".into());
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    let title = a.deck().0.slides[untitled].title.clone().unwrap();
    assert_that!(title.contains('A') && title.contains('B')).is_true();
}

#[test]
fn blank_title_reads_as_none() {
    let mut deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    deck.slides[2].title = None;
    r.apply(&deck, "Sans titre").unwrap();
    assert_that!(r.deck().0.slides[2].title).is_none();
}

fn assert_send<T: Send>() {}

#[test]
fn deck_replica_is_send() {
    assert_send::<DeckReplica>();
}

/// Slide vide (ni blocs ni éléments), pour y placer des éléments de test.
fn empty_slide_index(deck: &Deck) -> usize {
    deck.slides.iter().position(|s| s.elements.is_empty() && s.blocks.is_empty()).unwrap()
}

fn any_element(deck: &Deck) -> crate::Element {
    deck.slides.iter().flat_map(|s| s.elements.iter()).next().unwrap().clone()
}

#[test]
fn duplicate_element_ids_reconcile_and_read_back_both() {
    let mut deck = roadmap();
    let donor = any_element(&deck);
    let idx = empty_slide_index(&deck);
    deck.slides[idx].elements =
        vec![crate::Element { id: "x".into(), ..donor.clone() }, crate::Element { id: "x".into(), ..donor }];
    let r = DeckReplica::new(&deck).unwrap();
    assert_that!(r.deck().0.slides[idx].elements).has_length(2);
}

#[test]
fn shrinking_from_duplicate_ids_keeps_only_the_remaining_id() {
    let mut deck = roadmap();
    let donor = any_element(&deck);
    let idx = empty_slide_index(&deck);
    deck.slides[idx].elements = vec![
        crate::Element { id: "x".into(), ..donor.clone() },
        crate::Element { id: "x".into(), ..donor.clone() },
        crate::Element { id: "y".into(), ..donor.clone() },
    ];
    let mut r = DeckReplica::new(&deck).unwrap();
    deck.slides[idx].elements = vec![crate::Element { id: "x".into(), ..donor }];
    r.apply(&deck, "réduire").unwrap();
    let got = r.deck().0;
    assert_that!(got.slides[idx].elements).has_length(1);
    assert_that!(got.slides[idx].elements[0].id.as_str()).is_equal_to("x");
}

#[test]
fn empty_element_ids_reconcile_without_error() {
    let mut deck = roadmap();
    let donor = any_element(&deck);
    let idx = empty_slide_index(&deck);
    deck.slides[idx].elements =
        vec![crate::Element { id: String::new(), ..donor.clone() }, crate::Element { id: String::new(), ..donor }];
    let r = DeckReplica::new(&deck).unwrap();
    assert_that!(r.deck().0.slides[idx].elements).has_length(2);
}

#[test]
fn moving_first_slide_to_last_and_last_to_first_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let mut moved = deck.clone();
    let n = moved.slides.len();
    moved.slides.swap(0, n - 1);
    r.apply(&moved, "réordonner").unwrap();
    assert_that!(r.deck().0).is_equal_to(&moved);
}

#[test]
fn deleting_a_slide_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let mut trimmed = deck.clone();
    trimmed.slides.remove(3);
    r.apply(&trimmed, "supprimer une slide").unwrap();
    assert_that!(r.deck().0).is_equal_to(&trimmed);
}

#[test]
fn deleting_an_element_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let mut trimmed = deck.clone();
    let idx = trimmed.slides.iter().position(|s| !s.elements.is_empty()).unwrap();
    trimmed.slides[idx].elements.remove(0);
    r.apply(&trimmed, "supprimer un élément").unwrap();
    assert_that!(r.deck().0).is_equal_to(&trimmed);
}

#[test]
fn removing_frame_and_style_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let mut edited = deck.clone();
    let slide = edited.slides.iter_mut().find(|s| s.id == "s-libre").unwrap();
    let el = slide.elements.iter_mut().find(|e| e.id == "e-fond").unwrap();
    el.frame = None;
    el.style = crate::Style::default();
    r.apply(&edited, "retirer cadre et style").unwrap();
    assert_that!(r.deck().0).is_equal_to(&edited);
}

#[test]
fn changing_element_kind_from_card_to_text_loses_title_and_kicker() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let mut edited = deck.clone();
    let slide = edited.slides.iter_mut().find(|s| s.id == "s-priorites").unwrap();
    let el = slide.blocks.iter_mut().find(|e| e.id == "c1").unwrap();
    let paragraphs = el.body.text().unwrap().to_vec();
    el.body = crate::ElementBody::Text { text: paragraphs };
    r.apply(&edited, "carte en texte").unwrap();
    assert_that!(r.deck().0).is_equal_to(&edited);
}

#[test]
fn concurrent_first_paragraph_on_empty_text_keeps_both() {
    let mut deck = roadmap();
    let idx = empty_slide_index(&deck);
    deck.slides[idx].elements = vec![crate::Element::text("e-vide", vec![])];
    let base = DeckReplica::new(&deck).unwrap();
    let snap = base.snapshot().unwrap();
    let (mut a, mut b) =
        (DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0, DeckReplica::from_snapshot(&snap, 1 << 20).unwrap().0);
    let (mut da, mut db) = (deck.clone(), deck.clone());
    da.slides[idx].elements[0] = crate::Element::text("e-vide", vec![crate::Paragraph::plain("Alpha")]);
    db.slides[idx].elements[0] = crate::Element::text("e-vide", vec![crate::Paragraph::plain("Bravo")]);
    a.apply(&da, "A").unwrap();
    b.apply(&db, "B").unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0;
    let all: String = merged.slides[idx].elements[0]
        .body
        .text()
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
    deck.slides[idx].elements =
        vec![crate::Element::text("e-riche", vec![crate::Paragraph { runs, list: Some(crate::ListKind::Number) }])];
    let mut r = DeckReplica::new(&deck).unwrap();
    assert_that!(r.deck().0).is_equal_to(&deck);
    let before = r.len_ops();
    r.apply(&deck, "rien").unwrap();
    assert_that!(r.len_ops()).is_equal_to(before);
}

#[test]
fn removing_card_text_empties_it_and_round_trips() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck).unwrap();
    let mut edited = deck.clone();
    let slide = edited.slides.iter_mut().find(|s| s.id == "s-priorites").unwrap();
    let el = slide.blocks.iter_mut().find(|e| e.id == "c1").unwrap();
    if let crate::ElementBody::Card { text, .. } = &mut el.body {
        text.clear();
    }
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
    let r = DeckReplica::new(&deck).unwrap();
    assert_that!(r.deck().0.vars).is_equal_to(deck.vars);
}

#[test]
fn merge_unrelated_returns_unrelated() {
    let mut a = DeckReplica::new(&roadmap()).unwrap();
    let b = DeckReplica::new(&roadmap()).unwrap();
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
            let r = DeckReplica::from_snapshot(&bytes, 64 << 20).unwrap().0;
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
    let r = DeckReplica::new(&deck).unwrap();
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
    let r = DeckReplica::new(&deck).unwrap();
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
    let mut r = DeckReplica::new(&deck).unwrap();
    let before = messages(&r).len();
    let mut swapped = deck.clone();
    swapped.slides.swap(0, 3);
    r.apply(&swapped, "Échanger").unwrap();
    let got = messages(&r).split_off(before);
    // Trois `mov` (0↔3 déplace aussi les voisines) : deux commits intermédiaires, puis le dernier.
    assert_that!(got).is_equal_to(vec![format!("{PART}Échanger"), format!("{PART}Échanger"), "Échanger".to_owned()]);
}
