use speculoos::prelude::*;

use crate::{Deck, SourceBundle, slide_changes};

fn roadmap() -> Deck {
    let mut deck = Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap();
    crate::canonicalize(&mut deck);
    deck
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("title", crate::ParamValue::Text(t.into()));
    d
}

fn edited(labels: &[&str]) -> (Deck, SourceBundle) {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.set_author(Some("Julie".into()));
    for l in labels {
        b.apply(titled(&deck, l), l, None).unwrap();
    }
    (deck, b)
}

#[test]
fn history_lists_creation_then_one_version_per_commit() {
    let (_, b) = edited(&["Un", "Deux"]);
    let h = b.history();
    assert_that!(h.len()).is_equal_to(3);
    let labels: Vec<_> = h.versions().iter().map(|v| v.label.clone().unwrap_or_default()).collect();
    assert_that!(labels[1..].to_vec()).is_equal_to(vec!["Un".to_owned(), "Deux".to_owned()]);
    assert_that!(h.versions()[1].author.as_deref()).is_equal_to(Some("Julie"));
    // Ruling 1 : `DeckReplica::new` committe la création avec le même pair
    // Loro que les modifications suivantes, donc le début de session est au
    // premier cran (index 0), pas au premier commit étiqueté (index 1).
    assert_that!(h.versions()[0].session_start).is_true();
    assert_that!(h.versions()[1].session_start).is_false();
    assert_that!(h.versions()[2].session_start).is_false();
}

#[test]
fn deck_at_reads_the_state_after_each_version() {
    let (deck, b) = edited(&["Un", "Deux"]);
    let h = b.history();
    assert_that!(h.deck_at(0).unwrap().0).is_equal_to(&deck);
    assert_that!(h.deck_at(1).unwrap().0.slides[2].params.text("title")).is_equal_to(Some("Un"));
    assert_that!(h.deck_at(99).unwrap().0.slides[2].params.text("title")).is_equal_to(Some("Deux"));
}

#[test]
fn history_marks_the_merge_and_keeps_concurrent_branches_in_each_prefix() {
    let deck = roadmap();
    let base = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let zip = base.to_zip().unwrap();
    let limits = crate::SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 28, compact_after_secs: 0 };
    let (mut a, _) = SourceBundle::from_zip(&zip, limits, crate::test_registry::shared()).unwrap();
    let (mut b, _) = SourceBundle::from_zip(&zip, limits, crate::test_registry::shared()).unwrap();
    a.apply(titled(&deck, "de A"), "A", None).unwrap();
    let mut d = deck.clone();
    crate::test_registry::set_theme(&mut d, "b");
    b.apply(d, "B", None).unwrap();
    a.merge(b).unwrap();
    let mut after = a.deck().clone();
    after.slides[3].params.set("title", crate::ParamValue::Text("après".into()));
    a.apply(after, "Après", None).unwrap();
    let h = a.history();
    let last = h.len() - 1;
    assert_that!(h.versions()[last].merge).is_true();
    // Le cran juste avant la fusion contient les deux branches (préfixe, pas ancêtres).
    let before = h.deck_at(last - 1).unwrap().0;
    assert_that!(before.slides[2].params.text("title")).is_equal_to(Some("de A"));
    assert_that!(before.themes).is_equal_to(crate::test_registry::theme_pins("b"));
}

#[test]
fn restore_brings_back_a_version_as_a_new_undoable_commit() {
    let (_, mut b) = edited(&["Un", "Deux"]);
    let h = b.history();
    b.restore(&h, 1, "Restaurer").unwrap();
    assert_that!(b.deck().slides[2].params.text("title")).is_equal_to(Some("Un"));
    assert_that!(b.history().len()).is_equal_to(4);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Restaurer".to_owned()));
    b.undo().unwrap();
    assert_that!(b.deck().slides[2].params.text("title")).is_equal_to(Some("Deux"));
}

#[test]
fn restore_keeps_authors_of_later_sessions() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let old = b.history();
    b.set_author(Some("Julie".into()));
    b.apply(titled(&deck, "Un"), "Un", None).unwrap();
    let julie = b.replica().peer_id();
    b.restore(&old, 0, "Restaurer").unwrap();
    assert_that!(b.replica().author_of(julie)).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn restore_with_no_prior_edit_does_not_leave_a_spurious_unlabeled_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.set_author(Some("Julie".into()));
    let h = b.history();
    b.restore(&h, 0, "Restaurer").unwrap();
    // Un seul cran de plus (« Restaurer ») : pas de commit parasite sans
    // libellé venant d'un `claim_author` pris à part avant `revert_to`.
    assert_that!(b.history().len()).is_equal_to(2);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Restaurer".to_owned()));
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_equal_to(Some("Julie".to_owned()));
    b.undo().unwrap();
    assert_that!(b.replica().undo_label()).is_equal_to(None);
}

/// Restaure la version `i` de `h` et vérifie que le deck est exactement
/// cette version, en une étape d'annulation au libellé donné.
fn restore_exactly(b: &mut SourceBundle, h: &super::HistoryPreview, i: usize) {
    let want = h.deck_at(i).unwrap().0;
    b.restore(h, i, "Restaurer").unwrap();
    assert_that!(b.deck().slides.len()).named(&format!("slides, version {i}")).is_equal_to(want.slides.len());
    assert_that!(b.deck()).named(&format!("version {i}")).is_equal_to(&want);
    assert_that!(b.replica().deck().0).is_equal_to(&want);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Restaurer".to_owned()));
}

/// [`restore_exactly`], puis annuler ramène le deck d'avant.
fn restore_and_undo(b: &mut SourceBundle, h: &super::HistoryPreview, i: usize) {
    let before = b.deck().clone();
    restore_exactly(b, h, i);
    b.undo().unwrap();
    assert_that!(b.deck()).named(&format!("annuler, version {i}")).is_equal_to(&before);
}

/// Le deck avec la slide `from` déplacée en `to` (un glisser de l'interface).
fn moved(deck: &Deck, from: usize, to: usize) -> Deck {
    let mut d = deck.clone();
    let s = d.slides.remove(from);
    d.slides.insert(to, s);
    d
}

/// Le deck avec les slides `a` et `b` échangées.
fn swapped(deck: &Deck, a: usize, b: usize) -> Deck {
    let mut d = deck.clone();
    d.slides.swap(a, b);
    d
}

/// Le deck sans la slide `i`.
fn without(deck: &Deck, i: usize) -> Deck {
    let mut d = deck.clone();
    d.slides.remove(i);
    d
}

#[test]
fn restore_across_a_slide_move_brings_back_every_slide() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.apply(moved(&deck, 3, 0), "Déplacer", None).unwrap();
    let h = b.history();
    restore_and_undo(&mut b, &h, 0);
}

#[test]
fn restore_across_a_delete_then_a_move_brings_back_each_earlier_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let fewer = without(&deck, 2);
    b.apply(fewer.clone(), "Supprimer", None).unwrap();
    b.apply(moved(&fewer, 4, 1), "Déplacer", None).unwrap();
    let h = b.history();
    for i in 0..h.len() - 1 {
        restore_and_undo(&mut b, &h, i);
    }
}

#[test]
fn restore_after_a_zip_round_trip_brings_back_each_earlier_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let fewer = without(&deck, 5);
    b.apply(fewer.clone(), "Supprimer", None).unwrap();
    b.apply(moved(&fewer, 0, 4), "Déplacer", None).unwrap();
    let limits = crate::SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 28, compact_after_secs: 0 };
    let (mut b, _) = SourceBundle::from_zip(&b.to_zip().unwrap(), limits, crate::test_registry::shared()).unwrap();
    let h = b.history();
    for i in 0..h.len() - 1 {
        restore_and_undo(&mut b, &h, i);
    }
}

/// Le cas de la revue : supprimer, échanger deux slides (plusieurs `mov` en
/// un commit), puis restaurer chaque version. `revert_to` y perdait une
/// slide ou échouait (`ContainerDeleted`).
#[test]
fn restore_across_a_delete_then_a_swap_brings_back_each_earlier_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let fewer = without(&deck, 2);
    b.apply(fewer.clone(), "Supprimer", None).unwrap();
    b.apply(swapped(&fewer, 0, 3), "Échanger", None).unwrap();
    let h = b.history();
    for i in 0..h.len() - 1 {
        restore_exactly(&mut b, &h, i);
    }
}

/// Un échange de slides non voisines, annulé : t-263 (un `mov` par commit)
/// le rend sans perte, restauration comprise.
#[test]
fn restore_across_a_swap_can_be_undone() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.apply(swapped(&deck, 0, 3), "Échanger", None).unwrap();
    let h = b.history();
    restore_and_undo(&mut b, &h, 0);
}

/// Un réordonnancement de slides est un seul change Loro (arbre du deck,
/// sans commit intermédiaire) : un seul cran, à l'état d'après, sous son libellé.
#[test]
fn a_reorder_of_several_slides_is_one_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut reordered = deck.clone();
    reordered.slides.reverse();
    let changes = b.replica().len_changes();
    b.apply(reordered.clone(), "Réordonner", None).unwrap();
    assert_that!(b.replica().len_changes()).is_equal_to(changes + 1);
    let h = b.history();
    assert_that!(h.len()).is_equal_to(2);
    assert_that!(h.versions()[1].label.as_deref()).is_equal_to(Some("Réordonner"));
    assert_that!(h.deck_at(1).unwrap().0).is_equal_to(&reordered);
}

#[test]
fn restore_across_a_reorder_is_one_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.apply(swapped(&deck, 0, 3), "Échanger", None).unwrap();
    let h = b.history();
    let before = h.len();
    restore_exactly(&mut b, &h, 0);
    let after = b.history();
    assert_that!(after.len()).is_equal_to(before + 1);
    assert_that!(after.versions()[before].label.as_deref()).is_equal_to(Some("Restaurer"));
    assert_that!(after.deck_at(before).unwrap().0).is_equal_to(&deck);
}

#[test]
fn restore_from_an_unreadable_history_fails_and_leaves_the_replica_as_its_view() {
    let (_, mut b) = edited(&["Un", "Deux"]);
    let before = (b.deck().clone(), b.replica().len_changes());
    let h = b.history();
    // Un aperçu qui a paniqué (version illisible) : ses verrous sont empoisonnés.
    h.poison_for_tests();
    assert_that!(b.restore(&h, 0, "Restaurer").is_err()).is_true();
    assert_that!(b.deck()).is_equal_to(&before.0);
    assert_that!(b.replica().deck().0).is_equal_to(&before.0);
    // Rien en attente : un export ne committe rien de plus.
    b.to_zip().unwrap();
    assert_that!(b.replica().len_changes()).is_equal_to(before.1);
    assert_that!(b.replica().undo_label()).is_equal_to(Some("Deux".to_owned()));
}

#[test]
fn slide_changes_compares_slides_by_id() {
    let current = roadmap();
    let mut version = current.clone();
    version.slides[2].params.set("title", crate::ParamValue::Text("autre".into()));
    let removed = version.slides.remove(4).id;
    let mut extra = version.slides[0].clone();
    extra.id = "s-disparue".into();
    version.slides.push(extra);
    let c = slide_changes(&version, &current);
    assert_that!(c.changed_since).is_equal_to(vec![current.slides[2].id.clone()]);
    assert_that!(c.removed_since).is_equal_to(vec!["s-disparue".to_owned()]);
    assert_that!(c.added_since).is_equal_to(vec![removed]);
}

/// `roadmap` et un texte cadré `n` dans `elements` de `s-constat` (t-694).
fn two_slots() -> Deck {
    let op = |v: serde_json::Value| -> crate::Op {
        serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
    };
    let mut deck = roadmap();
    let n = serde_json::json!({ "id": "n", "kind": "base/text", "frame": { "x": 100, "y": 100, "w": 400, "h": 200 },
        "params": { "body": [{ "runs": [{ "t": "Note libre" }] }] } });
    let add = op(serde_json::json!({ "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "elements" }, "element": n }));
    crate::apply_all(&mut deck, vec![add], &crate::test_registry::registry()).unwrap_or_else(|e| panic!("{e:?}"));
    deck
}

/// `deck` où `b-points` quitte `blocks` pour `elements`, cadré : un `mov` d'arbre.
fn points_to_elements(deck: &Deck) -> Deck {
    let mut d = deck.clone();
    let slide = d.slide_mut("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    let blocks = slide.params.slot_mut("blocks").unwrap_or_else(|| panic!("blocks absent"));
    let at = blocks.iter().position(|e| e.id == "b-points").unwrap_or_else(|| panic!("b-points absent"));
    let mut points = blocks.remove(at);
    points.frame = Some(crate::Frame { x: 0.0, y: 0.0, w: 400.0, h: 200.0 });
    slide.params.slot_mut("elements").unwrap_or_else(|| panic!("elements absent")).push(points);
    crate::canonicalize(&mut d);
    d
}

/// Le slot de la slide `s-constat` qui porte `b-points`.
fn points_slot(deck: &Deck) -> Option<String> {
    deck.slide("s-constat").and_then(|s| s.locate("b-points")).map(|l| l.slot.to_owned())
}

#[test]
fn a_move_across_slots_is_one_version() {
    let deck = two_slots();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let (versions, changes) = (b.history().len(), b.replica().len_changes());
    b.apply(points_to_elements(&deck), "Déplacer", None).unwrap();
    assert_that!(b.history().len()).is_equal_to(versions + 1);
    assert_that!(b.replica().len_changes()).is_equal_to(changes + 1);
}

#[test]
fn deck_at_before_a_move_across_slots_shows_the_old_slot() {
    let deck = two_slots();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.apply(points_to_elements(&deck), "Déplacer", None).unwrap();
    let h = b.history();
    let last = h.len() - 1;
    assert_that!(points_slot(&h.deck_at(last - 1).unwrap().0)).is_equal_to(Some("blocks".to_owned()));
    assert_that!(points_slot(&h.deck_at(last).unwrap().0)).is_equal_to(Some("elements".to_owned()));
}

#[test]
fn restore_across_a_move_across_slots_brings_back_the_old_slot() {
    let deck = two_slots();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.apply(points_to_elements(&deck), "Déplacer", None).unwrap();
    let h = b.history();
    assert_that!(points_slot(&h.deck_at(0).unwrap().0)).is_equal_to(Some("blocks".to_owned()));
    restore_and_undo(&mut b, &h, 0);
    assert_that!(points_slot(b.deck())).is_equal_to(Some("elements".to_owned()));
}

/// `b-points` quitte `s-constat` pour `s-priorites`.
fn points_to_priorities(deck: &Deck) -> Deck {
    let mut d = deck.clone();
    let points = {
        let slide = d.slide_mut("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
        let blocks = slide.params.slot_mut("blocks").unwrap_or_else(|| panic!("blocks absent"));
        let at = blocks.iter().position(|e| e.id == "b-points").unwrap_or_else(|| panic!("b-points absent"));
        blocks.remove(at)
    };
    let slide = d.slide_mut("s-priorites").unwrap_or_else(|| panic!("s-priorites absente"));
    slide.params.slot_mut("blocks").unwrap_or_else(|| panic!("blocks absent")).insert(0, points);
    crate::canonicalize(&mut d);
    d
}

#[test]
fn a_move_between_slides_is_one_version() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let (versions, changes) = (b.history().len(), b.replica().len_changes());
    // Le renommage de la slide d'arrivée s'écrit avant le `mov` : un commit entre les deux se verrait.
    let mut moved = points_to_priorities(&deck);
    let target = moved.slide_mut("s-priorites").unwrap_or_else(|| panic!("s-priorites absente"));
    target.params.set("title", crate::ParamValue::Text("Priorités".into()));
    b.apply(moved, "Déplacer", None).unwrap();
    assert_that!(b.history().len()).is_equal_to(versions + 1);
    assert_that!(b.replica().len_changes()).is_equal_to(changes + 1);
}

/// Un deck où `s-constat` est retirée, avec l'historique d'avant et d'après.
fn after_removing_constat() -> (Deck, SourceBundle) {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    let mut removed = deck.clone();
    removed.slides.retain(|s| s.id != "s-constat");
    b.apply(removed, "Retirer", None).unwrap();
    (deck, b)
}

#[test]
fn deck_at_before_a_slide_removal_shows_the_slide() {
    let (deck, b) = after_removing_constat();
    let h = b.history();
    let last = h.len() - 1;
    assert_that!(h.deck_at(last - 1).unwrap().0).is_equal_to(&deck);
    assert_that!(h.deck_at(last).unwrap().0.slide("s-constat").is_none()).is_true();
}

/// Les racines du deck brut dont l'`id` est `slide`, avec leur marque (lue brute).
fn raw_roots(b: &SourceBundle, slide: &str) -> Vec<(loro::TreeID, bool)> {
    use loro::{Container, LoroValue, TreeParentId, ValueOrContainer};
    let Some(ValueOrContainer::Container(Container::Tree(tree))) =
        b.replica().doc.get_map(crate::replica::ROOT).get("slides")
    else {
        panic!("arbre du deck attendu")
    };
    tree.children(TreeParentId::Root)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|n| {
            let meta = tree.get_meta(n).unwrap_or_else(|e| panic!("{e:?}"));
            let named =
                matches!(meta.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if s.as_str() == slide);
            named.then(|| (n, meta.keys().any(|k| k.starts_with("@deleted/"))))
        })
        .collect()
}

/// Restaurer avant le retrait rend la slide sous un nouveau nœud : l'ancien
/// garde sa marque, jamais ranimé par son id (R5, point 6).
#[test]
fn restore_before_a_slide_removal_brings_it_back_as_a_new_node() {
    let (deck, mut b) = after_removing_constat();
    let removed = raw_roots(&b, "s-constat");
    assert_that!(removed.len()).is_equal_to(1);
    assert_that!(removed[0].1).is_true();
    let h = b.history();
    restore_and_undo(&mut b, &h, 0);
    b.restore(&h, 0, "Restaurer").unwrap();
    assert_that!(b.deck()).is_equal_to(&deck);
    let roots = raw_roots(&b, "s-constat");
    assert_that!(roots.len()).is_equal_to(2);
    let old = roots.iter().find(|(n, _)| *n == removed[0].0).unwrap_or_else(|| panic!("ancien nœud perdu"));
    assert_that!(old.1).named("l'ancien nœud garde sa marque").is_true();
    let new: Vec<_> = roots.iter().filter(|(n, _)| *n != removed[0].0).collect();
    assert_that!(new.len()).is_equal_to(1);
    assert_that!(new[0].1).named("le nouveau nœud est vivant").is_false();
}

fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement de pile"))
}

/// Un replica dont l'historique porte une chaîne de 20 000 nœuds vides sous
/// `s-constat` (versions « forge »), remontés ensuite sous la slide (« aplatie »).
fn forged_then_flattened() -> crate::DeckReplica {
    on_stack(1 << 30, || {
        let r = crate::DeckReplica::new(&roadmap(), crate::test_registry::shared()).unwrap_or_else(|e| panic!("{e}"));
        let tree = crate::replica::tree::deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
        let points = crate::replica::tree::node_of(&r.doc, "s-constat", "b-points").1;
        let Some(loro::TreeParentId::Node(slide)) = tree.parent(points) else { panic!("slide de b-points") };
        let mut parent = loro::TreeParentId::Node(slide);
        let mut chain = Vec::new();
        for _ in 0..20_000 {
            let n = tree.create(parent).unwrap_or_else(|e| panic!("{e}"));
            chain.push(n);
            parent = loro::TreeParentId::Node(n);
        }
        r.doc.set_next_commit_message("forge");
        r.doc.commit();
        for n in chain.iter().rev() {
            tree.mov(*n, loro::TreeParentId::Node(slide)).unwrap_or_else(|e| panic!("{e}"));
        }
        r.doc.set_next_commit_message("aplatie");
        r.doc.commit();
        r
    })
}

#[test]
fn deck_at_refuses_a_version_too_deep_and_reads_the_others_on_a_small_stack() {
    let r = forged_then_flattened();
    let current = r.deck().0;
    let h = r.history();
    // Loro peut découper un gros commit : la dernière version « forge » porte toute la chaîne.
    let forge =
        h.versions().iter().rposition(|v| v.label.as_deref() == Some("forge")).unwrap_or_else(|| panic!("forge"));
    let last = h.len() - 1;
    let (deep, latest, first) = on_stack(256 * 1024, move || {
        let deep = h.deck_at(forge).err().map(|e| e.kind());
        let latest = h.deck_at(last).map(|(d, _)| d).ok();
        let first = h.deck_at(0).map(|(d, _)| d).ok();
        (deep, latest, first)
    });
    assert_that!(deep).is_equal_to(Some(crate::error::ReplicaErrorKind::Corrupted));
    assert_that!(latest).is_equal_to(Some(current));
    assert_that!(first).is_equal_to(Some(roadmap()));
    on_stack(1 << 30, move || drop(r));
}

/// Témoin : un fichier dont l'historique a porté une chaîne profonde, mais dont
/// l'état est plat, s'ouvre par rejeu, s'enregistre et liste ses versions sur un fil de l'app.
#[test]
fn file_with_a_deep_past_and_a_flat_present_opens_saves_and_lists_history_on_an_app_thread() {
    let r = forged_then_flattened();
    let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
    on_stack(1 << 30, move || drop(r));
    let deep = on_stack(2 << 20, move || {
        let (back, _) = crate::DeckReplica::from_snapshot(&bytes, 1 << 26, crate::test_registry::shared())
            .unwrap_or_else(|e| panic!("ouverture : {e}"));
        back.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
        let h = back.history();
        let forge =
            h.versions().iter().rposition(|v| v.label.as_deref() == Some("forge")).unwrap_or_else(|| panic!("forge"));
        let deep = h.deck_at(forge).err().map(|e| e.kind());
        drop((h, back));
        deep
    });
    assert_that!(deep).is_equal_to(Some(crate::error::ReplicaErrorKind::Corrupted));
}
