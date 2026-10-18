use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackFiles, PackPin, PackVersion};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use oku_slides_i18n::Localizer;

use crate::pack::PackManifest;
use crate::pack_notice::pack_notice_banner;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, logged, manifest, namespace_of, pin, session_on};
use crate::{Request, Session};

fn files(id: Uuid, name: &str, parent: Option<&PackVersion>, created: &str) -> PackFiles {
    manifest(id, name, parent, created).to_files(std::collections::BTreeMap::new()).unwrap()
}

/// L'op qui épingle la version `pack` du pack `id`, sous l'espace de noms de son id.
fn pin_op(id: Uuid, pack: &PackFiles) -> Op {
    Op::AddPack { namespace: namespace_of(id), pin: PackPin { id, version: pack.version(), requires: Vec::new() } }
}

/// La version du pack `id` que le deck de `s` épingle.
fn pinned_version(s: &Session, id: Uuid) -> Option<PackVersion> {
    s.deck().widgets.get(&namespace_of(id)).map(|r| r.version.clone())
}

/// L'id du pack de l'utilisateur épinglé par le deck de `s` (pas le pack par défaut).
fn user_pack_id(s: &Session) -> Uuid {
    s.deck().widgets.values().map(|r| r.id).find(|id| *id != crate::DEFAULT_PACK).unwrap()
}

/// Un fichier enregistré par une autre session (dossier `them`), dont le deck embarque `pack`.
fn received_file(them: &Path, pack: &PackFiles) -> PathBuf {
    let mut other = session_on(them);
    let id = PackManifest::from_files(pack).unwrap().id;
    other.library.add_version(id, pack).unwrap();
    other.apply("Pack", vec![pin_op(id, pack)], None).unwrap();
    let path = them.join("recu.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

fn notice(s: &mut Session) -> serde_json::Value {
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    snap["pack_notice"].clone()
}

/// Les composants de la surface du bandeau.
fn components(n: &serde_json::Value) -> Vec<serde_json::Value> {
    let messages = n["messages"].as_array().cloned().unwrap_or_default();
    let update = messages.iter().find_map(|m| m.get("updateComponents")).cloned().unwrap_or_default();
    update["components"].as_array().cloned().unwrap_or_default()
}

/// Le composant `id` du bandeau.
fn component(n: &serde_json::Value, id: &str) -> serde_json::Value {
    components(n).into_iter().find(|c| c["id"] == id).unwrap_or_default()
}

/// Le texte du bandeau.
fn notice_text(n: &serde_json::Value) -> String {
    component(n, "root")["text"].as_str().unwrap_or_default().to_owned()
}

/// Libellé et évènement de chaque bouton du bandeau, dans l'ordre.
fn buttons(n: &serde_json::Value) -> Vec<(String, String)> {
    let ids = component(n, "root")["children"].as_array().cloned().unwrap_or_default();
    ids.iter()
        .map(|id| component(n, id.as_str().unwrap_or_default()))
        .map(|b| {
            let label = b["label"].as_str().unwrap_or_default().to_owned();
            (label, b["action"]["event"]["name"].as_str().unwrap_or_default().to_owned())
        })
        .collect()
}

#[test]
fn open_adds_received_versions_silently_without_changing_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.versions(id).len()).is_equal_to(2);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
    // t-355 : la version reçue est marquée, la mienne non.
    assert_that!((s.library.is_received(id, &v0.version()), s.library.is_received(id, &v1.version())))
        .is_equal_to((false, true));
    // t-355 : la version reçue est marquée, la mienne non.
    assert_that!((s.library.is_received(id, &v0.version()), s.library.is_received(id, &v1.version())))
        .is_equal_to((false, true));
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v1.version()));
}

#[test]
fn open_with_an_unknown_pack_creates_it_with_the_received_version_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "Nouveau", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.open(&received_file(them.path(), &v)).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v.version()));
    assert_that!(notice(&mut s).is_null()).is_true();
}

/// v0 ← v1 ; v0 ← v2. `mine` : ma version courante ; `theirs` : celle du fichier.
#[rstest]
#[case::newer(0, 1, "plus récente")]
#[case::older(1, 0, "plus ancienne")]
#[case::parallel(1, 2, "en parallèle")]
fn open_with_another_version_shows_the_banner_with_its_relation(
    #[case] mine: usize,
    #[case] theirs: usize,
    #[case] words: &str,
) {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "Charte", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "Charte", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let v2 = files(id, "Charte", Some(&v0.version()), "2026-10-03T10:00:00Z");
    let v = [v0, v1, v2];
    let mut s = session_on(me.path());
    s.library.add_version(id, &v[0]).unwrap();
    s.library.add_version(id, &v[1]).unwrap();
    s.library.set_current(id, &v[mine].version()).unwrap();
    s.open(&received_file(them.path(), &v[theirs])).unwrap();
    let n = notice(&mut s);
    assert_that!(notice_text(&n)).contains(words);
    assert_that!(notice_text(&n)).contains("Charte");
    assert_that!(s.library.current(id)).is_equal_to(Some(v[mine].version()));
}

#[test]
fn open_with_the_current_version_shows_no_banner() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "P", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.library.add_version(id, &v).unwrap();
    s.open(&received_file(them.path(), &v)).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
}

/// Ma version courante v0, le fichier en v1 : la session ouverte, et les deux versions.
fn opened_on_newer() -> (tempfile::TempDir, tempfile::TempDir, Session, PackFiles, PackFiles) {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    (me, them, s, v0, v1)
}

#[test]
fn pack_use_mine_switches_the_deck_and_undo_brings_theirs_back() {
    let (_me, _them, mut s, v0, v1) = opened_on_newer();
    let id = user_pack_id(&s);
    s.handle(Request::PackUseMine).unwrap();
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v0.version()));
    assert_that!(s.sources_for_tests().packs.contains_key(&v0.version())).is_true();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.handle(Request::Undo).unwrap();
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v1.version()));
}

#[test]
fn pack_make_current_makes_theirs_current_and_closes_the_banner() {
    let (_me, _them, mut s, _v0, v1) = opened_on_newer();
    let id = user_pack_id(&s);
    s.handle(Request::PackMakeCurrent).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v1.version()));
    assert_that!(notice(&mut s).is_null()).is_true();
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v1.version()));
}

#[test]
fn pack_notice_close_hides_the_banner_for_the_session() {
    let (_me, them, mut s, _v0, _v1) = opened_on_newer();
    s.handle(Request::PackNoticeClose).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.open(&them.path().join("recu.oku-slides.pdf")).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
}

#[test]
fn pack_use_mine_is_refused_while_editing_templates() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    s.edit_templates(crate::pack::DEFAULT_PACK).unwrap();
    let err = s.handle(Request::PackUseMine).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::EditingTemplates);
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
fn merge_adds_received_versions_silently_without_banner() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let base = received_file(them.path(), &v0);
    let mut s = session_on(me.path());
    s.open(&base).unwrap();
    let path = received_edit(them.path(), &base, &v1);
    s.handle(Request::Merge { path }).unwrap();
    assert_that!(s.library.versions(id).len()).is_equal_to(2);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
    // t-355 : les deux versions sont arrivées avec des decks reçus.
    assert_that!((s.library.is_received(id, &v0.version()), s.library.is_received(id, &v1.version())))
        .is_equal_to((true, true));
    assert_that!(notice(&mut s).is_null()).is_true();
}

#[test]
fn pack_notice_after_the_deck_changes_pack_is_gone_and_make_current_does_nothing() {
    let (_me, _them, mut s, v0, _v1) = opened_on_newer();
    let id = user_pack_id(&s);
    s.apply("Pack", vec![pin_op(id, &v0)], None).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.handle(Request::PackMakeCurrent).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
}

#[test]
fn pack_make_current_is_refused_while_editing_templates() {
    let (_me, _them, mut s, v0, _v1) = opened_on_newer();
    let id = user_pack_id(&s);
    s.edit_templates(crate::pack::DEFAULT_PACK).unwrap();
    let err = s.handle(Request::PackMakeCurrent).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::EditingTemplates);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
}

/// t-328 : une version reçue que la bibliothèque n'a pas pu ajouter n'empêche
/// pas l'ouverture ; le bandeau ne propose pas d'en faire ma version courante.
#[test]
fn open_when_adding_the_received_version_fails_opens_and_offers_no_make_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "Charte", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "Charte", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    // Un fichier à la place du dossier de v1 : son écriture échoue.
    let versions = me.path().join("widgets").join(id.to_string()).join("versions");
    std::fs::write(versions.join(v1.version().hex()), b"").unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.version(id, &v1.version())).is_none();
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v1.version()));
    let n = notice(&mut s);
    assert_that!(notice_text(&n)).contains("Charte");
    assert_that!(buttons(&n)).is_equal_to(vec![("Passer à ma version".to_owned(), "pack_use_mine".to_owned())]);
}

#[test]
fn pack_notice_surface_offers_make_current_when_the_library_has_their_version() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    assert_that!(buttons(&notice(&mut s))).is_equal_to(vec![
        ("Passer à ma version".to_owned(), "pack_use_mine".to_owned()),
        ("Faire de la sienne ma version courante".to_owned(), "pack_make_current".to_owned()),
    ]);
}

#[test]
fn pack_notice_surface_is_placed_as_a_banner() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    assert_that!(notice(&mut s)["place"].as_str()).is_equal_to(Some("banner"));
}

#[test]
fn pack_notice_surface_use_mine_button_is_primary() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    assert_that!(component(&notice(&mut s), "pack-use-mine")["primary"].as_bool()).is_equal_to(Some(true));
}

#[test]
fn pack_notice_surface_make_current_button_is_not_primary() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    assert_that!(component(&notice(&mut s), "pack-make-current")["primary"].as_bool()).is_none();
}

/// Les noms d'évènement du bandeau (boutons et croix), dans l'ordre.
fn banner_events() -> Vec<String> {
    let banner = pack_notice_banner("x".into(), true, &Localizer::new(Some("fr")));
    let json = serde_json::to_value(banner).unwrap();
    components(&json)
        .iter()
        .flat_map(|c| [c["action"]["event"]["name"].clone(), c["close"]["event"]["name"].clone()])
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn pack_notice_banner_sends_three_events() {
    assert_that!(banner_events()).has_length(3);
}

/// Chaque évènement du bandeau porte le nom de la requête qu'il envoie (`cmd`).
#[test]
fn pack_notice_banner_events_are_request_names() {
    for name in banner_events() {
        let parsed = serde_json::from_value::<Request>(serde_json::json!({ "cmd": name }));
        assert_that!(parsed.is_ok()).named(&name).is_true();
    }
}

#[test]
fn pack_notice_surface_close_sends_pack_notice_close_with_its_label() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    let root = component(&notice(&mut s), "root");
    assert_that!(root["close"]["event"]["name"].as_str()).is_equal_to(Some("pack_notice_close"));
    assert_that!(root["close_label"].as_str()).is_equal_to(Some("Fermer"));
}

#[test]
fn open_of_a_deck_only_pack_adds_nothing_to_the_library_and_shows_no_notice() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let mut other = session_on(them.path());
    let r = adopt_deck_only_pack(&mut other);
    let path = them.path().join("importe.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    let packs = s.library.packs();
    s.open(&path).unwrap();
    assert_that!(pin(&s, "deck")).is_equal_to(Some(r.clone()));
    assert_that!(s.library.versions(r.id).is_empty()).is_true();
    assert_that!(s.library.packs()).is_equal_to(&packs);
    assert_that!(notice(&mut s)).is_equal_to(serde_json::Value::Null);
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
    let files = PackFiles::new(manifest, std::collections::BTreeMap::new());
    let mut deck_only = true;
    let log = logged(|| deck_only = super::is_deck_only(&files));
    assert_that!(deck_only).is_false();
    assert_that!(log.as_str()).contains("WARN");
}

/// Deux packs reçus, le premier par espace de noms à jour : le bandeau parle du second.
#[test]
fn pack_notice_for_second_namespace_is_shown_when_first_is_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let (a, b) = if namespace_of(a) < namespace_of(b) { (a, b) } else { (b, a) };
    let a0 = files(a, "Premier", None, "2026-10-01T10:00:00Z");
    let b0 = files(b, "Second", None, "2026-10-01T10:00:00Z");
    let b1 = files(b, "Second", Some(&b0.version()), "2026-10-02T10:00:00Z");
    let mut other = session_on(them.path());
    other.library.add_version(a, &a0).unwrap();
    other.library.add_version(b, &b1).unwrap();
    other.apply("Packs", vec![pin_op(a, &a0), pin_op(b, &b1)], None).unwrap();
    let path = them.path().join("deux.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut s = session_on(me.path());
    s.library.add_version(a, &a0).unwrap();
    s.library.add_version(b, &b0).unwrap();
    s.open(&path).unwrap();
    let text = notice_text(&notice(&mut s));
    assert_that!(text.as_str()).contains("Second");
    assert_that!(text.as_str()).does_not_contain("Premier");
}

#[test]
fn pack_use_mine_pins_the_library_version() {
    let (_me, _them, mut s, v0, _v1) = opened_on_newer();
    let id = user_pack_id(&s);
    s.pack_use_mine().unwrap();
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(v0.version()));
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
}
