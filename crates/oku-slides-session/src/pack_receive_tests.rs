use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use oku_slides_core::{Guides, Op, PackFiles, PackKind, ThemePin};
use speculoos::prelude::*;
use uuid::Uuid;

use oku_slides_i18n::Localizer;

use crate::deck_packs::deck_packs_tests::{files, pin_op, pinned_version, received_file};
use crate::guide_approvals::VersionRef;
use crate::library_kind::LibraryKind;
use crate::pack::PackManifest;
use crate::received_review::received_review_tests::{
    opened_deck_bringing_pack_with_use_guide, pack_files, received_deck, refs, reopen,
};
use crate::test_support::{adopt_deck_only_pack, fresh_dir, logged, ns, opened_on, pin, session_on};
use crate::{Request, Session, SessionErrorKind};

fn accept(s: &mut Session, versions: Vec<VersionRef>) {
    s.handle(Request::AcceptReceived { versions }).unwrap();
}

#[test]
fn accept_received_of_new_version_keeps_current_and_marks_it_received() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.versions(id).len()).is_equal_to(1);
    accept(&mut s, vec![VersionRef { id, version: v1.version() }]);
    assert_that!(s.library.versions(id).len()).is_equal_to(2);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
    // t-355: the received version is marked, mine is not.
    assert_that!((s.library.is_received(id, &v0.version()), s.library.is_received(id, &v1.version())))
        .is_equal_to((false, true));
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v1.version()));
}

#[test]
fn accept_received_of_unknown_pack_creates_it_with_the_received_version_current() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "Nouveau", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.open(&received_file(them.path(), &v)).unwrap();
    accept(&mut s, vec![VersionRef { id, version: v.version() }]);
    assert_that!(s.library.current(id)).is_equal_to(Some(v.version()));
    assert_that!(s.pack_updates()).is_equal_to(0);
    assert_that!(s.received_to_review()).is_empty();
}

#[test]
fn accept_received_installs_pack_marked_received_and_approves_guides() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("use me");
    let review = s.received_to_review();
    accept(&mut s, refs(&review));
    let (id, version) = (review[0].id, review[0].version.clone());
    assert_that!(s.library.is_received(id, &version)).is_true();
    assert_that!(s.guide_active(id, &version, false)).is_true();
    assert_that!(s.received_to_review()).is_empty();
}

#[test]
fn accept_received_of_unpinned_version_is_refused() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("");
    let version = files(Uuid::new_v4(), "F", None, "2026-10-01T10:00:00Z").version();
    let forged = VersionRef { id: Uuid::new_v4(), version };
    let err = s.handle(Request::AcceptReceived { versions: vec![forged] }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotHeld);
}

/// Another version of a pack the deck holds: refused by the pack's name, never its id.
#[test]
fn accept_received_of_other_version_of_held_pack_names_the_pack_in_french() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("");
    let id = s.received_to_review()[0].id;
    let version = files(id, "F", None, "2026-09-01T10:00:00Z").version();
    let err = s.handle(Request::AcceptReceived { versions: vec![VersionRef { id, version }] }).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with("session-version-not-held", &[("pack", "Brought".into())]);
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotHeld);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr)).is_equal_to(expected);
}

#[test]
fn later_received_of_version_the_deck_does_not_hold_is_not_kept() {
    let (_d, mut s) = opened_deck_bringing_pack_with_use_guide("");
    let forged =
        VersionRef { id: Uuid::new_v4(), version: files(Uuid::new_v4(), "F", None, "2026-10-01T10:00:00Z").version() };
    s.handle(Request::LaterReceived { versions: vec![forged.clone()] }).unwrap();
    assert_that!(s.received_later.contains(&forged)).is_false();
}

#[test]
fn accept_received_in_a_theme_tab_is_refused() {
    let dir = fresh_dir("oku-slides-receive-theme-tab-");
    let (_s, id) = crate::theme_tab::theme_tab_tests::with_maison(dir.path());
    let mut tab = crate::theme_tab::theme_tab_tests::theme_tab(dir.path(), id);
    let err = tab.handle(Request::AcceptReceived { versions: Vec::new() }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WrongDocument);
}

#[test]
fn later_received_empties_list_for_tab_and_deck_still_renders() {
    let id = Uuid::new_v4();
    let (_d, mut s) = received_deck(&[pack_files(id, "brought", "")]);
    let versions = refs(&s.received_to_review());
    s.handle(Request::LaterReceived { versions }).unwrap();
    let slide = s.deck().slides[0].id.clone();
    assert_that!(s.received_to_review()).is_empty();
    assert_that!(s.library.knows(id)).is_false();
    assert_that!(s.svg(&slide).is_ok()).is_true();
}

#[test]
fn reopened_tab_lists_later_version_again() {
    let id = Uuid::new_v4();
    let (d, mut s) = received_deck(&[pack_files(id, "brought", "")]);
    let versions = refs(&s.received_to_review());
    s.handle(Request::LaterReceived { versions }).unwrap();
    let again = reopen(&d).received_to_review();
    assert_that!(again.iter().map(|r| (r.id, r.install)).collect::<Vec<_>>()).is_equal_to(vec![(id, true)]);
}

#[test]
fn accept_received_skips_blocked_version_installs_others() {
    let (theirs, fresh) = (Uuid::new_v4(), Uuid::new_v4());
    let (d, _s) = received_deck(&[pack_files(theirs, "shared", ""), pack_files(fresh, "fresh", "")]);
    let mine = Uuid::new_v4();
    session_on(d.me.path()).library.add_version(mine, &pack_files(mine, "shared", "")).unwrap();
    let mut s = reopen(&d);
    let review = s.received_to_review();
    accept(&mut s, refs(&review));
    assert_that!((s.library.knows(theirs), s.library.knows(fresh))).is_equal_to((false, true));
}

/// Version `id` of a theme under `namespace` extending `extends`, with Pro's tokens at the root.
fn theme_files(id: Uuid, namespace: &str, extends: &[&str]) -> PackFiles {
    let pro = crate::theme::Themes::builtins().iter().find(|b| b.manifest.id == crate::theme::PRO_THEME).unwrap();
    let manifest = crate::ThemeManifest {
        id,
        name: namespace.into(),
        namespace: ns(namespace),
        extends: extends.iter().map(|e| ns(e)).collect(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: if extends.is_empty() { pro.manifest.tokens.clone() } else { serde_json::Map::new() },
        guides: Guides::default(),
    };
    manifest.to_files().unwrap()
}

/// A deck tab on `me` pinning theme `maison` and its child `maison-sombre`, held only by the deck.
fn deck_with_theme_chain(me: &Path) -> (Session, (Uuid, PackFiles), (Uuid, PackFiles)) {
    let mut s = opened_on(me);
    let (root, child) = (Uuid::new_v4(), Uuid::new_v4());
    let (r, c) = (theme_files(root, "maison", &[]), theme_files(child, "maison-sombre", &["maison"]));
    let pins = BTreeMap::from([
        (ns("maison"), ThemePin { id: root, version: r.version(), extends: Vec::new() }),
        (ns("maison-sombre"), ThemePin { id: child, version: c.version(), extends: vec![ns("maison")] }),
    ]);
    s.doc.sources.add_theme(r.clone());
    s.doc.sources.add_theme(c.clone());
    s.apply("Theme", vec![Op::SetTheme { theme: Some(ns("maison-sombre")), themes: pins }], None).unwrap();
    (s, (root, r), (child, c))
}

#[test]
fn accept_received_installs_theme_and_its_parents() {
    let me = fresh_dir("oku-slides-receive-theme-");
    let (mut s, (root, r), (child, c)) = deck_with_theme_chain(me.path());
    accept(&mut s, vec![VersionRef { id: child, version: c.version() }]);
    let installed = (s.themes.lists(root, &r.version()), s.themes.lists(child, &c.version()));
    assert_that!(installed).is_equal_to((true, true));
    assert_that!(s.themes.is_received(child, &c.version())).is_true();
    assert_that!(s.theme_list.iter().any(|t| t.namespace == ns("maison-sombre"))).is_true();
}

/// A user whose library has a pack under `maison`, so a received theme `maison` cannot enter it.
fn theme_chain_with_taken_root() -> (tempfile::TempDir, Session, Uuid, Uuid) {
    let me = fresh_dir("oku-slides-receive-theme-");
    let mine = Uuid::new_v4();
    session_on(me.path()).library.add_version(mine, &pack_files(mine, "maison", "")).unwrap();
    let (s, (root, _), (child, _)) = deck_with_theme_chain(me.path());
    (me, s, root, child)
}

#[test]
fn received_to_review_of_child_theme_with_blocked_parent_has_blocker() {
    let (_me, s, root, child) = theme_chain_with_taken_root();
    let review = s.received_to_review();
    let of = |id: Uuid| review.iter().find(|r| r.id == id).map(|r| (r.install, r.blocker.is_some()));
    let expected = s.localizer.tr_with("received-blocker-parent", &[("namespace", "maison".into())]);
    assert_that!((of(root), of(child))).is_equal_to((Some((false, true)), Some((false, true))));
    assert_that!(review.iter().find(|r| r.id == child).and_then(|r| r.blocker.clone())).is_equal_to(Some(expected));
}

#[test]
fn received_to_review_of_child_theme_with_unreadable_parent_has_blocker() {
    let me = fresh_dir("oku-slides-receive-theme-");
    let (mut s, (_, r), (child, _)) = deck_with_theme_chain(me.path());
    let mut junk = serde_json::Map::new();
    junk.insert("name".into(), serde_json::json!("maison"));
    s.doc.sources.themes.insert(r.version(), PackFiles::new(PackKind::Widgets, junk, BTreeMap::new()));
    let review = s.fresh_reviews();
    let blocked = review.iter().find(|r| r.id == child).map(|r| (r.install, r.blocker.clone()));
    let expected = s.localizer.tr_with("received-blocker-parent", &[("namespace", "maison".into())]);
    assert_that!(blocked).is_equal_to(Some((false, Some(expected))));
}

#[test]
fn install_with_parents_of_child_whose_parent_stays_out_is_refused() {
    let (_me, s, _root, child) = theme_chain_with_taken_root();
    let held = s.held_versions();
    let one = held.iter().find(|h| h.id == child).unwrap();
    // Listed as installable although its parent is not: the install itself refuses it.
    let installable = vec![VersionRef { id: child, version: one.version.clone() }];
    let err = s.install_with_parents(one, &held, &installable, &mut Vec::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ReceivedParentMissing);
    assert_that!(s.themes.lists(child, one.version)).is_false();
}

#[test]
fn accept_received_of_only_blocked_versions_empties_the_list() {
    let theirs = Uuid::new_v4();
    let (d, _s) = received_deck(&[pack_files(theirs, "shared", "")]);
    let mine = Uuid::new_v4();
    session_on(d.me.path()).library.add_version(mine, &pack_files(mine, "shared", "")).unwrap();
    let mut s = reopen(&d);
    let review = s.received_to_review();
    assert_that!(review.iter().map(|r| r.install).collect::<Vec<_>>()).is_equal_to(vec![false]);
    accept(&mut s, refs(&review));
    assert_that!(s.received_to_review()).is_empty();
    assert_that!(s.library.knows(theirs)).is_false();
}

/// Makes `dir` read-only until the guard drops, so a library write under it fails.
struct ReadOnly(PathBuf);

impl ReadOnly {
    fn new(dir: PathBuf) -> Self {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        Self(dir)
    }
}

impl Drop for ReadOnly {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
    }
}

#[test]
fn accept_received_partial_failure_keeps_the_first_install_and_refreshes_the_lists() {
    let id = Uuid::new_v4();
    let (d, mut s) = received_deck(&[pack_files(id, "brought", "")]);
    let theme = Uuid::new_v4();
    let t = theme_files(theme, "maison", &[]);
    s.doc.sources.add_theme(t.clone());
    let pins = BTreeMap::from([(ns("maison"), ThemePin { id: theme, version: t.version(), extends: Vec::new() })]);
    s.apply("Theme", vec![Op::SetTheme { theme: Some(ns("maison")), themes: pins }], None).unwrap();
    let _locked = ReadOnly::new(d.me.path().join("themes"));
    (s.pack_updates, s.theme_list) = (7, Vec::new());
    let pack = s.held_versions().iter().find(|h| h.id == id).map(|h| h.version.clone()).unwrap();
    let versions = vec![VersionRef { id, version: pack.clone() }, VersionRef { id: theme, version: t.version() }];
    let err = s.handle(Request::AcceptReceived { versions });
    assert_that!(err.is_err()).is_true();
    assert_that!((s.library.lists(id, &pack), s.themes.lists(theme, &t.version()))).is_equal_to((true, false));
    assert_that!((s.pack_updates, s.theme_list.is_empty())).is_equal_to((0, false));
}

/// Une version enregistrée par l'autre session à partir du fichier `base`, son deck passé à `pack`.
fn received_edit(them: &Path, base: &Path, pack: &PackFiles) -> PathBuf {
    let mut other = session_on(them);
    other.open(base).unwrap();
    let id = PackManifest::from_files(pack).unwrap().id;
    other.library.add_version(id, pack).unwrap();
    other.apply("Pack", vec![pin_op(id, pack)], None).unwrap();
    let path = them.join("suite.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

#[test]
fn merge_bringing_version_lists_it() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let base = received_file(them.path(), &v0);
    let mut s = session_on(me.path());
    s.open(&base).unwrap();
    let path = received_edit(them.path(), &base, &v1);
    s.handle(Request::Merge { path }).unwrap();
    let listed: Vec<_> = s.received_to_review().iter().map(|r| r.version.clone()).collect();
    assert_that!(listed.contains(&v1.version())).is_true();
    assert_that!(s.library.versions(id)).is_empty();
}

#[test]
fn open_of_a_deck_only_pack_adds_nothing_to_the_library() {
    let (me, them) = (fresh_dir("oku-slides-receive-me-"), fresh_dir("oku-slides-receive-them-"));
    let mut other = session_on(them.path());
    let r = adopt_deck_only_pack(&mut other);
    let path = them.path().join("importe.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    let packs = s.library.packs(PackKind::Widgets);
    s.open(&path).unwrap();
    assert_that!(pin(&s, "deck")).is_equal_to(Some(r.clone()));
    assert_that!(s.library.versions(r.id).is_empty()).is_true();
    assert_that!(s.library.packs(PackKind::Widgets)).is_equal_to(&packs);
    assert_that!(s.pack_updates()).is_equal_to(0);
}

fn deck_pack(s: &mut Session) -> serde_json::Value {
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    snap["deck_pack"].clone()
}

#[test]
fn snapshot_deck_pack_is_false_for_a_library_pack() {
    let dir = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(dir.path());
    assert_that!(deck_pack(&mut s)).is_equal_to(serde_json::Value::Bool(false));
}

#[test]
fn snapshot_deck_pack_is_true_once_the_deck_takes_a_deck_only_pack() {
    let dir = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(dir.path());
    adopt_deck_only_pack(&mut s);
    assert_that!(deck_pack(&mut s)).is_equal_to(serde_json::Value::Bool(true));
}

#[test]
fn is_deck_only_with_an_unreadable_scope_is_false_and_logged() {
    let mut manifest = serde_json::Map::new();
    manifest.insert("scope".into(), serde_json::json!("ailleurs"));
    let files = PackFiles::new(PackKind::Widgets, manifest, std::collections::BTreeMap::new());
    let mut deck_only = true;
    let log = logged(|| deck_only = super::is_deck_only(&files));
    assert_that!(deck_only).is_false();
    assert_that!(log.as_str()).contains("WARN");
}

/// Un thème de la bibliothèque d'avant les espaces de noms est écarté, et dit une fois par processus.
#[test]
fn notice_skipped_themes_counts_the_unreadable_library_themes_once() {
    let data = fresh_dir("oku-slides-themes-skipped-");
    crate::test_support::write_raw_theme(&data.path().join("themes"), crate::test_support::OLD_THEME_JSON);
    let mut s = session_on(data.path());
    let expected = s.localizer.tr_with("notice-themes-skipped", &[("count", 1.into())]);
    assert_that!(s.doc.notice.clone()).is_equal_to(Some(expected.clone()));
    s.doc.notice = None;
    s.notice_skipped_themes();
    assert_that!(s.doc.notice.clone()).is_none();
    let de = Localizer::new(Some("de")).tr_with("notice-themes-skipped", &[("count", 1.into())]);
    assert_that!(de)
        .is_not_equal_to(Localizer::new(Some("en")).tr_with("notice-themes-skipped", &[("count", 1.into())]));
}

/// L'avis des thèmes écartés suit le document qui remplace le deck vierge (reprise d'un onglet) ; vu, il ne revient pas.
#[test]
fn notice_skipped_themes_follows_the_document_loaded_over_the_blank_deck_once() {
    let data = fresh_dir("oku-slides-themes-skipped-load-");
    crate::test_support::write_raw_theme(&data.path().join("themes"), crate::test_support::OLD_THEME_JSON);
    let mut s = session_on(data.path());
    let expected = s.localizer.tr_with("notice-themes-skipped", &[("count", 1.into())]);
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/roadmap.json");
    s.open(&fixture).unwrap();
    assert_that!(s.doc.notice.clone()).is_equal_to(Some(expected));
    // Vu par l'interface : le snapshot le vide ; un autre document ne le reprend pas.
    s.handle(Request::Snapshot).unwrap();
    s.open(&fixture).unwrap();
    assert_that!(s.doc.notice.clone()).is_none();
    let other = session_on(data.path());
    assert_that!(other.doc.notice.clone()).is_none();
}
