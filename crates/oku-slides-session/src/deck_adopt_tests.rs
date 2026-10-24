use speculoos::prelude::*;
use uuid::Uuid;

use crate::deck_packs::deck_packs_tests::{files, received_file};
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{
    add_pack, add_user_theme, add_version, fresh_dir, library_of, manifest, ns, rename_title, session_on,
};

#[test]
fn adopt_pins_pinning_a_library_pack_freezes_its_version() {
    let data = fresh_dir("oku-slides-adopt-");
    let mut s = session_on(data.path());
    let m = manifest(Uuid::new_v4(), "P", None, "2026-10-24T10:00:00Z");
    let pin = add_version(&s.library, &m, &[]);
    s.apply("Pack", vec![add_pack(&m, &pin)], None).unwrap();
    assert_that!(s.library.is_frozen(m.id, &pin.version)).is_true();
}

/// Un deck reçu épingle une version inconnue d'un pack connu : elle reste reçue, la courante ne bouge pas.
#[test]
fn adopt_pins_keeps_a_received_version_received() {
    let (me, them) = (fresh_dir("oku-slides-adopt-me-"), fresh_dir("oku-slides-adopt-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.library.set_current(id, &v0.version()).unwrap();
    s.open(&path).unwrap();
    s.adopt_pins();
    assert_that!(s.library.is_received(id, &v1.version())).is_true();
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
}

#[test]
fn adopt_pins_in_pack_mode_leaves_the_session_version_a_draft() {
    let data = fresh_dir("oku-slides-adopt-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    let v = s.save_pack(false).unwrap();
    assert_that!(library_of(&s).is_frozen(ESSENTIEL_PACK, &v)).is_false();
}

#[test]
fn close_pack_freezes_the_version_the_deck_now_pins() {
    let data = fresh_dir("oku-slides-adopt-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    let v = s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    assert_that!(library_of(&s).is_frozen(ESSENTIEL_PACK, &v)).is_true();
}

#[test]
fn adopt_pins_setting_a_user_theme_freezes_its_version() {
    let data = fresh_dir("oku-slides-adopt-");
    let mut s = session_on(data.path());
    let id = Uuid::new_v4();
    let version = add_user_theme(&s.themes, id, "marque", &[], Default::default(), None);
    s.theme_list = s.themes.themes();
    s.set_theme(&ns("marque")).unwrap();
    assert_that!(s.themes.is_frozen(id, &version)).is_true();
}
