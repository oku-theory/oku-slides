use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::library::DraftWrite;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, session_on};
use crate::test_support_packs::{CARD_V2, draft_of, opener_on, pack_tab_from, scope_of, snapshot_json};
use crate::{BASE_PACK, Request, SessionErrorKind};

/// The deck of `s` in a widget session on `acme`: its unfrozen draft.
fn in_widget_session(s: &mut Session, acme: Uuid) -> PackVersion {
    let ns = s.user_deck().widgets.iter().find(|(_, p)| p.id == acme).map(|(ns, _)| ns.clone()).unwrap();
    s.set_widget(Some(&ns), "card", CARD_V2.into()).unwrap();
    let v = s.user_deck().widgets[&ns].version.clone();
    assert_that!(draft_of(s, acme, &v)).is_equal_to(DraftWrite::Replace(v.clone()));
    v
}

#[test]
fn route_pack_of_a_library_pack_from_a_deck_is_tab() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let route = s.route_pack(acme, None).unwrap();
    assert_that!(matches!(&route, PackRoute::Tab { scope, name } if *scope == scope_of(&s) && name == "acme"))
        .is_true();
}

#[test]
fn route_pack_to_a_tab_changes_nothing() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let v = in_widget_session(&mut s, acme);
    let before = snapshot_json(&s);
    s.route_pack(acme, None).unwrap();
    assert_that!(s.widget_session.is_some()).is_true();
    assert_that!(draft_of(&s, acme, &v)).is_equal_to(DraftWrite::Replace(v));
    assert_that!(snapshot_json(&s)).is_equal_to(before);
}

#[test]
fn take_preview_ends_the_deck_widget_session() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let v = in_widget_session(&mut s, acme);
    s.take_preview(acme).unwrap();
    assert_that!(draft_of(&s, acme, &v)).is_equal_to(DraftWrite::Child(v));
}

#[test]
fn take_preview_of_a_pack_that_became_deck_only_is_refused() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let current = s.library.current_version(acme).unwrap();
    let mut m = current.manifest.clone();
    m.scope = oku_slides_core::PackScope::Deck;
    s.doc.sources.add_pack(m.to_files(current.files.kind(), current.files.widgets().clone()).unwrap());
    let err = s.take_preview(acme).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::DeckOnly);
}

#[test]
fn route_pack_of_a_deck_only_pack_from_a_deck_tab_is_deck_tab() {
    let data = fresh_dir("oku-slides-route-");
    let dir = fresh_dir("oku-slides-route-tab-");
    let mut s = session_on(data.path());
    assert_that!(s.resume_fresh(dir.path().to_path_buf())).is_true();
    let d = adopt_deck_only_pack(&mut s);
    s.flush_resume();
    let state = std::fs::read(dir.path().join("state.json")).unwrap();
    let route = s.route_pack(d.id, None).unwrap();
    assert_that!(matches!(&route, PackRoute::DeckTab { name } if name == "Importé")).is_true();
    s.flush_resume();
    assert_that!((matches!(s.mode, Mode::Deck), std::fs::read(dir.path().join("state.json")).unwrap()))
        .is_equal_to((true, state));
}

#[test]
fn route_pack_of_a_deck_only_pack_without_lease_scope_is_refused() {
    let mut s = crate::test_support::session();
    let d = adopt_deck_only_pack(&mut s);
    s.library = crate::PackLibrary::new(None, None, s.themes.clone(), s.storage.clone());
    assert_that!(s.route_pack(d.id, None).unwrap_err().kind()).is_equal_to(SessionErrorKind::DeckOnly);
}

#[test]
fn route_pack_of_the_edited_pack_opens_the_widget_here() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let route = t.route_pack(acme, Some("card")).unwrap();
    assert_that!(matches!(route, PackRoute::Here)).is_true();
    assert_that!(t.ui.widget.as_deref()).is_equal_to(Some("card"));
}

#[test]
fn route_pack_of_another_library_pack_from_a_pack_tab_is_tab() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    assert_that!(matches!(t.route_pack(BASE_PACK, None).unwrap(), PackRoute::Tab { .. })).is_true();
}

#[test]
fn route_pack_of_another_deck_only_pack_from_a_pack_tab_is_in_pack_tab() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let d = adopt_deck_only_pack(&mut s);
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let err = t.route_pack(d.id, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InPackTab);
}

#[rstest]
#[case::help(true, SessionErrorKind::ReadOnly)]
#[case::history(false, SessionErrorKind::InHistory)]
fn route_pack_from_help_or_history_is_refused(#[case] help: bool, #[case] kind: SessionErrorKind) {
    let mut s = crate::test_support::session();
    if help {
        s.open_help("fr", None).unwrap();
    } else {
        s.handle(Request::HistoryOpen).unwrap();
    }
    assert_that!(s.route_pack(BASE_PACK, None).unwrap_err().kind()).is_equal_to(kind);
}

/// What `route_pack_of_an_unknown_pack_or_widget_is_refused` opens.
#[derive(Debug, Clone, Copy)]
enum Unknown {
    Pack,
    LibraryWidget,
    DeckOnlyWidget,
}

#[rstest]
#[case::pack(Unknown::Pack, SessionErrorKind::UnknownPack)]
#[case::library_widget(Unknown::LibraryWidget, SessionErrorKind::UnknownWidget)]
#[case::deck_only_widget(Unknown::DeckOnlyWidget, SessionErrorKind::UnknownWidget)]
fn route_pack_of_an_unknown_pack_or_widget_is_refused(#[case] what: Unknown, #[case] kind: SessionErrorKind) {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let d = adopt_deck_only_pack(&mut s);
    let (pack, widget) = match what {
        Unknown::Pack => (Uuid::new_v4(), None),
        Unknown::LibraryWidget => (acme, Some("nope")),
        Unknown::DeckOnlyWidget => (d.id, Some("nope")),
    };
    let before = snapshot_json(&s);
    assert_that!(s.route_pack(pack, widget).unwrap_err().kind()).is_equal_to(kind);
    assert_that!(snapshot_json(&s)).is_equal_to(before);
    assert_that!(s.editing_view().is_none()).is_true();
}

#[test]
fn take_preview_seed_carries_the_font_but_no_asset() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    assert_that!(s.doc.sources.assets.is_empty()).is_false();
    let seed = s.take_preview(acme).unwrap();
    assert_that!(seed.fonts.contains_key("Inter.ttf")).is_true();
    let lease = PackLease::acquire(&scope_of(&s), acme, "acme", "acme").unwrap();
    let mut t = session_on(data.path());
    t.init_pack_tab(*seed, lease, acme, None).unwrap();
    assert_that!(crate::test_support::preview_of(&t).sources.assets.is_empty()).is_true();
}

#[test]
fn take_preview_seed_keeps_only_the_versions_its_pins_reach() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let unpinned: Vec<PackVersion> = {
        let reach = crate::deck_closure::pinned_versions(s.user_deck());
        s.doc.sources.packs.keys().filter(|v| !reach.contains(*v)).cloned().collect()
    };
    assert_that!(unpinned.is_empty()).is_false();
    let seed = s.take_preview(acme).unwrap();
    assert_that!(seed.packs.keys().any(|v| unpinned.contains(v))).is_false();
    let reached: std::collections::BTreeSet<PackVersion> = crate::deck_closure::pinned_versions(&seed.deck)
        .into_iter()
        .filter(|v| s.doc.sources.packs.contains_key(v))
        .collect();
    assert_that!(seed.packs.keys().cloned().collect::<std::collections::BTreeSet<_>>()).is_equal_to(reached);
}
