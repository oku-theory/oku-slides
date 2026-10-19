//! Compaction des nœuds morts à l'ouverture (t-770).

use std::sync::Arc;

use loro::{TreeID, TreeParentId};
use speculoos::prelude::*;

use super::super::tree::{deck_tree_of, node_of};
use super::super::{DeckReplica, ReplicaLimits};
use super::*;
use crate::{Deck, MergeReport};

const DAY: i64 = 86_400;
const DELAY: i64 = 30 * DAY;

fn registry() -> Arc<crate::WidgetRegistry> {
    crate::test_registry::shared()
}

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

/// Un replica de `roadmap` sans `b-points` (retiré, donc marqué maintenant), et le `TreeID` de `b-points`.
fn with_points_removed() -> (DeckReplica, TreeID) {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, registry()).unwrap_or_else(|e| panic!("{e}"));
    let (_, points) = node_of(&r.doc, "s-constat", "b-points");
    let mut without = deck.clone();
    let remove = serde_json::json!({ "op": "remove_element", "slide": "s-constat", "element": "b-points" });
    crate::apply_all(&mut without, vec![serde_json::from_value(remove).unwrap_or_else(|e| panic!("{e}"))], &registry())
        .unwrap_or_else(|e| panic!("{e:?}"));
    r.apply(&without, "Retirer", None).unwrap_or_else(|e| panic!("{e}"));
    (r, points)
}

/// Pose sur `n` la marque `@deleted/<peer>` datée `at`, en un commit (daté maintenant par Loro).
fn forge_mark(r: &DeckReplica, n: TreeID, peer: u64, at: i64) {
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    tree.get_meta(n)
        .unwrap_or_else(|e| panic!("{e}"))
        .insert(&format!("@deleted/{peer}"), at)
        .unwrap_or_else(|e| panic!("{e}"));
    r.doc.commit();
}

/// Un change daté `at` (dans le futur) : le document a eu de l'activité jusque-là.
fn activity_at(r: &DeckReplica, at: i64) {
    r.doc.get_map("meta").insert("activité", at).unwrap_or_else(|e| panic!("{e}"));
    r.doc.set_next_commit_timestamp(at);
    r.doc.commit();
}

/// Rouvre `r` comme à l'ouverture d'un fichier, à l'heure `at`, avec le délai `after`.
fn reopen(r: &DeckReplica, at: i64, after: i64) -> DeckReplica {
    let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
    let limits = ReplicaLimits { max_bytes: 1 << 24, compact_after_secs: after };
    DeckReplica::import_snapshot(&bytes, limits, registry(), at).unwrap_or_else(|e| panic!("{e}")).0
}

fn deleted(r: &DeckReplica, n: TreeID) -> bool {
    deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre")).is_node_deleted(&n).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn open_compacts_a_node_with_a_mark_older_than_the_delay() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let reopened = reopen(&r, now(), DELAY);
    assert_that!(deleted(&reopened, points)).is_true();
    assert_that!(reopened.deck().0).is_equal_to(r.deck().0);
}

#[test]
fn open_keeps_a_node_whose_marks_are_all_recent() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 10 * DAY);
    assert_that!(deleted(&reopen(&r, now(), DELAY), points)).is_false();
}

#[test]
fn open_compacts_after_thirty_days_of_document_activity() {
    let (r, points) = with_points_removed();
    // La marque du retrait date de maintenant ; 31 jours d'activité la rendent ancienne.
    activity_at(&r, now() + 31 * DAY);
    assert_that!(deleted(&reopen(&r, now() + 31 * DAY, DELAY), points)).is_true();
}

#[test]
fn open_with_a_clock_far_ahead_is_bounded_by_the_newest_change() {
    let (r, points) = with_points_removed();
    // Horloge locale à +10 ans, sans activité : la limite reste « change le plus récent − 30 j ».
    assert_that!(deleted(&reopen(&r, now() + 3650 * DAY, DELAY), points)).is_false();
}

#[test]
fn compact_doc_compacts_a_mark_exactly_at_the_limit_and_keeps_one_a_second_later() {
    let (r, points) = with_points_removed();
    let newest = newest_change(&r.doc).unwrap_or_else(|| panic!("change daté"));
    let limit = newest - DELAY;
    forge_mark(&r, points, 7, limit + 1);
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    // Les marques du retrait (maintenant) et forgée (limit + 1) sont récentes.
    let at = newest_change(&r.doc).unwrap_or_else(|| panic!("change daté"));
    assert_that!(compact_doc(&r.doc, at, at - limit).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    forge_mark(&r, points, 8, limit);
    let at = newest_change(&r.doc).unwrap_or_else(|| panic!("change daté"));
    assert_that!(compact_doc(&r.doc, at, at - limit).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(1);
    assert_that!(tree.is_node_deleted(&points).unwrap_or_else(|e| panic!("{e}"))).is_true();
}

#[test]
fn is_old_never_holds_for_a_null_or_negative_date() {
    for t in [0, -1, i64::MIN] {
        assert_that!(is_old(t, i64::MAX)).named(&t.to_string()).is_false();
    }
    assert_that!(is_old(1, 1)).is_true();
    assert_that!(is_old(2, 1)).is_false();
}

#[test]
fn compact_doc_compacts_an_empty_node_after_thirty_days_of_activity() {
    let r = DeckReplica::new(&roadmap(), registry()).unwrap_or_else(|e| panic!("{e}"));
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    let empty = tree.create(TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
    r.doc.commit();
    assert_that!(compact_doc(&r.doc, now() + 31 * DAY, DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    activity_at(&r, now() + 31 * DAY);
    assert_that!(compact_doc(&r.doc, now() + 31 * DAY, DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(1);
    assert_that!(tree.is_node_deleted(&empty).unwrap_or_else(|e| panic!("{e}"))).is_true();
}

#[test]
fn compact_doc_counts_a_dead_parent_and_its_dead_child_once() {
    let (r, points) = with_points_removed();
    let tree = deck_tree_of(&r.doc).unwrap_or_else(|| panic!("arbre"));
    let Some(TreeParentId::Node(slide)) = tree.parent(points) else { panic!("slide de b-points") };
    let parent = tree.create(TreeParentId::Node(slide)).unwrap_or_else(|e| panic!("{e}"));
    let child = tree.create(TreeParentId::Node(parent)).unwrap_or_else(|e| panic!("{e}"));
    for n in [child, parent] {
        let meta = tree.get_meta(n).unwrap_or_else(|e| panic!("{e}"));
        meta.insert("id", "x").unwrap_or_else(|e| panic!("{e}"));
        meta.insert("@deleted/9", now() - 40 * DAY).unwrap_or_else(|e| panic!("{e}"));
    }
    r.doc.commit();
    assert_that!(compact_doc(&r.doc, now(), DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(1);
    for n in [parent, child] {
        assert_that!(tree.is_node_deleted(&n).unwrap_or_else(|e| panic!("{e}"))).is_true();
    }
    assert_that!(tree.is_node_deleted(&points).unwrap_or_else(|e| panic!("{e}"))).is_false();
}

#[test]
fn compact_doc_with_nothing_to_compact_makes_no_commit() {
    let r = DeckReplica::new(&roadmap(), registry()).unwrap_or_else(|e| panic!("{e}"));
    let before = r.doc.oplog_vv();
    assert_that!(compact_doc(&r.doc, now(), DELAY).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    assert_that!(r.doc.oplog_vv()).is_equal_to(before);
}

#[test]
fn compact_doc_with_a_zero_delay_compacts_nothing() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, 1);
    assert_that!(compact_doc(&r.doc, now(), 0).unwrap_or_else(|e| panic!("{e}"))).is_equal_to(0);
    assert_that!(deleted(&r, points)).is_false();
}

#[test]
fn open_compaction_is_not_undoable_and_leaves_no_history_notch() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let versions = r.history().len();
    let mut reopened = reopen(&r, now(), DELAY);
    assert_that!(deleted(&reopened, points)).named("précondition").is_true();
    assert_that!(reopened.can_undo()).is_false();
    assert_that!(reopened.history().len()).is_equal_to(versions);
    // Une édition après l'ouverture se défait sans ramener le nœud compacté.
    let mut renamed = reopened.deck().0;
    renamed.slides[0].name = Some("renommée".to_owned());
    reopened.apply(&renamed, "Renommer", None).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(reopened.undo().unwrap_or_else(|e| panic!("{e}"))).is_true();
    assert_that!(deleted(&reopened, points)).is_true();
    assert_that!(reopened.deck().0).is_equal_to(r.deck().0);
}

#[test]
fn open_dates_the_compaction_commit_like_the_newest_change() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let newest = newest_change(&r.doc).unwrap_or_else(|| panic!("change daté"));
    // Loro date un commit à l'horloge réelle, pas au `now` passé : on laisse l'horloge avancer
    // pour que « maintenant » diffère du change le plus récent, puis on ouvre bien plus tard.
    std::thread::sleep(std::time::Duration::from_millis(2100));
    let reopened = reopen(&r, now() + 40 * DAY, DELAY);
    assert_that!(deleted(&reopened, points)).named("précondition").is_true();
    let heads: Vec<loro::ID> = reopened.doc.oplog_frontiers().iter().collect();
    let mut compact_at: Option<i64> = None;
    reopened
        .doc
        .travel_change_ancestors(&heads, &mut |m| {
            if m.message.as_deref() == Some(COMPACT) {
                compact_at = Some(m.timestamp);
            }
            std::ops::ControlFlow::Continue(())
        })
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(compact_at).is_equal_to(Some(newest));
    assert_that!(newest_change(&reopened.doc)).is_equal_to(Some(newest));
}

#[test]
fn compacted_and_uncompacted_copies_converge() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let mut a = reopen(&r, now(), DELAY);
    let mut b = reopen(&r, now(), 0);
    assert_that!(deleted(&a, points)).named("précondition a").is_true();
    assert_that!(deleted(&b, points)).named("précondition b").is_false();
    let mut renamed = b.deck().0;
    renamed.slides[0].name = Some("renommée".to_owned());
    b.apply(&renamed, "Renommer", None).unwrap_or_else(|e| panic!("{e}"));
    a.merge(&b).unwrap_or_else(|e| panic!("{e}"));
    b.merge(&a).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(a.deck().0).is_equal_to(b.deck().0);
    assert_that!(deleted(&b, points)).is_true();
}

#[test]
fn merge_report_does_not_count_the_compaction() {
    let (r, points) = with_points_removed();
    forge_mark(&r, points, 7, now() - 40 * DAY);
    let mut a = reopen(&r, now(), 0);
    let b = reopen(&r, now(), DELAY);
    assert_that!(deleted(&b, points)).named("précondition").is_true();
    let report = a.merge(&b).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(report).is_equal_to(MergeReport { changes: 0, authors: Vec::new(), anonymous: 0 });
}

/// Mesure t-770 : 40 slides × 20 éléments, ~10 000 nœuds marqués, avant et après compaction.
#[test]
#[ignore = "mesure"]
fn measure_compaction_cost_40x20_with_10000_marked() {
    let text = |id: String, t: &str| serde_json::json!({ "id": id, "kind": "base/text", "params": { "body": [{ "runs": [{ "t": t }] }] } });
    let slot = |v: Vec<serde_json::Value>| -> crate::ParamValue {
        crate::ParamValue::Slot(serde_json::from_value(serde_json::Value::Array(v)).unwrap_or_else(|e| panic!("{e}")))
    };
    let mut deck = roadmap();
    let template = deck.slides.iter().find(|s| s.id == "s-constat").cloned().unwrap_or_else(|| panic!("s-constat"));
    deck.slides = (0..40)
        .map(|i| {
            let mut s = template.clone();
            s.id = format!("s{i}");
            s.params.set("blocks", slot((0..20).map(|j| text(format!("e{i}-{j}"), "x")).collect()));
            s
        })
        .collect();
    let mut r = DeckReplica::new(&deck, registry()).unwrap_or_else(|e| panic!("{e}"));
    for round in 0..500 {
        let mut more = deck.clone();
        let s = &mut more.slides[round % 40];
        let mut blocks: Vec<serde_json::Value> = (0..20).map(|j| text(format!("e{}-{j}", round % 40), "x")).collect();
        blocks.extend((0..20).map(|j| text(format!("t{round}-{j}"), "y")));
        s.params.set("blocks", slot(blocks));
        r.apply(&more, "ajout", None).unwrap_or_else(|e| panic!("{e}"));
        r.apply(&deck, "retrait", None).unwrap_or_else(|e| panic!("{e}"));
    }
    let measure = |r: &DeckReplica, label: &str| {
        let t = std::time::Instant::now();
        let read = r.deck().0;
        let read_time = t.elapsed();
        let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
        let t = std::time::Instant::now();
        let (mut back, _) = DeckReplica::from_snapshot(&bytes, 1 << 26, registry()).unwrap_or_else(|e| panic!("{e}"));
        let open_time = t.elapsed();
        let mut renamed = read.clone();
        renamed.slides[0].name = Some("renommée".to_owned());
        let t = std::time::Instant::now();
        back.apply(&renamed, "titre", None).unwrap_or_else(|e| panic!("{e}"));
        let apply_time = t.elapsed();
        let mut other = DeckReplica::from_snapshot(&bytes, 1 << 26, registry()).unwrap_or_else(|e| panic!("{e}")).0;
        let t = std::time::Instant::now();
        other.merge(&back).unwrap_or_else(|e| panic!("{e}"));
        println!(
            "{label} : deck() {read_time:?}, from_snapshot {open_time:?}, apply {apply_time:?}, merge {:?}, snapshot {} o",
            t.elapsed(),
            bytes.len()
        );
    };
    measure(&r, "avant");
    activity_at(&r, now() + 31 * DAY);
    let bytes = r.snapshot().unwrap_or_else(|e| panic!("{e}"));
    let compacted_doc = loro::LoroDoc::new();
    compacted_doc.import(&bytes).unwrap_or_else(|e| panic!("{e}"));
    let count = compact_doc(&compacted_doc, now() + 31 * DAY, DELAY).unwrap_or_else(|e| panic!("{e}"));
    println!("nœuds compactés : {count}");
    assert_that!(count).is_greater_than(9_000);
    let limits = ReplicaLimits { max_bytes: 1 << 26, compact_after_secs: DELAY };
    let (compacted, _) =
        DeckReplica::import_snapshot(&bytes, limits, registry(), now() + 31 * DAY).unwrap_or_else(|e| panic!("{e}"));
    measure(&compacted, "après");
}
