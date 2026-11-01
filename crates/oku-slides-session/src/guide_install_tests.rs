use std::path::PathBuf;

use oku_slides_core::{GuideKind, Guides};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::deck_packs::deck_packs_tests::received_file;
use crate::guide_approvals::VersionRef;
use crate::guide_install::GuidesReview;
use crate::test_support::{adopt_deck_only_pack, files_of, fresh_dir, manifest, opened_on, session_on};
use crate::test_support_packs::{CARD_SRC, LINKED_TAB, deck_pack_tab_of, forward_to_owner};
use crate::{Request, Session};

/// The data folders of a received deck, kept alive with the file.
struct Data {
    me: tempfile::TempDir,
    _them: Option<tempfile::TempDir>,
    file: Option<PathBuf>,
}

/// A deck file whose deck embeds a pack with `use_` as use guide, opened by a user who never saw it.
fn opened_deck_bringing_pack_with_use_guide(use_: &str) -> (Data, Session) {
    let (me, them) = (fresh_dir("oku-slides-install-me-"), fresh_dir("oku-slides-install-them-"));
    let id = Uuid::new_v4();
    let mut m = manifest(id, "Brought", None, "2026-10-01T10:00:00Z");
    m.guides = Guides { edit: String::new(), use_: use_.to_owned() };
    let file = received_file(them.path(), &files_of(&m));
    let mut s = session_on(me.path());
    s.open(&file).unwrap();
    (Data { me, _them: Some(them), file: Some(file) }, s)
}

/// The same deck opened again by a new session on the same data folder.
fn reopen(d: &Data) -> Session {
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

fn refs(reviews: &[GuidesReview]) -> Vec<VersionRef> {
    reviews.iter().map(|r| VersionRef { id: r.id, version: r.version.clone() }).collect()
}

#[test]
fn guides_to_approve_lists_a_received_version_with_guides() {
    let (_d, s) = opened_deck_bringing_pack_with_use_guide("use me");
    let review = s.snapshot().guides_to_approve;
    assert_that!(review.iter().map(|r| r.guides.use_.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["use me"]);
}

/// M1: the dialog shows what the agent would read: hidden controls dropped, cut at the limit.
#[test]
fn guides_to_approve_holds_the_text_the_agent_reads() {
    let hostile = format!("  a\u{202E}b{}", "x".repeat(5000));
    let (_d, s) = opened_deck_bringing_pack_with_use_guide(&hostile);
    let review = s.snapshot().guides_to_approve;
    let shown = review.first().map(|r| r.guides.use_.clone()).unwrap_or_default();
    assert_that!(shown.contains('\u{202E}')).is_false();
    assert_that!(shown.starts_with("ab")).is_true();
    assert_that!((shown.chars().count(), shown.ends_with('…'))).is_equal_to((s.guide_limit, true));
}

#[test]
fn guides_to_approve_describes_what_the_version_holds() {
    let (_d, s) = opened_deck_bringing_pack_with_use_guide("use me");
    let review = s.snapshot().guides_to_approve;
    assert_that!(review.iter().map(|r| (r.kind, r.name.as_str(), r.items)).collect::<Vec<_>>())
        .is_equal_to(vec![("widgets", "Brought", 0)]);
}

#[test]
fn later_guides_empties_the_list_for_this_tab() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let versions = refs(&s.snapshot().guides_to_approve);
    s.handle(Request::LaterGuides { versions }).unwrap();
    assert_that!(s.snapshot().guides_to_approve).is_empty();
}

#[test]
fn later_guides_asks_again_after_reopening() {
    let (d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let versions = refs(&s.snapshot().guides_to_approve);
    s.handle(Request::LaterGuides { versions }).unwrap();
    let again = reopen(&d);
    assert_that!(again.snapshot().guides_to_approve).has_length(1);
}

#[test]
fn approve_guides_empties_the_list_for_good() {
    let (d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let versions = refs(&s.snapshot().guides_to_approve);
    s.handle(Request::ApproveGuides { versions }).unwrap();
    assert_that!(reopen(&d).snapshot().guides_to_approve).is_empty();
}

#[test]
fn approve_guides_does_not_change_the_deck() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let (before, versions) = (s.snapshot().can_undo, refs(&s.snapshot().guides_to_approve));
    s.handle(Request::ApproveGuides { versions }).unwrap();
    assert_that!((before, s.snapshot().can_undo, s.snapshot().dirty)).is_equal_to((false, false, false));
}

#[rstest]
#[case::no_guides(opened_deck_bringing_pack_with_use_guide(""))]
#[case::builtin_only(deck_session())]
fn guides_to_approve_skips_what_needs_no_approval(#[case] fixture: (Data, Session)) {
    assert_that!(fixture.1.snapshot().guides_to_approve).is_empty();
}

#[test]
fn guides_to_approve_skips_a_local_version() {
    let me = fresh_dir("oku-slides-install-me-");
    let mut s = opened_on(me.path());
    let id = Uuid::new_v4();
    let mut m = manifest(id, "Mine", None, "2026-10-01T10:00:00Z");
    m.guides.set(GuideKind::Use, "mine".into());
    let files = files_of(&m);
    s.library.add_version(id, &files).unwrap();
    s.doc.sources.add_pack(files.clone());
    s.apply("Pack", vec![crate::deck_packs::deck_packs_tests::pin_op(id, &files)], None).unwrap();
    assert_that!(s.snapshot().guides_to_approve).is_empty();
}

#[test]
fn guides_to_approve_lists_a_pinned_theme_held_only_by_the_deck() {
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
    let review = s.snapshot().guides_to_approve;
    assert_that!(review.iter().map(|r| (r.kind, r.guides.edit.as_str())).collect::<Vec<_>>())
        .is_equal_to(vec![("theme", "edit it")]);
}

#[test]
fn guides_to_approve_skips_the_version_the_users_own_deck_pack_tab_wrote() {
    let data = fresh_dir("oku-slides-install-own-");
    let mut owner = opened_on(data.path());
    let pin = adopt_deck_only_pack(&mut owner);
    let mut tab = deck_pack_tab_of(&mut owner, data.path(), pin.id);
    tab.set_guide(GuideKind::Use, "mine".into()).unwrap();
    tab.handle(Request::SavePack { as_new: false }).unwrap();
    forward_to_owner(&mut tab, LINKED_TAB, &mut owner).unwrap();
    let carries_guide = owner.held_versions().iter().any(|h| h.id == pin.id && !super::guides_of(h.files).is_empty());
    assert_that!(carries_guide).is_true();
    assert_that!(owner.snapshot().guides_to_approve).is_empty();
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
    let before = s.snapshot().guides_to_approve;
    write_deck_widget(&mut s);
    let after = s.snapshot().guides_to_approve;
    assert_that!(before.iter().map(|r| r.guides.use_.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["hostile"]);
    assert_that!(after.iter().map(|r| r.guides.use_.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["hostile"]);
    assert_that!(after[0].version == before[0].version).is_false();
}

#[test]
fn deck_widget_write_after_approval_approves_the_new_version() {
    let (_d, mut s) = opened_deck_with_deck_pack_guide("hostile");
    let versions = refs(&s.snapshot().guides_to_approve);
    s.handle(Request::ApproveGuides { versions }).unwrap();
    write_deck_widget(&mut s);
    assert_that!(s.snapshot().guides_to_approve).is_empty();
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
    let review = s.snapshot().guides_to_approve;
    assert_that!(s.guide_origin(edited.id, &edited.version, true)).is_equal_to(crate::GuideOrigin::Received);
    assert_that!(review.iter().map(|r| (r.version == edited.version, r.guides.use_.as_str())).collect::<Vec<_>>())
        .is_equal_to(vec![(true, "hostile")]);
}

#[test]
fn token_edit_of_an_approved_received_theme_is_active() {
    let (_d, mut s, id) = deck_pinning_received_theme();
    let versions = refs(&s.snapshot().guides_to_approve);
    s.handle(Request::ApproveGuides { versions }).unwrap();
    let edited = edit_accent(&mut s, id);
    assert_that!(s.guide_active(edited.id, &edited.version, true)).is_true();
    assert_that!(s.snapshot().guides_to_approve).is_empty();
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
