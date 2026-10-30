use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::library::DraftWrite;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, session_on};
use crate::test_support_packs::{CARD_V2, draft_of, opener_on, pack_tab_from, scope_of, snapshot_json};
use crate::{BASE_PACK, ESSENTIEL_PACK, Request, SessionErrorKind};

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

/// A help session on `data` whose deck pins a deck-only pack, as no help deck does: its pin.
fn help_pinning_a_deck_only_pack(data: &std::path::Path) -> (Session, oku_slides_core::PackPin) {
    let mut s = session_on(data);
    let pin = adopt_deck_only_pack(&mut s);
    let files = s.doc.sources.packs[&pin.version].clone();
    s.open_help("fr", None).unwrap();
    s.doc.sources.add_pack(files);
    let mut deck = s.doc.sources.deck().clone();
    deck.pins_mut(oku_slides_core::PackKind::Widgets).insert(oku_slides_core::Namespace::deck(), pin.clone());
    s.doc.sources.apply_silent(deck).unwrap();
    (s, pin)
}

/// A help session on `data`.
fn help_on(data: &std::path::Path) -> Session {
    let mut s = session_on(data);
    s.open_help("fr", None).unwrap();
    s
}

#[test]
fn route_pack_of_a_library_pack_from_help_is_tab() {
    let data = fresh_dir("oku-slides-route-");
    let mut s = help_on(data.path());
    let route = s.route_pack(ESSENTIEL_PACK, None).unwrap();
    assert_that!(matches!(&route, PackRoute::Tab { scope, .. } if *scope == scope_of(&s))).is_true();
}

#[test]
fn take_preview_from_help_seeds_a_pack_tab_whose_preview_renders() {
    let data = fresh_dir("oku-slides-route-");
    let mut s = help_on(data.path());
    let seed = s.take_preview(ESSENTIEL_PACK).unwrap();
    let lease = PackLease::acquire(&scope_of(&s), ESSENTIEL_PACK, "essentiel", "essentiel").unwrap();
    let mut t = session_on(data.path());
    t.init_pack_tab(*seed, lease, ESSENTIEL_PACK, None).unwrap();
    let preview = crate::test_support::preview_of(&t).sources.deck().slides.len();
    let first = t.deck().slides[0].id.clone();
    let rendered = !t.svg(&first).unwrap().is_empty();
    assert_that!((t.is_pack_tab(), preview, rendered)).is_equal_to((true, s.deck().slides.len(), true));
}

#[test]
fn take_preview_from_help_changes_nothing_in_help() {
    let data = fresh_dir("oku-slides-route-");
    let dir = fresh_dir("oku-slides-route-tab-");
    let mut s = help_on(data.path());
    assert_that!(s.resume_fresh(dir.path().to_path_buf())).is_true();
    s.flush_resume();
    let (before, state) = (snapshot_json(&s), std::fs::read(dir.path().join("state.json")).unwrap());
    s.take_preview(ESSENTIEL_PACK).unwrap();
    s.flush_resume();
    let after = (snapshot_json(&s), std::fs::read(dir.path().join("state.json")).unwrap());
    assert_that!(after).is_equal_to((before, state));
}

/// A pack open `open_pack_from_a_tab_that_opens_none_is_refused` tries.
#[derive(Debug, Clone, Copy)]
enum Refused {
    /// `new_pack`'s check, from help.
    HelpNewPack,
    /// A deck-only pack the help deck pins.
    HelpDeckOnly,
    /// The preview of that deck-only pack.
    HelpDeckOnlyPreview,
    /// A pack the help deck does not pin, nor the library holds.
    HelpUnknown,
    /// A library pack, from history.
    History,
    /// A library pack, from a theme tab.
    ThemeTab,
}

#[rstest]
#[case::help_new_pack(Refused::HelpNewPack, SessionErrorKind::ReadOnly)]
#[case::help_deck_only(Refused::HelpDeckOnly, SessionErrorKind::DeckOnly)]
#[case::help_deck_only_preview(Refused::HelpDeckOnlyPreview, SessionErrorKind::DeckOnly)]
#[case::help_unknown(Refused::HelpUnknown, SessionErrorKind::UnknownPack)]
#[case::history(Refused::History, SessionErrorKind::InHistory)]
#[case::theme_tab(Refused::ThemeTab, SessionErrorKind::WrongDocument)]
fn open_pack_from_a_tab_that_opens_none_is_refused(#[case] open: Refused, #[case] kind: SessionErrorKind) {
    let data = fresh_dir("oku-slides-route-");
    let err = match open {
        Refused::HelpNewPack => help_on(data.path()).can_open_pack().unwrap_err(),
        Refused::HelpDeckOnly => {
            let (mut s, pin) = help_pinning_a_deck_only_pack(data.path());
            s.route_pack(pin.id, None).unwrap_err()
        }
        Refused::HelpDeckOnlyPreview => {
            let (mut s, pin) = help_pinning_a_deck_only_pack(data.path());
            s.take_preview(pin.id).unwrap_err()
        }
        Refused::HelpUnknown => help_on(data.path()).route_pack(Uuid::new_v4(), None).unwrap_err(),
        Refused::History => {
            let mut s = session_on(data.path());
            s.handle(Request::HistoryOpen).unwrap();
            s.route_pack(BASE_PACK, None).unwrap_err()
        }
        Refused::ThemeTab => crate::theme_tab::theme_tab_tests::theme_tab(data.path(), crate::theme::PAPIER_THEME)
            .route_pack(BASE_PACK, None)
            .unwrap_err(),
    };
    assert_that!(err.kind()).is_equal_to(kind);
}

#[test]
fn route_pack_of_a_deck_only_pack_from_help_is_refused_in_fr() {
    let data = fresh_dir("oku-slides-route-");
    let (mut s, pin) = help_pinning_a_deck_only_pack(data.path());
    let err = s.route_pack(pin.id, None).unwrap_err();
    let shown = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(shown.as_str()).contains("n'appartient qu'à son deck");
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
