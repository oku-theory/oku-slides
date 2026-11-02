//! The received reviews' cache, the cut capability lists, and the write gate on received versions.

use oku_slides_core::{Capabilities, CapabilityKind, PackKind};
use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::pack::PackManifest;
use crate::received_review::received_review_tests::{
    opened_deck_bringing_pack_with_use_guide, pack_files, received_deck, refs, reopen,
};
use crate::received_review::{CapabilityTotals, SHOWN_NAMES, cleaned};
use crate::test_support::{files_of, ns, session_on};
use crate::test_support_packs::CARD_SRC;
use crate::{Request, SessionErrorKind};

/// `count` names `prefix0`, `prefix1`…
fn names(prefix: &str, count: usize) -> Vec<String> {
    (0..count).map(|i| format!("{prefix}{i}")).collect()
}

#[test]
fn cleaned_with_long_lists_shows_the_first_names_and_counts_them_all() {
    let caps = Capabilities {
        kind: CapabilityKind::Widgets,
        name: "Big".into(),
        namespace: "big".into(),
        description: String::new(),
        items: names("w", 60),
        requires: names("r", 51),
        tokens: names("t", 50),
        fonts: names("f", 2),
        profiles: 0,
        code_bytes: 0,
    };
    let (shown, totals) = cleaned(&caps);
    let lengths = (shown.items.len(), shown.requires.len(), shown.tokens.len(), shown.fonts.len());
    assert_that!(lengths).is_equal_to((SHOWN_NAMES, SHOWN_NAMES, SHOWN_NAMES, 2));
    assert_that!(totals).is_equal_to(CapabilityTotals { items: 60, requires: 51, tokens: 50, fonts: 2 });
    assert_that!(shown.items.first().map(String::as_str)).is_equal_to(Some("w0"));
}

#[test]
fn snapshot_received_review_serializes_its_totals() {
    let (_d, s) = received_deck(&[pack_files(Uuid::new_v4(), "brought", "")]);
    let json = serde_json::to_value(s.snapshot()).unwrap();
    let totals = json["received_to_review"][0]["totals"].clone();
    assert_that!(totals).is_equal_to(serde_json::json!({ "items": 0, "requires": 0, "tokens": 0, "fonts": 0 }));
}

#[test]
fn received_reviews_with_unchanged_key_reads_no_file_again() {
    let id = Uuid::new_v4();
    let (_d, mut s) = received_deck(&[pack_files(id, "brought", "")]);
    let before = s.received_to_review();
    // Files changed under the same version, which never happens outside a test: the cache answers.
    let pin = s.deck().widgets.values().find(|p| p.id == id).cloned().unwrap();
    let forged = oku_slides_core::PackFiles::new(
        PackKind::Widgets,
        serde_json::Map::new(),
        std::collections::BTreeMap::default(),
    );
    s.doc.sources.packs.insert(pin.version, forged);
    let after = s.received_to_review();
    assert_that!(after.iter().map(|r| r.blocker.clone()).collect::<Vec<_>>())
        .is_equal_to(before.iter().map(|r| r.blocker.clone()).collect::<Vec<_>>());
}

#[test]
fn received_reviews_after_accept_drop_the_installed_version() {
    let (_d, mut s) = received_deck(&[pack_files(Uuid::new_v4(), "brought", "")]);
    let versions = refs(&s.received_to_review());
    s.handle(Request::AcceptReceived { versions }).unwrap();
    assert_that!(s.received_reviews().is_empty()).is_true();
}

#[test]
fn received_reviews_after_later_keep_the_version_for_install_paths() {
    let (_d, mut s) = received_deck(&[pack_files(Uuid::new_v4(), "brought", "")]);
    let versions = refs(&s.received_to_review());
    s.handle(Request::LaterReceived { versions }).unwrap();
    assert_that!((s.received_to_review().len(), s.received_reviews().len())).is_equal_to((0, 1));
}

#[test]
fn received_reviews_after_library_write_elsewhere_show_the_new_blocker() {
    let theirs = Uuid::new_v4();
    let (d, s) = received_deck(&[pack_files(theirs, "shared", "")]);
    assert_that!(s.received_to_review()[0].install).is_true();
    let mine = Uuid::new_v4();
    session_on(d.me.path()).library.add_version(mine, &pack_files(mine, "shared", "")).unwrap();
    let review = s.received_to_review();
    assert_that!((review[0].install, review[0].blocker.is_some())).is_equal_to((false, true));
}

#[test]
fn received_reviews_after_approval_elsewhere_hold_no_guides() {
    let (d, s) = opened_deck_bringing_pack_with_use_guide("use me");
    let versions = refs(&s.received_to_review());
    let mut other = reopen(&d);
    other.handle(Request::ApproveGuides { versions }).unwrap();
    assert_that!(s.received_to_review().iter().all(|r| r.guides.is_empty())).is_true();
}

#[test]
fn name_of_with_empty_name_is_the_namespace() {
    let mut m = PackManifest::from_files(&pack_files(Uuid::new_v4(), "brought", "")).unwrap();
    m.name = " ".into();
    assert_that!(crate::received_review::name_of(&files_of(&m))).is_equal_to("brought".to_owned());
}

/// A deck bringing a new version of a pack the library knows, under namespace `brought`.
fn received_new_version() -> (crate::received_review::received_review_tests::Data, crate::Session, Uuid) {
    let id = Uuid::new_v4();
    let theirs = pack_files(id, "brought", "");
    let (d, _s) = received_deck(std::slice::from_ref(&theirs));
    let mut m = PackManifest::from_files(&theirs).unwrap();
    m.created = "2026-09-01T10:00:00Z".into();
    session_on(d.me.path()).library.add_version(id, &files_of(&m)).unwrap();
    let s = reopen(&d);
    (d, s, id)
}

#[test]
fn set_widget_on_received_new_version_is_refused_and_current_unchanged() {
    let (_d, mut s, id) = received_new_version();
    let current = s.library.current(id);
    let err = s.set_widget(Some(&ns("brought")), "card", CARD_SRC.into()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackNotInstalled);
    assert_that!((s.library.current(id), s.library.versions(id).len())).is_equal_to((current, 1));
}

#[test]
fn set_widget_on_received_new_version_is_refused_in_french_by_its_key() {
    let (_d, mut s, _id) = received_new_version();
    let err = s.set_widget(Some(&ns("brought")), "card", CARD_SRC.into()).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with("session-pack-not-installed", &[("pack", "Brought".into())]);
    assert_that!(err.localize(&fr)).is_equal_to(expected);
}

#[test]
fn set_widget_on_installed_received_new_version_is_written() {
    let (_d, mut s, id) = received_new_version();
    let versions = refs(&s.received_to_review());
    s.handle(Request::AcceptReceived { versions }).unwrap();
    let written = s.set_widget(Some(&ns("brought")), "card", CARD_SRC.into());
    assert_that!(written.is_ok()).is_true();
    assert_that!(s.library.versions(id).len()).is_equal_to(3);
}
