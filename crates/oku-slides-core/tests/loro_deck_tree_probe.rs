//! Sonde de caractérisation de loro 1.16.2 pour t-746 et t-747 : (a) une
//! suppression douce (une marque `@deleted/<pair>` par pair dans la méta, jamais
//! `tree.delete`) et
//! (b) un seul `LoroTree` pour tout le deck (slides en racine, éléments en
//! descendants, `@slot` indexé par le `TreeID` du parent). Ces tests fixent le
//! comportement observé de la bibliothèque, pas celui du produit. Réglages
//! repris de `loro_tree_probe.rs` (ADR-0005).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
#![expect(clippy::many_single_char_names, reason = "probe: peers, trees and nodes named as in the described scenario")]

use std::time::Instant;

use loro::{
    Container, ExportMode, LoroDoc, LoroMap, LoroText, LoroTree, LoroValue, TreeID, TreeParentId, UndoManager,
    ValueOrContainer,
};
use speculoos::prelude::*;

const TREE: &str = "deck";
const DELETED: &str = "@deleted";
const SLOT: &str = "@slot";

/// `unwrap` hors des fonctions de test, refusé par clippy.
trait Must<T> {
    fn must(self) -> T;
}

impl<T, E: std::fmt::Display> Must<T> for Result<T, E> {
    fn must(self) -> T {
        self.unwrap_or_else(|e| panic!("loro : {e}"))
    }
}

/// Un pair déterministe, réglé comme `replica::configure` (sans horodatage).
fn peer(id: u64) -> LoroDoc {
    let doc = LoroDoc::new();
    doc.set_peer_id(id).must();
    doc.set_record_timestamp(false);
    doc.set_change_merge_interval(-1);
    doc
}

/// Un second pair qui part de l'état de `a`.
fn fork(a: &LoroDoc, id: u64) -> LoroDoc {
    let b = peer(id);
    b.import(&a.export(ExportMode::Snapshot).must()).must();
    b
}

/// Import croisé de tout ce que chaque pair a.
fn sync(a: &LoroDoc, b: &LoroDoc) {
    a.import(&b.export(ExportMode::all_updates()).must()).must();
    b.import(&a.export(ExportMode::all_updates()).must()).must();
}

fn tree(doc: &LoroDoc) -> LoroTree {
    let t = doc.get_tree(TREE);
    t.enable_fractional_index(0);
    t
}

/// Un `UndoManager` réglé comme `replica::undo`.
fn undo_manager(doc: &LoroDoc) -> UndoManager {
    let mut m = UndoManager::new(doc);
    m.set_merge_interval(0);
    m
}

fn same(a: &LoroDoc, b: &LoroDoc) {
    assert_that!(a.get_deep_value()).is_equal_to(b.get_deep_value());
}

fn meta_str(t: &LoroTree, id: TreeID, key: &str) -> Option<String> {
    match t.get_meta(id).must().get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

fn meta_text(t: &LoroTree, id: TreeID) -> String {
    match t.get_meta(id).must().get("text") {
        Some(ValueOrContainer::Container(Container::Text(x))) => x.to_string(),
        other => panic!("pas de LoroText sous `text` : {other:?}"),
    }
}

fn edit_text(t: &LoroTree, id: TreeID, pos: usize, s: &str) {
    match t.get_meta(id).must().get("text") {
        Some(ValueOrContainer::Container(Container::Text(x))) => x.insert(pos, s).must(),
        other => panic!("pas de LoroText sous `text` : {other:?}"),
    }
}

fn slot_map(t: &LoroTree, x: TreeID) -> LoroMap {
    t.get_meta(x).must().ensure_mergeable_map(SLOT).must()
}

/// `@slot[parent] = slot`, la clé étant le `TreeID` du parent en chaîne.
fn set_slot(t: &LoroTree, x: TreeID, parent: TreeID, slot: &str) {
    slot_map(t, x).insert(&parent.to_string(), slot).must();
}

fn slot_of(t: &LoroTree, x: TreeID, parent: TreeID) -> Option<String> {
    match slot_map(t, x).get(&parent.to_string()) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

/// Élément à méta `kind`, `@slot` et texte `text`, sous `parent`.
fn element(t: &LoroTree, parent: TreeID, slot: &str, text: &str) -> TreeID {
    let x = t.create(parent).must();
    let meta = t.get_meta(x).must();
    meta.insert("kind", "text").must();
    meta.insert_container("text", LoroText::new()).must().insert(0, text).must();
    set_slot(t, x, parent, slot);
    x
}

fn mark(t: &LoroTree, x: TreeID) {
    t.get_meta(x).must().insert(DELETED, true).must();
}

fn unmark(t: &LoroTree, x: TreeID) {
    t.get_meta(x).must().delete(DELETED).must();
}

fn marked(t: &LoroTree, x: TreeID) -> bool {
    matches!(t.get_meta(x).must().get(DELETED), Some(ValueOrContainer::Value(LoroValue::Bool(true))))
}

/// Visible : ni x ni aucun de ses ancêtres n'est marqué (ni supprimé pour de bon).
fn visible(t: &LoroTree, x: TreeID) -> bool {
    let mut cur = TreeParentId::Node(x);
    let mut steps = 0;
    while let TreeParentId::Node(n) = cur {
        steps += 1;
        assert!(steps <= t.nodes().len() + 1, "cycle autour de {x:?}");
        if marked(t, n) || t.is_node_deleted(&n).must() {
            return false;
        }
        cur = t.parent(n).unwrap_or_else(|| panic!("{n:?} absent"));
    }
    true
}

fn kids(t: &LoroTree, p: impl Into<TreeParentId>) -> Vec<TreeID> {
    t.children(p).unwrap_or_default()
}

fn roots(t: &LoroTree) -> Vec<TreeID> {
    kids(t, TreeParentId::Root)
}

/// Lecture du deck telle que la ferait l'écriture : slides visibles dans
/// l'ordre, et pour chacune ses éléments visibles dans l'ordre, méta comprise.
type Shape = Vec<(TreeID, LoroValue, Vec<(TreeID, LoroValue)>)>;

fn read(t: &LoroTree) -> Shape {
    roots(t)
        .into_iter()
        .filter(|s| !marked(t, *s))
        .map(|s| {
            let elems = kids(t, s)
                .into_iter()
                .filter(|x| !marked(t, *x))
                .map(|x| (x, t.get_meta(x).must().get_deep_value()))
                .collect();
            (s, t.get_meta(s).must().get_deep_value(), elems)
        })
        .collect()
}

/// Deck minimal : deux slides racine, x sous s1, p (conteneur) sous s1 et q sous s2.
struct Fixture {
    a: LoroDoc,
    t: LoroTree,
    s1: TreeID,
    s2: TreeID,
    x: TreeID,
    p: TreeID,
}

fn fixture() -> Fixture {
    let a = peer(1);
    let t = tree(&a);
    let s1 = t.create(None).must();
    let s2 = t.create(None).must();
    let x = element(&t, s1, "a", "hello");
    let p = element(&t, s1, "a", "group");
    element(&t, s2, "a", "q");
    a.commit();
    Fixture { a, t, s1, s2, x, p }
}

/// Point 1: A marks x deleted, then undoes. B, without receiving anything, moves x
/// to s2 and edits its meta. After exchange, in both peer orders:
/// same `TreeID`, no more mark, B's parent, B's edit.
#[test]
fn p1_soft_delete_undone_keeps_concurrent_move_and_edit() {
    for bid in [2u64, 0] {
        let Fixture { a, t, s2, x, .. } = fixture();
        let b = fork(&a, bid);
        let tb = tree(&b);
        let mut m = undo_manager(&a);

        mark(&t, x);
        a.commit();
        assert_that!(visible(&t, x)).is_false();
        assert_that!(m.undo().must()).is_true();
        a.commit();
        tb.mov(x, s2).must();
        set_slot(&tb, x, s2, "b");
        tb.get_meta(x).must().insert("kind", "title").must();
        edit_text(&tb, x, 5, " world");
        b.commit();
        sync(&a, &b);

        same(&a, &b);
        for t in [&t, &tb] {
            assert_that!(t.nodes().len()).named(&format!("pair B = {bid}")).is_equal_to(5);
            assert_that!(t.is_node_deleted(&x).must()).is_false();
            assert_that!(marked(t, x)).is_false();
            assert_that!(visible(t, x)).is_true();
            assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(s2)));
            assert_that!(meta_str(t, x, "kind")).is_equal_to(Some("title".to_owned()));
            assert_that!(meta_text(t, x)).is_equal_to("hello world".to_owned());
            assert_that!(slot_of(t, x, s2)).is_equal_to(Some("b".to_owned()));
        }
    }
}

/// Point 1b : A reçoit le `mov` et l'édition de B ENTRE sa marque et son
/// annulation. L'annulation ne retire que la clé : x reste sous s2 (contraste
/// avec le point 5c de la sonde t-694, où défaire `tree.delete` ramenait x).
#[test]
fn p1_undo_soft_delete_after_receiving_move_does_not_move_back() {
    for bid in [2u64, 0] {
        let Fixture { a, t, s2, x, .. } = fixture();
        let b = fork(&a, bid);
        let tb = tree(&b);
        let mut m = undo_manager(&a);

        mark(&t, x);
        a.commit();
        tb.mov(x, s2).must();
        tb.get_meta(x).must().insert("kind", "title").must();
        b.commit();
        sync(&a, &b);
        assert_that!(visible(&t, x)).is_false();
        assert_that!(m.undo().must()).is_true();
        a.commit();
        sync(&a, &b);

        same(&a, &b);
        assert_that!(visible(&t, x)).is_true();
        assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(s2)));
        assert_that!(meta_str(&t, x, "kind")).is_equal_to(Some("title".to_owned()));
    }
}

/// Point 1c : A et B marquent x en parallèle, puis A annule (avant ou après
/// avoir reçu la marque de B). Rapporte si la suppression de B survit.
#[test]
fn p1_concurrent_double_soft_delete_then_one_undo() {
    let mut seen = Vec::new();
    for bid in [2u64, 0] {
        for received_first in [false, true] {
            let Fixture { a, t, x, .. } = fixture();
            let b = fork(&a, bid);
            let tb = tree(&b);
            let mut m = undo_manager(&a);

            mark(&t, x);
            a.commit();
            mark(&tb, x);
            b.commit();
            if received_first {
                sync(&a, &b);
            }
            assert_that!(m.undo().must()).is_true();
            a.commit();
            sync(&a, &b);
            same(&a, &b);
            seen.push((bid, received_first, marked(&t, x)));
        }
    }
    println!("double suppression, une annulation (pair B, reçue avant, x marqué) : {seen:?}");
    // L'annulation de A est une suppression de clé plus récente que la marque
    // de B : elle l'écrase, dans tous les cas. La suppression de B est perdue.
    assert_that!(seen).is_equal_to(vec![(2, false, false), (2, true, false), (0, false, false), (0, true, false)]);
}

/// Point 1d : Parade au point 1c. Une marque par pair, clé `@deleted/<pair>` à
/// plat dans la méta ; x est supprimé tant qu'il en reste une. L'annulation de
/// A ne retire que sa clé : la marque de B survit dans tous les cas.
#[test]
fn p1_per_peer_mark_keys_survive_a_concurrent_undo() {
    let peer_key = |doc: &LoroDoc| format!("{DELETED}/{}", doc.peer_id());
    let any_mark = |t: &LoroTree, x: TreeID| t.get_meta(x).must().keys().any(|k| k.starts_with(DELETED));
    for bid in [2u64, 0] {
        for received_first in [false, true] {
            let Fixture { a, t, x, .. } = fixture();
            let b = fork(&a, bid);
            let tb = tree(&b);
            let mut m = undo_manager(&a);

            t.get_meta(x).must().insert(&peer_key(&a), true).must();
            a.commit();
            tb.get_meta(x).must().insert(&peer_key(&b), true).must();
            b.commit();
            if received_first {
                sync(&a, &b);
            }
            assert_that!(m.undo().must()).is_true();
            a.commit();
            sync(&a, &b);
            same(&a, &b);
            assert_that!(any_mark(&t, x)).named(&format!("pair B = {bid}, reçue avant = {received_first}")).is_true();
            assert_that!(t.get_meta(x).must().get(&peer_key(&a))).is_none();
        }
    }
}

/// Point 2 : B reçoit la marque et l'annulation, puis déplace et édite x.
#[test]
fn p2_peer_acting_after_receiving_undo_keeps_its_move_and_edit() {
    for bid in [2u64, 0] {
        let Fixture { a, t, s2, x, .. } = fixture();
        let b = fork(&a, bid);
        let tb = tree(&b);
        let mut m = undo_manager(&a);

        mark(&t, x);
        a.commit();
        assert_that!(m.undo().must()).is_true();
        a.commit();
        sync(&a, &b);
        tb.mov(x, s2).must();
        tb.get_meta(x).must().insert("kind", "title").must();
        edit_text(&tb, x, 0, ">");
        b.commit();
        sync(&a, &b);

        same(&a, &b);
        for t in [&t, &tb] {
            assert_that!(t.nodes().len()).is_equal_to(5);
            assert_that!(visible(t, x)).is_true();
            assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(s2)));
            assert_that!(meta_str(t, x, "kind")).is_equal_to(Some("title".to_owned()));
            assert_that!(meta_text(t, x)).is_equal_to(">hello".to_owned());
        }
    }
}

/// Point 3 : A marque le parent p, B déplace x sous p en parallèle : x est
/// caché avec p. A annule : x réapparaît sous p, dans les deux ordres.
#[test]
fn p3_move_under_soft_deleted_parent_reappears_on_undo() {
    for bid in [2u64, 0] {
        let Fixture { a, t, x, p, .. } = fixture();
        let b = fork(&a, bid);
        let tb = tree(&b);
        let mut m = undo_manager(&a);

        mark(&t, p);
        a.commit();
        tb.mov(x, p).must();
        b.commit();
        sync(&a, &b);
        same(&a, &b);
        assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(p)));
        assert_that!(visible(&t, x)).is_false();
        assert_that!(marked(&t, x)).is_false();

        assert_that!(m.undo().must()).is_true();
        a.commit();
        sync(&a, &b);

        same(&a, &b);
        for t in [&t, &tb] {
            assert_that!(visible(t, p)).is_true();
            assert_that!(visible(t, x)).is_true();
            assert_that!(kids(t, p)).is_equal_to(vec![x]);
        }
    }
}

/// Point 4a : Rétablir après annuler remet la marque ; défaire encore la retire.
#[test]
fn p4_redo_restores_the_mark() {
    let Fixture { a, t, x, .. } = fixture();
    let mut m = undo_manager(&a);
    mark(&t, x);
    a.commit();
    assert_that!(m.undo().must()).is_true();
    assert_that!(marked(&t, x)).is_false();
    assert_that!(m.redo().must()).is_true();
    assert_that!(marked(&t, x)).is_true();
    assert_that!(m.undo().must()).is_true();
    assert_that!(marked(&t, x)).is_false();
    assert_that!(t.get_meta(x).must().get(DELETED)).is_none();
}

/// Point 4b : Plusieurs marques et démarques dans un même commit se défont et
/// se refont exactement.
#[test]
fn p4_many_marks_and_unmarks_in_one_commit_undo_exactly() {
    let a = peer(1);
    let t = tree(&a);
    let s = t.create(None).must();
    let c: Vec<TreeID> = (0..6).map(|i| element(&t, s, "a", &format!("e{i}"))).collect();
    mark(&t, c[4]);
    mark(&t, c[5]);
    a.commit();
    let initial = a.get_deep_value();
    let mut m = undo_manager(&a);

    mark(&t, c[0]);
    mark(&t, c[1]);
    unmark(&t, c[4]);
    mark(&t, c[2]);
    unmark(&t, c[2]);
    unmark(&t, c[5]);
    mark(&t, c[5]);
    a.commit();
    let done = a.get_deep_value();
    let flags = |t: &LoroTree| c.iter().map(|x| marked(t, *x)).collect::<Vec<_>>();
    assert_that!(flags(&t)).is_equal_to(vec![true, true, false, false, false, true]);

    assert_that!(m.undo().must()).is_true();
    assert_that!(flags(&t)).is_equal_to(vec![false, false, false, false, true, true]);
    assert_that!(a.get_deep_value()).is_equal_to(&initial);
    assert_that!(m.redo().must()).is_true();
    assert_that!(a.get_deep_value()).is_equal_to(done);
}

/// Point 4c : Marque et `mov` de x dans le même commit : défaire remet x à sa
/// place et sans marque, rétablir refait les deux.
#[test]
fn p4_mark_and_move_in_one_commit_undo_redo() {
    let Fixture { a, t, s1, s2, x, p } = fixture();
    let initial = a.get_deep_value();
    let mut m = undo_manager(&a);

    t.mov_to(x, s2, 0).must();
    mark(&t, x);
    a.commit();
    let done = a.get_deep_value();

    assert_that!(m.undo().must()).is_true();
    assert_that!(a.get_deep_value()).is_equal_to(&initial);
    assert_that!(kids(&t, s1)).is_equal_to(vec![x, p]);
    assert_that!(marked(&t, x)).is_false();
    assert_that!(m.redo().must()).is_true();
    assert_that!(a.get_deep_value()).is_equal_to(&done);
    assert_that!(kids(&t, s2)[0]).is_equal_to(x);
    assert_that!(marked(&t, x)).is_true();
}

struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).must() % n
    }
}

/// Un commit aléatoire de 2 à 4 `mov` : une slide déplacée parmi les racines,
/// ou un élément déplacé (même slide ou autre) avec `@slot` mis à jour.
fn random_commit(t: &LoroTree, rng: &mut Lcg, elems: &[TreeID]) -> usize {
    let n = 2 + rng.below(3);
    for _ in 0..n {
        if rng.below(3) == 0 {
            let slides = roots(t);
            let s = slides[rng.below(slides.len())];
            t.mov_to(s, None, rng.below(slides.len())).must();
        } else {
            let x = elems[rng.below(elems.len())];
            let slides = roots(t);
            let to = slides[rng.below(slides.len())];
            let same_parent = t.parent(x) == Some(TreeParentId::Node(to));
            let len = kids(t, to).len();
            let k = rng.below(if same_parent { len } else { len + 1 });
            t.mov_to(x, to, k).must();
            set_slot(t, x, to, if rng.below(2) == 0 { "a" } else { "b" });
        }
    }
    n
}

/// Point 5 : Arbre de deck, analogue de t-263. 4 slides de 3 éléments, 20
/// commits aléatoires de 2 à 4 `mov` (40 à 80 `mov`), sous 4 graines. Tout
/// défaire puis tout rétablir, en exigeant l'état exact à chaque étape.
#[test]
fn p5_random_deck_moves_undo_and_redo_exactly() {
    for seed in [1_u64, 7, 42, 2026] {
        let a = peer(1);
        let t = tree(&a);
        let mut elems = Vec::new();
        for i in 0..4 {
            let s = t.create(None).must();
            t.get_meta(s).must().insert("name", format!("s{i}")).must();
            for j in 0..3 {
                elems.push(element(&t, s, "a", &format!("e{i}{j}")));
            }
        }
        a.commit();
        let mut m = undo_manager(&a);
        let mut rng = Lcg(seed);
        let mut states = vec![read(&t)];
        let mut moves = 0;
        for _ in 0..20 {
            moves += random_commit(&t, &mut rng, &elems);
            a.commit();
            states.push(read(&t));
        }
        assert_that!(moves).is_greater_than_or_equal_to(40);

        for (k, want) in states.iter().enumerate().rev().skip(1) {
            assert_that!(m.undo().must()).is_true();
            assert_that!(read(&t)).named(&format!("graine {seed}, annuler vers {k}")).is_equal_to(want);
        }
        assert_that!(m.can_undo()).is_false();
        for (k, want) in states.iter().enumerate().skip(1) {
            assert_that!(m.redo().must()).is_true();
            assert_that!(read(&t)).named(&format!("graine {seed}, rétablir vers {k}")).is_equal_to(want);
        }
        assert_that!(t.nodes().len()).is_equal_to(16);
        assert_that!(t.get_nodes(false).len()).is_equal_to(16);
    }
}

/// Point 6 : A déplace x d'une slide (racine) à une autre, B édite la méta de x
/// en parallèle : édition gardée, x dans la nouvelle slide, dans les deux ordres.
#[test]
fn p6_move_between_slides_keeps_concurrent_meta_edit() {
    for bid in [2u64, 0] {
        let Fixture { a, t, s1, s2, x, .. } = fixture();
        let b = fork(&a, bid);
        let tb = tree(&b);

        t.mov_to(x, s2, 0).must();
        set_slot(&t, x, s2, "b");
        a.commit();
        tb.get_meta(x).must().insert("kind", "title").must();
        edit_text(&tb, x, 5, "!");
        b.commit();
        sync(&a, &b);

        same(&a, &b);
        for t in [&t, &tb] {
            assert_that!(t.parent(x)).is_equal_to(Some(TreeParentId::Node(s2)));
            assert_that!(kids(t, s2)[0]).is_equal_to(x);
            assert_that!(meta_str(t, x, "kind")).is_equal_to(Some("title".to_owned()));
            assert_that!(meta_text(t, x)).is_equal_to("hello!".to_owned());
            assert_that!(slot_of(t, x, s2)).is_equal_to(Some("b".to_owned()));
            assert_that!(slot_of(t, x, s1)).is_equal_to(Some("a".to_owned()));
        }
    }
}

/// Point 7 : Cycle entre slides : A met s1 sous un élément de s2, B met s2 sous un
/// élément de s1. Rien ne panique, les pairs convergent, sans cycle : le
/// premier `mov` dans l'ordre (lamport, pair) gagne, l'autre est ignoré.
#[test]
fn p7_cross_slide_cycle_converges_without_panic() {
    for bid in [2u64, 0] {
        let Fixture { a, t, s1, s2, x, .. } = fixture();
        let e2 = kids(&t, s2)[0];
        let b = fork(&a, bid);
        let tb = tree(&b);

        t.mov(s1, e2).must();
        a.commit();
        tb.mov(s2, x).must();
        b.commit();
        sync(&a, &b);

        same(&a, &b);
        for t in [&t, &tb] {
            for n in t.nodes() {
                assert_that!(visible(t, n)).is_true();
            }
            assert_that!(roots(t).len()).is_equal_to(1);
            if bid == 2 {
                // A (pair 1) passe d'abord : s1 sous e2, s2 reste racine.
                assert_that!(roots(t)).is_equal_to(vec![s2]);
                assert_that!(t.parent(s1)).is_equal_to(Some(TreeParentId::Node(e2)));
            } else {
                // B (pair 0) passe d'abord : s2 sous x, s1 reste racine.
                assert_that!(roots(t)).is_equal_to(vec![s1]);
                assert_that!(t.parent(s2)).is_equal_to(Some(TreeParentId::Node(x)));
            }
        }
    }
}

fn ms(start: Instant, n: u32) -> f64 {
    start.elapsed().as_secs_f64() * 1e3 / f64::from(n)
}

/// Lecture complète : slides, éléments, méta et texte.
fn read_all(t: &LoroTree, slides: &[TreeID]) -> usize {
    let mut seen = 0;
    for s in slides {
        for x in kids(t, *s) {
            if !marked(t, x) {
                seen += meta_text(t, x).len() + meta_str(t, x, "kind").map_or(0, |k| k.len());
            }
        }
    }
    seen
}

/// Point 8 : Coût, pour le rapport seulement (rien n'est asserté sur le temps).
/// Deck de 40 slides de 20 éléments : un seul arbre contre 40 arbres.
#[test]
fn p8_cost_single_tree_vs_tree_per_slide() {
    let (slides, per) = (40usize, 20usize);

    let a = peer(1);
    let t = tree(&a);
    let start = Instant::now();
    let mut elems = Vec::new();
    for _ in 0..slides {
        let s = t.create(None).must();
        for j in 0..per {
            elems.push(element(&t, s, "a", &format!("élément {j}")));
        }
    }
    a.commit();
    let build_one = ms(start, 1);

    let n = 50;
    let start = Instant::now();
    for _ in 0..n {
        let all = roots(&t);
        assert_that!(read_all(&t, &all)).is_greater_than(0);
    }
    let read_one = ms(start, n);

    let mut rng = Lcg(3);
    let n = 200;
    let start = Instant::now();
    for i in 0..n {
        let x = elems[i as usize % elems.len()];
        let all = roots(&t);
        let to = all[rng.below(all.len())];
        let k = rng.below(kids(&t, to).len().max(1));
        t.mov_to(x, to, k).must();
        set_slot(&t, x, to, "a");
        a.commit();
    }
    let mov_one = ms(start, n);
    assert_that!(t.get_nodes(false).len()).is_equal_to(slides * (per + 1));
    let snap_one = a.export(ExportMode::Snapshot).must().len();

    // 40 arbres : un passage d'une slide à l'autre est une copie puis une suppression.
    let d = peer(2);
    let start = Instant::now();
    let trees: Vec<(LoroTree, TreeID)> = (0..slides)
        .map(|i| {
            let t = tree_named(&d, i);
            let s = t.create(None).must();
            for j in 0..per {
                element(&t, s, "a", &format!("élément {j}"));
            }
            (t, s)
        })
        .collect();
    d.commit();
    let build_many = ms(start, 1);

    let n = 50;
    let start = Instant::now();
    for _ in 0..n {
        let seen: usize = trees.iter().map(|(t, s)| read_all(t, &[*s])).sum();
        assert_that!(seen).is_greater_than(0);
    }
    let read_many = ms(start, n);

    let n = 200;
    let start = Instant::now();
    for i in 0..n {
        let (from, fs) = &trees[i as usize % slides];
        let (to, ts) = &trees[rng.below(slides)];
        let Some(x) = kids(from, *fs).first().copied() else { continue };
        let text = meta_text(from, x);
        let y = to.create_at(*ts, rng.below(kids(to, *ts).len() + 1)).must();
        let meta = to.get_meta(y).must();
        meta.insert("kind", "text").must();
        meta.insert_container("text", LoroText::new()).must().insert(0, &text).must();
        set_slot(to, y, *ts, "a");
        from.delete(x).must();
        d.commit();
    }
    let mov_many = ms(start, n);
    let snap_many = d.export(ExportMode::Snapshot).must().len();

    println!(
        "coût 40x20 : un arbre — construction {build_one:.1} ms, lecture complète {read_one:.3} ms, \
         mov entre slides + commit {mov_one:.3} ms, snapshot {snap_one} o après 200 mov ; \
         40 arbres — construction {build_many:.1} ms, lecture complète {read_many:.3} ms, \
         copie + suppression entre arbres + commit {mov_many:.3} ms, snapshot {snap_many} o après 200 passages"
    );
}

fn tree_named(doc: &LoroDoc, i: usize) -> LoroTree {
    let t = doc.get_tree(format!("slide-{i}").as_str());
    t.enable_fractional_index(0);
    t
}

/// Point 9 : `TreeID::to_string` donne `counter@peer`, se relit par
/// `TreeID::try_from`, et la clé `@slot` qui en est faite se relit à
/// l'identique après snapshot et chez un autre pair.
#[test]
fn p9_tree_id_string_key_is_stable_across_snapshot_and_peers() {
    let a = peer(0x1234_5678_9abc_def0);
    let t = tree(&a);
    let s = t.create(None).must();
    let x = element(&t, s, "a", "hello");
    a.commit();

    let key = s.to_string();
    assert_that!(key).is_equal_to(format!("{}@{}", s.counter, s.peer));
    assert_that!(key.as_str()).is_equal_to("0@1311768467463790320");
    assert_that!(TreeID::try_from(key.as_str()).must()).is_equal_to(s);
    assert_that!(s.to_string()).is_equal_to(&key);

    let snap = LoroDoc::from_snapshot(&a.export(ExportMode::Snapshot).must()).must();
    let b = fork(&a, 2);
    let c = peer(3);
    c.import(&a.export(ExportMode::all_updates()).must()).must();
    for doc in [&snap, &b, &c] {
        let t = doc.get_tree(TREE);
        let s2 = roots(&t)[0];
        assert_that!(s2).is_equal_to(s);
        assert_that!(s2.to_string()).is_equal_to(&key);
        let keys: Vec<String> = slot_map(&t, x).keys().map(|k| k.to_string()).collect();
        assert_that!(keys).is_equal_to(vec![key.clone()]);
        assert_that!(TreeID::try_from(keys[0].as_str()).must()).is_equal_to(s);
        assert_that!(slot_of(&t, x, s)).is_equal_to(Some("a".to_owned()));
    }
    // Une chaîne mal formée est refusée, pas devinée.
    assert_that!(TreeID::try_from("x@1").is_err()).is_true();
    assert_that!(TreeID::try_from("12").is_err()).is_true();
}
