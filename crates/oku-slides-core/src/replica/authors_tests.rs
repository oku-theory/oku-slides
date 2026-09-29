use speculoos::prelude::*;

use crate::{Deck, DeckReplica, SourceBundle};

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap()
}

fn titled(deck: &Deck, t: &str) -> Deck {
    let mut d = deck.clone();
    d.slides[2].params.set("title", crate::ParamValue::Text(t.into()));
    d
}

#[test]
fn each_commit_is_its_own_change() {
    let deck = roadmap();
    let mut r = DeckReplica::new(&deck, crate::test_registry::shared()).unwrap();
    let before = r.len_changes();
    for t in ["Un", "Deux", "Trois"] {
        r.apply(&titled(&deck, t), t, None).unwrap();
    }
    assert_that!(r.len_changes()).is_equal_to(before + 3);
}

#[test]
fn first_edit_records_author_for_this_peer() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.set_author(Some("Julie".into()));
    b.apply(titled(&deck, "Un"), "Renommer", None).unwrap();
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn blank_author_records_nothing() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.set_author(Some("   ".into()));
    b.apply(titled(&deck, "Un"), "Renommer", None).unwrap();
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_none();
}

#[test]
fn silent_apply_records_no_author() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.set_author(Some("Julie".into()));
    b.apply_silent(titled(&deck, "Canonique")).unwrap();
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_none();
}

/// L'inscription fait partie de l'étape annulable : une première édition
/// annulée n'a rien laissé à signer, le pair redevient sans nom.
#[test]
fn undoing_the_first_edit_removes_the_author_entry() {
    let deck = roadmap();
    let mut b = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap();
    b.set_author(Some("Julie".into()));
    b.apply(titled(&deck, "Un"), "Renommer", None).unwrap();
    assert_that!(b.undo().unwrap()).is_true();
    let peer = b.replica().peer_id();
    assert_that!(b.replica().author_of(peer)).is_none();
}

fn peer(deck: &Deck, snap: &[u8], author: &str) -> SourceBundle {
    let (replica, read) = DeckReplica::from_snapshot(snap, 1 << 24, crate::test_registry::shared()).unwrap();
    let (mut b, _) = SourceBundle::from_replica(replica, read, Default::default(), Default::default());
    b.set_author(Some(author.into()));
    let _ = deck;
    b
}

#[test]
fn merge_reports_imported_changes_and_authors() {
    let deck = roadmap();
    let snap = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap().replica().snapshot().unwrap();
    let mut a = peer(&deck, &snap, "Matthieu");
    let mut b = peer(&deck, &snap, "Julie");
    b.apply(titled(&deck, "Un"), "Un", None).unwrap();
    let mut d = titled(&deck, "Un");
    d.theme = Some("pro".into());
    b.apply(d, "Thème", None).unwrap();
    let (report, issues) = a.merge(b).unwrap();
    // Seul `validate` signale quelque chose : l'image de `roadmap.json` n'a
    // pas son asset dans le bundle. Rien d'illisible.
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-missing-asset"]);
    assert_that!(report.changes).is_equal_to(2);
    assert_that!(report.authors).is_equal_to(vec!["Julie".to_owned()]);
    assert_that!(report.anonymous).is_equal_to(0);
}

#[test]
fn merge_twice_imports_nothing_the_second_time() {
    let deck = roadmap();
    let snap = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap().replica().snapshot().unwrap();
    let mut a = peer(&deck, &snap, "Matthieu");
    let mut b = peer(&deck, &snap, "Julie");
    b.apply(titled(&deck, "Un"), "Un", None).unwrap();
    let again = SourceBundle::from_zip(
        &b.to_zip().unwrap(),
        crate::SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 28 },
        crate::test_registry::shared(),
    )
    .unwrap()
    .0;
    a.merge(b).unwrap();
    let (report, _) = a.merge(again).unwrap();
    assert_that!(report).is_equal_to(crate::MergeReport::default());
}

#[test]
fn merge_counts_nameless_peers_as_anonymous() {
    let deck = roadmap();
    let snap = SourceBundle::new(deck.clone(), crate::test_registry::shared()).unwrap().replica().snapshot().unwrap();
    let mut a = peer(&deck, &snap, "Matthieu");
    let (replica, read) = DeckReplica::from_snapshot(&snap, 1 << 24, crate::test_registry::shared()).unwrap();
    let (mut b, _) = SourceBundle::from_replica(replica, read, Default::default(), Default::default());
    b.apply(titled(&deck, "Un"), "Un", None).unwrap();
    let (report, _) = a.merge(b).unwrap();
    assert_that!(report.authors).is_empty();
    assert_that!(report.anonymous).is_equal_to(1);
}
