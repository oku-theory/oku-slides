use std::path::Path;

use oku_slides_core::{PackFiles, PackKind, PackVersion};

use crate::SessionErrorKind;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::PackLibrary;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK, Relation};
use crate::test_support::{add_version, fresh_dir, manifest, manifest_ns};

/// Une bibliothèque des packs : les widgets sous `dir`, les layouts sous `dir/../<nom>-layouts`.
fn library(dir: &Path) -> PackLibrary {
    let layouts = dir.with_file_name(format!("{}-layouts", dir.file_name().unwrap().to_string_lossy()));
    PackLibrary::new(
        Some(dir.to_owned()),
        Some(layouts),
        crate::ThemeLibrary::builtin_only(),
        Storage::local().unwrap(),
    )
}

#[test]
fn builtin_versions_have_the_constant_ids() {
    let created: Vec<(Uuid, &str)> =
        builtin_versions().iter().map(|b| (b.manifest.id, b.manifest.created.as_str())).collect();
    assert_that!(created)
        .is_equal_to(vec![(BASE_PACK, "2026-10-23T00:00:00Z"), (ESSENTIEL_PACK, "2026-10-23T00:00:00Z")]);
}

#[test]
fn builtin_versions_have_the_previous_shipped_version_as_parent() {
    // L'avant-dernière ligne de `versions.txt` : la version livrée juste avant la courante.
    let previous = |p: oku_slides_render::BuiltinPack| {
        let shipped: Vec<&str> = p.shipped_versions().collect();
        shipped.len().checked_sub(2).map(|i| shipped[i].to_owned())
    };
    let parents: Vec<Option<String>> =
        builtin_versions().iter().map(|b| b.manifest.parent.as_ref().map(|v| v.hex().to_owned())).collect();
    let shipped: Vec<Option<String>> = oku_slides_render::BuiltinPack::ALL.into_iter().map(previous).collect();
    assert_that!(parents).is_equal_to(shipped);
}

#[rstest::rstest]
#[case(PackKind::Widgets, vec!["base"])]
#[case(PackKind::Layouts, vec!["essentiel"])]
fn library_builtins_by_kind(#[case] kind: PackKind, #[case] expected: Vec<&str>) {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let names: Vec<String> = lib.packs(kind).iter().filter(|p| p.default).map(|p| p.namespace.to_string()).collect();
    assert_that!(names).is_equal_to(expected.into_iter().map(str::to_owned).collect::<Vec<_>>());
}

#[test]
fn library_has_no_grilles() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let all: Vec<String> = PackKind::ALL.iter().flat_map(|k| lib.packs(*k)).map(|p| p.namespace.to_string()).collect();
    assert_that!(all.contains(&"grilles".to_owned())).is_false();
}

#[rstest::rstest]
#[case(PackKind::Widgets, "essentiel")]
#[case(PackKind::Layouts, "base")]
#[case(PackKind::Layouts, "deck-layouts")]
fn library_add_namespace_taken_by_any_kind_is_refused(#[case] kind: PackKind, #[case] namespace: &str) {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let m = manifest_ns(Uuid::new_v4(), namespace, &[]);
    let err = lib.add_version(m.id, &m.to_files(kind, BTreeMap::new()).unwrap()).unwrap_err();
    assert_that!(matches!(err.kind(), SessionErrorKind::NamespaceTaken | SessionErrorKind::NamespaceReserved))
        .is_true();
}

#[test]
fn builtin_pack_is_current_by_default_and_read_only() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = crate::test_support::builtin(BASE_PACK).version.clone();
    assert_that!(lib.current(BASE_PACK)).is_equal_to(Some(builtin.clone()));
    assert_that!(PackLibrary::is_builtin(BASE_PACK)).is_true();
    // Rien n'est écrit pour une version intégrée.
    assert_that!(dir.path().join(BASE_PACK.to_string()).exists()).is_false();
    assert_that!(lib.version(BASE_PACK, &builtin).is_some()).is_true();
}

#[test]
fn add_version_of_unknown_pack_creates_it_with_that_version_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let r = add_version(&lib, &manifest(id, "Mien", None, "2026-10-01T10:00:00Z"), &[("kpi", "#let widget = none")]);
    assert_that!(lib.current(id)).is_equal_to(Some(r.version.clone()));
    let on_disk = dir.path().join(id.to_string()).join("versions").join(r.version.hex());
    assert_that!(on_disk.join("widgets.json").exists() && on_disk.join("widgets").join("kpi.typ").exists()).is_true();
}

#[test]
fn add_version_then_versions_reads_every_widget_file_and_ignores_strangers() {
    let dir = crate::test_support::fresh_dir("oku-slides-library-widgets-");
    let lib = library(dir.path());
    let id = uuid::Uuid::new_v4();
    let m = manifest(id, "A", None, "2026-10-02T10:00:00Z");
    let r = crate::test_support::add_version(&lib, &m, &[("kpi", "K"), ("_commun", "C")]);
    let at = dir.path().join(id.to_string()).join("versions").join(r.version.hex()).join("widgets");
    std::fs::write(at.join(".DS_Store"), b"x").unwrap();
    let files = lib.version(id, &r.version).unwrap();
    assert_that!(files.widgets().len()).is_equal_to(2);
    assert_that!(files.matches(&r.version)).is_true();
}

#[test]
fn add_version_does_not_change_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let first = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    // Plus récente : la date ne fait pas la version courante.
    let later = add_version(&lib, &manifest(id, "B", Some(&first.version), "2026-10-02T10:00:00Z"), &[]);
    assert_that!(lib.current(id)).is_equal_to(Some(first.version));
    assert_that!(lib.versions(id).len()).is_equal_to(2);
    assert_that!(lib.add_version(id, &lib.version(id, &later.version).unwrap()).unwrap()).is_false();
}

#[test]
fn add_version_under_a_builtin_id_keeps_the_builtin_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = crate::test_support::builtin(BASE_PACK).version.clone();
    add_version(&lib, &manifest(BASE_PACK, "Sœur", Some(&builtin), "2026-10-02T10:00:00Z"), &[]);
    assert_that!(lib.current(BASE_PACK)).is_equal_to(Some(builtin));
    assert_that!(lib.versions(BASE_PACK).len()).is_equal_to(2);
}

#[test]
fn set_current_makes_a_version_current_even_over_a_builtin() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = crate::test_support::builtin(BASE_PACK).version.clone();
    let mine = add_version(&lib, &manifest(BASE_PACK, "Le mien", Some(&builtin), "2026-10-02T10:00:00Z"), &[]);
    lib.set_current(BASE_PACK, &mine.version).unwrap();
    assert_that!(lib.current(BASE_PACK)).is_equal_to(Some(mine.version));
}

/// Une session d'édition des modèles n'a qu'une version : chaque enregistrement remplace la précédente.
#[test]
fn replace_version_keeps_the_parent_removes_the_old_one_and_updates_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = crate::test_support::builtin(BASE_PACK).version.clone();
    let first = add_version(&lib, &manifest(BASE_PACK, "Un", Some(&builtin), "2026-10-02T10:00:00Z"), &[]);
    lib.set_current(BASE_PACK, &first.version).unwrap();
    let next = manifest(BASE_PACK, "Deux", Some(&builtin), "2026-10-02T10:05:00Z")
        .to_files(oku_slides_core::PackKind::Widgets, BTreeMap::new())
        .unwrap();
    let replaced = lib.replace_version(BASE_PACK, &first.version, &next).unwrap();
    assert_that!(replaced).is_equal_to(next.version());
    assert_that!(lib.current(BASE_PACK)).is_equal_to(Some(next.version()));
    assert_that!(lib.version(BASE_PACK, &first.version)).is_none();
    assert_that!(dir.path().join(BASE_PACK.to_string()).join("versions").join(first.version.hex()).exists()).is_false();
    let current = lib.current_version(BASE_PACK).unwrap();
    assert_that!(current.manifest.parent).is_equal_to(Some(builtin));
    assert_that!(lib.versions(BASE_PACK).len()).is_equal_to(2);
}

#[test]
fn replace_version_never_deletes_the_builtin_version() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = crate::test_support::builtin(BASE_PACK).version.clone();
    let next = manifest(BASE_PACK, "Mien", Some(&builtin), "2026-10-02T10:00:00Z")
        .to_files(oku_slides_core::PackKind::Widgets, BTreeMap::new())
        .unwrap();
    lib.replace_version(BASE_PACK, &builtin, &next).unwrap();
    assert_that!(lib.version(BASE_PACK, &builtin).is_some()).is_true();
    assert_that!(lib.current(BASE_PACK)).is_equal_to(Some(next.version()));
}

#[test]
fn set_current_to_an_absent_version_returns_pack_version_missing() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let absent = PackVersion::from_hex(&"0".repeat(64)).unwrap();
    let err = lib.set_current(BASE_PACK, &absent).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
    let err = lib.set_current(Uuid::new_v4(), &absent).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownPack);
}

#[test]
fn current_pointing_to_an_absent_version_falls_back_to_the_latest() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    let b = add_version(&lib, &manifest(id, "B", Some(&a.version), "2026-10-03T10:00:00Z"), &[]);
    std::fs::write(dir.path().join(id.to_string()).join("current"), format!("sha256:{}", "e".repeat(64))).unwrap();
    // Écrit à la main, hors de la bibliothèque : seule une instance neuve le voit.
    assert_that!(library(dir.path()).current(id)).is_equal_to(Some(b.version));
}

#[test]
fn version_folder_whose_content_differs_is_ignored() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    let forged = dir.path().join(id.to_string()).join("versions").join("f".repeat(64));
    std::fs::create_dir_all(&forged).unwrap();
    std::fs::copy(
        dir.path().join(id.to_string()).join("versions").join(a.version.hex()).join("widgets.json"),
        forged.join("widgets.json"),
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join(id.to_string()).join("versions").join("pas-une-version")).unwrap();
    assert_that!(lib.versions(id).len()).is_equal_to(1);
}

#[rstest::rstest]
#[case::same(0, 0, Relation::Same)]
#[case::older(0, 1, Relation::Ancestor)]
#[case::newer(1, 0, Relation::Descendant)]
#[case::grand_child(2, 0, Relation::Descendant)]
#[case::parallel(1, 3, Relation::Sibling)]
fn relation_follows_the_parent_chain(#[case] a: usize, #[case] b: usize, #[case] expected: Relation) {
    // v0 ← v1 ← v2 ; v0 ← v3
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let v0 = add_version(&lib, &manifest(id, "v0", None, "2026-10-01T10:00:00Z"), &[]).version;
    let v1 = add_version(&lib, &manifest(id, "v1", Some(&v0), "2026-10-02T10:00:00Z"), &[]).version;
    let v2 = add_version(&lib, &manifest(id, "v2", Some(&v1), "2026-10-03T10:00:00Z"), &[]).version;
    let v3 = add_version(&lib, &manifest(id, "v3", Some(&v0), "2026-10-04T10:00:00Z"), &[]).version;
    let v = [v0, v1, v2, v3];
    assert_that!(lib.relation(id, &v[a], &v[b])).is_equal_to(expected);
}

#[test]
fn packs_lists_builtins_first_then_user_packs_by_name() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let (z, a) = (Uuid::new_v4(), Uuid::new_v4());
    add_version(&lib, &manifest(z, "Zeta", None, "2026-10-01T10:00:00Z"), &[]);
    add_version(&lib, &manifest(a, "alpha", None, "2026-10-01T10:00:00Z"), &[]);
    std::fs::create_dir_all(dir.path().join("pas-un-uuid")).unwrap();
    let ids: Vec<(Uuid, bool, bool)> =
        lib.packs(PackKind::Widgets).into_iter().map(|p| (p.id, p.builtin, p.default)).collect();
    assert_that!(ids).is_equal_to(vec![(BASE_PACK, true, true), (a, false, false), (z, false, false)]);
}

#[test]
fn delete_removes_every_version_of_a_user_pack() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    lib.delete(id).unwrap();
    assert_that!(lib.current(id)).is_none();
    assert_that!(dir.path().join(id.to_string()).exists()).is_false();
}

#[test]
fn add_version_without_pack_dir_returns_no_pack_dir() {
    let lib = PackLibrary::new(None, None, crate::ThemeLibrary::builtin_only(), Storage::local().unwrap());
    let id = Uuid::new_v4();
    let files = manifest(id, "A", None, "2026-10-01T10:00:00Z").to_files(PackKind::Widgets, BTreeMap::new()).unwrap();
    let err = lib.add_version(id, &files).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NoLibraryDir);
}

#[test]
fn add_version_with_malformed_pack_json_is_refused_and_writes_nothing() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let json = serde_json::json!({ "id": id.to_string(), "name": "Sans modèles ni date" });
    let files = PackFiles::new(PackKind::Widgets, json.as_object().cloned().unwrap(), BTreeMap::new());
    let err = lib.add_version(id, &files).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::InvalidDocument);
    assert_that!(dir.path().join(id.to_string()).exists()).is_false();
}

#[test]
fn add_version_of_another_pack_is_refused_and_writes_nothing() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let (id, other) = (Uuid::new_v4(), Uuid::new_v4());
    let files = manifest(other, "Autre", None, "2026-10-01T10:00:00Z")
        .to_files(oku_slides_core::PackKind::Widgets, BTreeMap::new())
        .unwrap();
    let err = lib.add_version(id, &files).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::InvalidDocument);
    assert_that!(dir.path().join(id.to_string()).exists()).is_false();
    assert_that!(dir.path().join(other.to_string()).exists()).is_false();
}

#[test]
fn add_version_of_a_deck_only_pack_is_refused_and_writes_nothing() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let mut m = manifest(id, "Importé", None, "2026-10-01T10:00:00Z");
    m.scope = oku_slides_core::PackScope::Deck;
    let err =
        lib.add_version(id, &m.to_files(oku_slides_core::PackKind::Widgets, BTreeMap::new()).unwrap()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::DeckOnly);
    let message = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(message).contains(id.to_string().as_str());
    let english = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("en")));
    assert_that!(message).is_not_equal_to(english);
    assert_that!(dir.path().join(id.to_string()).exists()).is_false();
}

#[test]
fn replace_version_keeps_old_when_it_is_the_parent_of_the_new_one() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let first = add_version(&lib, &manifest(id, "Un", None, "2026-10-01T10:00:00Z"), &[]);
    let next = manifest(id, "Deux", Some(&first.version), "2026-10-01T10:05:00Z")
        .to_files(oku_slides_core::PackKind::Widgets, BTreeMap::new())
        .unwrap();
    lib.replace_version(id, &first.version, &next).unwrap();
    assert_that!(lib.current(id)).is_equal_to(Some(next.version()));
    assert_that!(lib.version(id, &first.version).is_some()).is_true();
    assert_that!(lib.relation(id, &first.version, &next.version())).is_equal_to(Relation::Ancestor);
}

#[cfg(unix)]
#[test]
fn replace_version_whose_old_folder_cannot_be_deleted_still_succeeds() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let first = add_version(&lib, &manifest(id, "Un", None, "2026-10-01T10:00:00Z"), &[]);
    let old = dir.path().join(id.to_string()).join("versions").join(first.version.hex());
    // Dossier en lecture seule : son `pack.json` ne peut pas être retiré.
    std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o555)).unwrap();
    let next = manifest(id, "Deux", None, "2026-10-01T10:05:00Z")
        .to_files(oku_slides_core::PackKind::Widgets, BTreeMap::new())
        .unwrap();
    let replaced = lib.replace_version(id, &first.version, &next);
    std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_that!(replaced.unwrap()).is_equal_to(next.version());
    assert_that!(lib.current(id)).is_equal_to(Some(next.version()));
    assert_that!(old.join("widgets.json").exists()).is_true();
}

#[test]
fn relation_with_a_dangling_parent_chain_is_sibling() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let missing = PackVersion::from_hex(&"a".repeat(64)).unwrap();
    let a = add_version(&lib, &manifest(id, "A", Some(&missing), "2026-10-01T10:00:00Z"), &[]).version;
    let b = add_version(&lib, &manifest(id, "B", None, "2026-10-02T10:00:00Z"), &[]).version;
    assert_that!(lib.relation(id, &a, &b)).is_equal_to(Relation::Sibling);
    assert_that!(lib.relation(id, &b, &a)).is_equal_to(Relation::Sibling);
}

/// Supprimer sous l'id d'un pack intégré : retour à la version intégrée.
#[test]
fn delete_under_a_builtin_id_reverts_to_the_builtin_version() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = crate::test_support::builtin(BASE_PACK).version.clone();
    let mine = add_version(&lib, &manifest(BASE_PACK, "Le mien", Some(&builtin), "2026-10-02T10:00:00Z"), &[]);
    lib.set_current(BASE_PACK, &mine.version).unwrap();
    lib.delete(BASE_PACK).unwrap();
    assert_that!(lib.current(BASE_PACK)).is_equal_to(Some(builtin));
    assert_that!(lib.versions(BASE_PACK).len()).is_equal_to(1);
    assert_that!(lib.version(BASE_PACK, &mine.version)).is_none();
}

/// t-328 : un dossier au format ancien (nom non-UUID) est ignoré, signalé une fois.
#[test]
fn packs_ignore_a_legacy_folder_and_report_it_once() {
    let dir = fresh_dir("oku-slides-library-");
    let legacy = dir.path().join("ma-charte");
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("pack.json"), br#"{"name":"Ma charte"}"#).unwrap();
    let lib = library(dir.path());
    assert_that!(lib.packs(PackKind::Widgets).iter().any(|p| p.name == "Ma charte")).is_false();
    // `packs()` l'a déjà signalé : plus jamais pour ce dossier.
    assert_that!(first_legacy_sighting(&legacy)).is_false();
    assert_that!(first_legacy_sighting(&dir.path().join("autre-ancien"))).is_true();
    assert_that!(first_legacy_sighting(&dir.path().join("autre-ancien"))).is_false();
}

/// Une version reçue avec un deck pour `m`.
fn receive(lib: &PackLibrary, m: &crate::pack::PackManifest) -> oku_slides_core::PackPin {
    let files = m.to_files(oku_slides_core::PackKind::Widgets, Default::default()).unwrap();
    lib.receive_version(m.id, &files).unwrap();
    oku_slides_core::PackPin { id: m.id, version: files.version(), requires: Vec::new() }
}

/// t-355 : un pack inconnu reçu avec un deck est créé, marqué reçu dans les listes.
#[test]
fn receive_version_of_an_unknown_pack_lists_it_as_received() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let r = receive(&lib, &manifest(id, "Reçu", None, "2026-10-01T10:00:00Z"));
    let info = lib.packs(PackKind::Widgets).into_iter().find(|p| p.id == id).unwrap();
    assert_that!((info.version, info.received)).is_equal_to((r.version.clone(), true));
    assert_that!(lib.is_received(id, &r.version)).is_true();
}

#[test]
fn receive_version_of_a_deck_only_pack_is_refused_and_writes_no_mark() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let mut m = manifest(id, "Importé", None, "2026-10-01T10:00:00Z");
    m.scope = oku_slides_core::PackScope::Deck;
    let err =
        lib.receive_version(id, &m.to_files(oku_slides_core::PackKind::Widgets, BTreeMap::new()).unwrap()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::DeckOnly);
    assert_that!(dir.path().join(id.to_string()).exists()).is_false();
}

#[test]
fn add_version_is_not_received() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let r = add_version(&lib, &manifest(id, "Mien", None, "2026-10-01T10:00:00Z"), &[]);
    assert_that!(lib.packs(PackKind::Widgets).into_iter().find(|p| p.id == id).map(|p| p.received))
        .is_equal_to(Some(false));
    assert_that!(lib.is_received(id, &r.version)).is_false();
}

/// t-355 : le repli de la version courante ne croit pas le `created` d'une
/// version reçue, même plus récent : la mienne reste devant.
#[test]
fn broken_current_falls_back_to_my_latest_version_over_a_received_one() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let mine = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), &[]);
    receive(&lib, &manifest(id, "Forgée", Some(&mine.version), "2099-01-01T00:00:00Z"));
    std::fs::write(dir.path().join(id.to_string()).join("current"), b"illisible").unwrap();
    assert_that!(library(dir.path()).current(id)).is_equal_to(Some(mine.version));
}

#[test]
fn broken_current_with_only_received_versions_takes_the_latest_received() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = receive(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"));
    let b = receive(&lib, &manifest(id, "B", Some(&a.version), "2026-10-02T10:00:00Z"));
    std::fs::write(dir.path().join(id.to_string()).join("current"), b"illisible").unwrap();
    assert_that!(library(dir.path()).current(id)).is_equal_to(Some(b.version));
}

/// t-355 : un dossier au nom d'UUID non canonique (majuscules) est ignoré
/// comme un dossier ancien, et n'est signalé qu'une fois.
#[test]
fn packs_ignore_a_non_canonical_uuid_folder() {
    let dir = fresh_dir("oku-slides-library-");
    let id = Uuid::new_v4();
    let upper = dir.path().join(id.to_string().to_uppercase());
    let lib = library(dir.path());
    add_version(&lib, &manifest(id, "Mien", None, "2026-10-01T10:00:00Z"), &[]);
    std::fs::rename(dir.path().join(id.to_string()), &upper).unwrap();
    assert_that!(library(dir.path()).packs(PackKind::Widgets).iter().any(|p| p.id == id)).is_false();
    assert_that!(first_legacy_sighting(&upper)).is_false();
}
