use std::path::PathBuf;

use oku_slides_core::{CapabilityKind, GuideKind, Guides, Op, PackFiles, PackKind, PackPin};
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::guide_approvals::VersionRef;
use crate::pack::PackManifest;
use crate::received_review::{ReceivedRelation, ReceivedReview};
use crate::test_support::{
    adopt_deck_only_pack, files_of, fresh_dir, manifest, manifest_ns, ns, opened_on, session_on,
};
use crate::test_support_packs::{CARD_SRC, LINKED_TAB, deck_pack_tab_of, forward_to_owner};
use crate::{Request, Session, SessionErrorKind};

/// The data folders of a received deck, kept alive with the file.
pub(crate) struct Data {
    /// The receiving user's data folder.
    pub(crate) me: tempfile::TempDir,
    _them: Option<tempfile::TempDir>,
    /// The received file.
    pub(crate) file: Option<PathBuf>,
}

/// A deck file written by another user whose deck pins `packs`, opened by a user who never saw them.
pub(crate) fn received_deck(packs: &[PackFiles]) -> (Data, Session) {
    let (me, them) = (fresh_dir("oku-slides-received-me-"), fresh_dir("oku-slides-received-them-"));
    let mut other = session_on(them.path());
    let ops = packs
        .iter()
        .map(|files| {
            let m = PackManifest::from_files(files).unwrap();
            other.library.add_version(m.id, files).unwrap();
            let pin = PackPin { id: m.id, version: files.version(), requires: Vec::new() };
            Op::AddPack { kind: PackKind::Widgets, namespace: m.namespace, pin }
        })
        .collect();
    other.apply("Packs", ops, None).unwrap();
    let file = them.path().join("received.oku-slides.pdf");
    other.handle(Request::Save { path: Some(file.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    s.open(&file).unwrap();
    (Data { me, _them: Some(them), file: Some(file) }, s)
}

/// The files of a widgets pack `id` under `namespace`, carrying `use_` as use guide.
pub(crate) fn pack_files(id: Uuid, namespace: &str, use_: &str) -> PackFiles {
    let mut m = manifest_ns(id, namespace, &[]);
    m.name = "Brought".into();
    m.guides = Guides { edit: String::new(), use_: use_.to_owned() };
    files_of(&m)
}

/// A deck file whose deck embeds a pack with `use_` as use guide, opened by a user who never saw it.
pub(crate) fn opened_deck_bringing_pack_with_use_guide(use_: &str) -> (Data, Session) {
    let id = Uuid::new_v4();
    let mut m = manifest(id, "Brought", None, "2026-10-01T10:00:00Z");
    m.guides = Guides { edit: String::new(), use_: use_.to_owned() };
    received_deck(&[files_of(&m)])
}

/// The same deck opened again by a new session on the same data folder.
pub(crate) fn reopen(d: &Data) -> Session {
    let mut s = session_on(d.me.path());
    s.open(d.file.as_ref().unwrap()).unwrap();
    s
}

/// A deck with built-in packs only.
fn deck_session() -> (Data, Session) {
    let me = fresh_dir("oku-slides-install-me-");
    let s = opened_on(me.path());
    (Data { me, _them: None, file: None }, s)
}

/// A deck pinning a pack of the user's own library.
fn deck_with_local_pack() -> (Data, Session) {
    let me = fresh_dir("oku-slides-install-me-");
    let mut s = opened_on(me.path());
    let id = Uuid::new_v4();
    let files = files_of(&manifest(id, "Mine", None, "2026-10-01T10:00:00Z"));
    s.library.add_version(id, &files).unwrap();
    s.doc.sources.add_pack(files.clone());
    s.apply("Pack", vec![crate::deck_packs::deck_packs_tests::pin_op(id, &files)], None).unwrap();
    (Data { me, _them: None, file: None }, s)
}

/// A received deck whose own deck-only pack carries no guide.
fn received_deck_only_pack() -> (Data, Session) {
    let (me, them) = (fresh_dir("oku-slides-install-me-"), fresh_dir("oku-slides-install-them-"));
    let mut other = opened_on(them.path());
    adopt_deck_only_pack(&mut other);
    let file = them.path().join("imported.oku-slides.pdf");
    other.handle(Request::Save { path: Some(file.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    s.open(&file).unwrap();
    (Data { me, _them: Some(them), file: Some(file) }, s)
}

pub(crate) fn refs(reviews: &[ReceivedReview]) -> Vec<VersionRef> {
    reviews.iter().map(ReceivedReview::version_ref).collect()
}

#[test]
fn open_deck_with_new_library_pack_leaves_library_unchanged() {
    let id = Uuid::new_v4();
    let (_d, s) = received_deck(&[pack_files(id, "brought", "")]);
    assert_that!(s.library.versions(id)).is_empty();
    assert_that!(s.library.knows(id)).is_false();
}

#[test]
fn received_to_review_lists_new_pack_with_capabilities() {
    let id = Uuid::new_v4();
    let (_d, s) = received_deck(&[pack_files(id, "brought", "")]);
    let review = s.snapshot().received_to_review;
    let seen: Vec<_> = review
        .iter()
        .map(|r| (r.id, r.capabilities.kind, r.capabilities.name.as_str(), r.relation, r.install, r.blocker.clone()))
        .collect();
    assert_that!(seen).is_equal_to(vec![(
        id,
        CapabilityKind::Widgets,
        "Brought",
        ReceivedRelation::NewPack,
        true,
        None,
    )]);
    assert_that!(review[0].guides.is_empty()).is_true();
}

#[test]
fn received_to_review_of_new_version_of_known_pack_is_new_version() {
    let id = Uuid::new_v4();
    let theirs = pack_files(id, "brought", "");
    let (d, _s) = received_deck(std::slice::from_ref(&theirs));
    let mut m = PackManifest::from_files(&theirs).unwrap();
    m.created = "2026-09-01T10:00:00Z".into();
    session_on(d.me.path()).library.add_version(id, &files_of(&m)).unwrap();
    let s = reopen(&d);
    let review = s.snapshot().received_to_review;
    assert_that!(review.iter().map(|r| (r.relation, r.install)).collect::<Vec<_>>())
        .is_equal_to(vec![(ReceivedRelation::NewVersion, true)]);
}

/// A user whose library has another pack under `namespace`, opening a deck that brings `namespace` too.
fn received_with_taken_namespace(namespace: &str, other_id: Uuid) -> (Data, Session, Uuid) {
    let (d, _s) = received_deck(&[pack_files(other_id, namespace, ""), pack_files(Uuid::new_v4(), "fresh", "")]);
    let mine = Uuid::new_v4();
    session_on(d.me.path()).library.add_version(mine, &pack_files(mine, namespace, "")).unwrap();
    let s = reopen(&d);
    (d, s, mine)
}

#[test]
fn received_to_review_with_taken_namespace_has_blocker_and_no_install() {
    let theirs = Uuid::new_v4();
    let (_d, s, _mine) = received_with_taken_namespace("shared", theirs);
    let review = s.snapshot().received_to_review;
    let blocked = review.iter().find(|r| r.id == theirs).unwrap();
    let expected = s.localizer.tr_with("received-blocker-namespace-taken", &[("namespace", "shared".into())]);
    assert_that!((blocked.install, blocked.blocker.clone())).is_equal_to((false, Some(expected)));
}

#[test]
fn received_blocker_namespace_taken_is_translated_in_french() {
    let args = [("namespace", "shared".into())];
    let fr = Localizer::new(Some("fr")).tr_with("received-blocker-namespace-taken", &args);
    let en = Localizer::new(Some("en")).tr_with("received-blocker-namespace-taken", &args);
    assert_that!(fr).is_not_equal_to(en);
    assert_that!(fr.contains("shared")).is_true();
}

#[test]
fn received_to_review_of_kind_mismatch_is_unreadable_without_install() {
    let id = Uuid::new_v4();
    let (d, _s) = received_deck(&[pack_files(id, "brought", "")]);
    let mut s = reopen(&d);
    // The content claims a theme where the deck holds a widgets pack: never trusted.
    let pin = s.deck().widgets.values().find(|p| p.id == id).cloned().unwrap();
    let mut manifest = s.doc.sources.packs[&pin.version].manifest().clone();
    manifest.insert("tokens".into(), serde_json::json!({ "color": {} }));
    let forged = PackFiles::new(PackKind::Widgets, manifest, std::collections::BTreeMap::new());
    s.doc.sources.packs.insert(pin.version.clone(), forged);
    let review = s.received_to_review();
    let expected = s.localizer.tr("received-blocker-unreadable");
    assert_that!(review.iter().map(|r| (r.install, r.blocker.clone())).collect::<Vec<_>>())
        .is_equal_to(vec![(false, Some(expected))]);
}

#[rstest]
#[case::builtin(deck_session())]
#[case::local(deck_with_local_pack())]
#[case::deck_only(received_deck_only_pack())]
fn received_to_review_never_lists_install_of(#[case] fixture: (Data, Session)) {
    let installs: Vec<_> = fixture.1.received_to_review().into_iter().filter(|r| r.install).collect();
    assert_that!(installs).is_empty();
}

#[test]
fn received_to_review_outside_deck_tab_is_empty() {
    let (_d, mut s) = received_deck(&[pack_files(Uuid::new_v4(), "brought", "use me")]);
    s.handle(Request::HistoryOpen).unwrap();
    assert_that!(s.snapshot().received_to_review).is_empty();
}

#[test]
fn open_pack_tab_on_not_installed_received_version_is_refused() {
    let id = Uuid::new_v4();
    let (_d, mut s) = received_deck(&[pack_files(id, "brought", "")]);
    let err = s.route_pack(id, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackNotInstalled);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr).is_not_equal_to(en);
    assert_that!(fr.contains("Brought")).is_true();
}

/// A widget written into a received pack never installed would put the pack in the library.
#[test]
fn set_widget_on_not_installed_received_pack_is_refused_and_library_unchanged() {
    let id = Uuid::new_v4();
    let (_d, mut s) = received_deck(&[pack_files(id, "brought", "")]);
    let err = s.set_widget(Some(&ns("brought")), "card", CARD_SRC.into()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackNotInstalled);
    assert_that!(s.library.knows(id)).is_false();
}

#[test]
fn received_to_review_lists_a_received_version_with_guides() {
    let (_d, s) = opened_deck_bringing_pack_with_use_guide("use me");
    let review = s.snapshot().received_to_review;
    assert_that!(review.iter().map(|r| r.guides.use_.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["use me"]);
}

/// M1: the dialog shows what the agent would read: hidden controls dropped, cut at the limit.
#[test]
fn received_to_review_holds_the_text_the_agent_reads() {
    let hostile = format!("  a\u{202E}b{}", "x".repeat(5000));
    let (_d, s) = opened_deck_bringing_pack_with_use_guide(&hostile);
    let review = s.snapshot().received_to_review;
    let shown = review.first().map(|r| r.guides.use_.clone()).unwrap_or_default();
    assert_that!(shown.contains('\u{202E}')).is_false();
    assert_that!(shown.starts_with("ab")).is_true();
    assert_that!((shown.chars().count(), shown.ends_with('…'))).is_equal_to((s.guide_limit, true));
}

#[test]
fn received_to_review_cleans_the_name_of_controls() {
    let id = Uuid::new_v4();
    let mut m = manifest(id, "Bro\u{202E}ught", None, "2026-10-01T10:00:00Z");
    m.guides = Guides::default();
    let (_d, s) = received_deck(&[files_of(&m)]);
    let review = s.snapshot().received_to_review;
    assert_that!(review.iter().map(|r| r.capabilities.name.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["Brought"]);
}

#[test]
fn later_received_empties_the_list_for_this_tab() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let versions = refs(&s.snapshot().received_to_review);
    s.handle(Request::LaterReceived { versions }).unwrap();
    assert_that!(s.snapshot().received_to_review).is_empty();
}

#[test]
fn approve_guides_empties_the_guides_for_good() {
    let (d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let versions = refs(&s.snapshot().received_to_review);
    s.handle(Request::ApproveGuides { versions }).unwrap();
    let again = reopen(&d).snapshot().received_to_review;
    assert_that!(again.iter().map(|r| (r.guides.is_empty(), r.install)).collect::<Vec<_>>())
        .is_equal_to(vec![(true, true)]);
}

#[test]
fn approve_guides_does_not_change_the_deck() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let (before, versions) = (s.snapshot().can_undo, refs(&s.snapshot().received_to_review));
    s.handle(Request::ApproveGuides { versions }).unwrap();
    assert_that!((before, s.snapshot().can_undo, s.snapshot().dirty)).is_equal_to((false, false, false));
}

#[rstest]
#[case::builtin_only(deck_session())]
#[case::local(deck_with_local_pack())]
fn received_to_review_skips_what_needs_no_review(#[case] fixture: (Data, Session)) {
    assert_that!(fixture.1.snapshot().received_to_review).is_empty();
}

#[test]
fn received_to_review_lists_a_pinned_theme_held_only_by_the_deck() {
    let me = fresh_dir("oku-slides-install-me-");
    let mut s = opened_on(me.path());
    let id = Uuid::new_v4();
    let manifest = crate::ThemeManifest {
        id,
        name: "Brought".into(),
        namespace: crate::test_support::ns("brought"),
        extends: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: serde_json::Map::new(),
        guides: Guides { edit: "edit it".into(), use_: String::new() },
    };
    let files = manifest.to_files().unwrap();
    let pin = oku_slides_core::ThemePin { id, version: files.version(), extends: Vec::new() };
    s.doc.sources.add_theme(files);
    let op = oku_slides_core::Op::SetTheme {
        theme: Some(manifest.namespace.clone()),
        themes: std::collections::BTreeMap::from([(manifest.namespace.clone(), pin)]),
    };
    s.apply("Theme", vec![op], None).unwrap();
    let review = s.snapshot().received_to_review;
    assert_that!(review.iter().map(|r| (r.capabilities.kind, r.guides.edit.as_str())).collect::<Vec<_>>())
        .is_equal_to(vec![(CapabilityKind::Theme, "edit it")]);
}

#[test]
fn received_to_review_skips_the_version_the_users_own_deck_pack_tab_wrote() {
    let data = fresh_dir("oku-slides-install-own-");
    let mut owner = opened_on(data.path());
    let pin = adopt_deck_only_pack(&mut owner);
    let mut tab = deck_pack_tab_of(&mut owner, data.path(), pin.id);
    tab.set_guide(GuideKind::Use, "mine".into()).unwrap();
    tab.handle(Request::SavePack { as_new: false }).unwrap();
    forward_to_owner(&mut tab, LINKED_TAB, &mut owner).unwrap();
    let carries_guide = owner.held_versions().iter().any(|h| h.id == pin.id && !super::guides_of(h.files).is_empty());
    assert_that!(carries_guide).is_true();
    assert_that!(owner.snapshot().received_to_review).is_empty();
    assert_that!(tab.snapshot().guides.unwrap().approved).is_true();
}

/// A deck whose own deck pack carries `use_` as use guide, written elsewhere and opened here.
fn opened_deck_with_deck_pack_guide(use_: &str) -> (Data, Session) {
    let (me, them) = (fresh_dir("oku-slides-install-me-"), fresh_dir("oku-slides-install-them-"));
    let mut owner = opened_on(them.path());
    let pin = adopt_deck_only_pack(&mut owner);
    let mut tab = deck_pack_tab_of(&mut owner, them.path(), pin.id);
    tab.set_guide(GuideKind::Use, use_.into()).unwrap();
    tab.handle(Request::SavePack { as_new: false }).unwrap();
    forward_to_owner(&mut tab, LINKED_TAB, &mut owner).unwrap();
    let file = them.path().join("hostile.oku-slides.pdf");
    owner.handle(Request::Save { path: Some(file.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    s.open(&file).unwrap();
    (Data { me, _them: Some(them), file: Some(file) }, s)
}

/// The deck-mode widget write an agent's slide edit goes through: a new version of the deck pack.
fn write_deck_widget(s: &mut Session) {
    s.set_widget(Some(&oku_slides_core::Namespace::deck()), "card", CARD_SRC.into()).unwrap();
    s.end_widget_session();
}

#[test]
fn deck_widget_write_on_a_received_deck_pack_keeps_its_guide_pending() {
    let (_d, mut s) = opened_deck_with_deck_pack_guide("hostile");
    let before = s.snapshot().received_to_review;
    write_deck_widget(&mut s);
    let after = s.snapshot().received_to_review;
    assert_that!(before.iter().map(|r| r.guides.use_.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["hostile"]);
    assert_that!(after.iter().map(|r| r.guides.use_.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["hostile"]);
    assert_that!(after[0].version == before[0].version).is_false();
}

#[test]
fn deck_widget_write_after_approval_approves_the_new_version() {
    let (_d, mut s) = opened_deck_with_deck_pack_guide("hostile");
    let versions = refs(&s.snapshot().received_to_review);
    s.handle(Request::ApproveGuides { versions }).unwrap();
    write_deck_widget(&mut s);
    assert_that!(s.snapshot().received_to_review).is_empty();
}

#[test]
fn approve_guides_of_the_received_version_after_a_deck_widget_write_approves_the_written_one() {
    let (_d, mut s) = opened_deck_with_deck_pack_guide("hostile");
    let original = refs(&s.snapshot().received_to_review);
    write_deck_widget(&mut s);
    s.handle(Request::ApproveGuides { versions: original }).unwrap();
    assert_that!(s.snapshot().received_to_review).is_empty();
}

/// A deck pinning a theme the library only received, which carries a use guide.
fn deck_pinning_received_theme() -> (tempfile::TempDir, Session, Uuid) {
    use crate::library_kind::LibraryKind;
    let me = fresh_dir("oku-slides-install-me-");
    let mut s = opened_on(me.path());
    let pro = crate::theme::Themes::builtins().iter().find(|b| b.manifest.id == crate::theme::PRO_THEME).unwrap();
    let id = Uuid::new_v4();
    let manifest = crate::ThemeManifest {
        id,
        name: "Brought".into(),
        namespace: crate::test_support::ns("brought"),
        extends: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: pro.manifest.tokens.clone(),
        guides: Guides { edit: String::new(), use_: "hostile".into() },
    };
    let files = manifest.to_files().unwrap();
    s.themes.receive_version(id, &files).unwrap();
    let pin = oku_slides_core::ThemePin { id, version: files.version(), extends: Vec::new() };
    s.doc.sources.add_theme(files);
    let themes = std::collections::BTreeMap::from([(manifest.namespace.clone(), pin)]);
    s.apply("Theme", vec![oku_slides_core::Op::SetTheme { theme: Some(manifest.namespace), themes }], None).unwrap();
    (me, s, id)
}

fn edit_accent(s: &mut Session, id: Uuid) -> crate::EditedTheme {
    let op = crate::TokenOp::SetToken { path: vec!["color".into(), "accent".into()], value: "#c0392b".into() };
    s.edit_theme(id, None, vec![op]).unwrap()
}

#[test]
fn token_edit_of_a_received_theme_keeps_its_guide_received_and_pending() {
    let (_d, mut s, id) = deck_pinning_received_theme();
    let edited = edit_accent(&mut s, id);
    let review = s.snapshot().received_to_review;
    assert_that!(s.guide_origin(edited.id, &edited.version, true)).is_equal_to(crate::GuideOrigin::Received);
    assert_that!(review.iter().map(|r| (r.version == edited.version, r.guides.use_.as_str())).collect::<Vec<_>>())
        .is_equal_to(vec![(true, "hostile")]);
}

#[test]
fn token_edit_of_an_approved_received_theme_is_active() {
    let (_d, mut s, id) = deck_pinning_received_theme();
    let versions = refs(&s.snapshot().received_to_review);
    s.handle(Request::ApproveGuides { versions }).unwrap();
    let edited = edit_accent(&mut s, id);
    assert_that!(s.guide_active(edited.id, &edited.version, true)).is_true();
    assert_that!(s.snapshot().received_to_review).is_empty();
}

#[test]
fn pack_tab_saving_a_received_library_pack_keeps_its_unchanged_guide_received() {
    let me = fresh_dir("oku-slides-install-me-");
    let id = Uuid::new_v4();
    let mut m = manifest(id, "Brought", None, "2026-10-01T10:00:00Z");
    m.guides = Guides { edit: String::new(), use_: "hostile".into() };
    let files = files_of(&m);
    session_on(me.path()).library.receive_version(id, &files).unwrap();
    let mut tab = crate::test_support_packs::pack_tab_of(me.path(), id);
    tab.set_widget(None, "card", CARD_SRC.into()).unwrap();
    let view = tab.snapshot().guides.unwrap();
    assert_that!(view.version == files.version()).is_false();
    assert_that!((view.origin, view.guides.use_.as_str())).is_equal_to((crate::GuideOrigin::Received, "hostile"));
}

#[test]
fn approve_guides_of_another_version_with_other_guides_leaves_the_pinned_one_pending() {
    let (_d, mut s) = opened_deck_with_deck_pack_guide("hostile");
    let pinned = refs(&s.snapshot().received_to_review);
    let files = s.doc.sources.packs[&pinned[0].version].clone();
    let mut m = files.manifest().clone();
    m.insert("guides".into(), serde_json::json!({ "use": "other" }));
    let other = PackFiles::new(files.kind(), m, files.widgets().clone());
    s.doc.sources.add_pack(other.clone());
    let versions = vec![VersionRef { id: pinned[0].id, version: other.version() }];
    s.handle(Request::ApproveGuides { versions }).unwrap();
    assert_that!(refs(&s.snapshot().received_to_review)).is_equal_to(pinned);
}

/// Version `name` of theme `id` with use guide `use_`, received into the theme library.
fn received_theme(s: &Session, id: Uuid, name: &str, use_: &str) -> PackFiles {
    let manifest = crate::ThemeManifest {
        id,
        name: name.into(),
        namespace: ns("brought"),
        extends: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: serde_json::Map::new(),
        guides: Guides { edit: String::new(), use_: use_.into() },
    };
    let files = manifest.to_files().unwrap();
    s.themes.receive_version(id, &files).unwrap();
    files
}

#[test]
fn approve_guides_of_a_received_theme_approves_its_received_versions_with_the_same_guides() {
    let me = fresh_dir("oku-slides-install-me-");
    let mut s = opened_on(me.path());
    let id = Uuid::new_v4();
    let original = received_theme(&s, id, "Brought", "use me");
    let edited = received_theme(&s, id, "Brought, edited", "use me");
    let other = received_theme(&s, id, "Brought, other", "obey me");
    s.handle(Request::ApproveGuides { versions: vec![VersionRef { id, version: original.version() }] }).unwrap();
    let active = (s.guide_active(id, &edited.version(), true), s.guide_active(id, &other.version(), true));
    assert_that!(active).is_equal_to((true, false));
}
