use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Barrier;

use oku_slides_core::{PackFiles, PackKind, PackVersion};
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::Storage;
use crate::library_kind::Widgets;
use crate::pack::BASE_PACK;
use crate::test_support::{builtin, fresh_dir, manifest, ns};
use crate::theme::{ThemeLibrary, ThemeManifest};

/// Une bibliothèque de packs de widgets sous `dir`.
fn widgets(dir: &Path) -> Library<Widgets> {
    Library::new(Some(dir.to_owned()), Storage::local().unwrap())
}

/// Les fichiers d'une version du pack de widgets `id`, sans widgets.
fn files(id: Uuid, name: &str, parent: Option<&PackVersion>) -> PackFiles {
    manifest(id, name, parent, "2026-10-24T10:00:00Z").to_files(PackKind::Widgets, BTreeMap::new()).unwrap()
}

/// Enregistre `name` comme brouillon `draft` de `id`, parente `base` sauf brouillon figé ; sa version.
fn save(
    lib: &Library<Widgets>,
    id: Uuid,
    draft: Option<&PackVersion>,
    base: Option<&PackVersion>,
    name: &str,
) -> PackVersion {
    let lock = draft_lock();
    let write = lib.draft_write(&lock, id, draft);
    let parent = match &write {
        DraftWrite::Child(v) => Some(v),
        DraftWrite::New | DraftWrite::Replace(_) => base,
    };
    lib.save_draft(&lock, id, &write, &files(id, name, parent)).unwrap()
}

#[test]
fn draft_write_without_draft_is_new() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    assert_that!(lib.draft_write(&draft_lock(), Uuid::new_v4(), None)).is_equal_to(DraftWrite::New);
}

#[test]
fn save_draft_unadopted_replaces_in_place() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let first = save(&lib, id, None, None, "Un");
    let second = save(&lib, id, Some(&first), None, "Deux");
    assert_that!(lib.version(id, &first)).is_none();
    assert_that!(lib.current(id)).is_equal_to(Some(second));
}

#[test]
fn save_draft_adopted_opens_child_version() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let first = save(&lib, id, None, None, "Un");
    lib.adopt(&draft_lock(), id, &files(id, "Un", None)).unwrap();
    let second = save(&lib, id, Some(&first), None, "Deux");
    assert_that!(lib.version(id, &first).is_some()).is_true();
    assert_that!(lib.current_version(id).unwrap().manifest.parent).is_equal_to(Some(first));
    assert_that!(lib.current(id)).is_equal_to(Some(second));
}

#[test]
fn adopt_marks_the_version_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = save(&lib, id, None, None, "Un");
    lib.adopt(&draft_lock(), id, &files(id, "Un", None)).unwrap();
    assert_that!(lib.is_frozen(id, &v)).is_true();
}

#[test]
fn unadopted_local_version_is_not_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = save(&lib, id, None, None, "Un");
    assert_that!(lib.is_frozen(id, &v)).is_false();
}

/// Une marque illisible (dossier de version sans droit de lecture) ne rend jamais remplaçable.
#[cfg(unix)]
#[test]
fn unreadable_marker_counts_as_frozen() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = save(&lib, id, None, None, "Un");
    let folder = dir.path().join(id.to_string()).join("versions").join(v.hex());
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o000)).unwrap();
    let write = lib.draft_write(&draft_lock(), id, Some(&v));
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_that!(write).is_equal_to(DraftWrite::Child(v));
}

#[test]
fn replace_version_never_deletes_a_frozen_old_version() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let old = save(&lib, id, None, None, "Un");
    lib.adopt(&draft_lock(), id, &files(id, "Un", None)).unwrap();
    lib.replace_version(id, &old, &files(id, "Deux", None)).unwrap();
    assert_that!(lib.version(id, &old).is_some()).is_true();
}

#[test]
fn builtin_version_is_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    assert_that!(lib.is_frozen(BASE_PACK, &builtin(BASE_PACK).version)).is_true();
}

#[test]
fn received_version_is_frozen() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = files(id, "Reçu", None);
    lib.receive_version(id, &v).unwrap();
    assert_that!(lib.is_frozen(id, &v.version())).is_true();
}

#[test]
fn adopt_is_idempotent() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let v = files(id, "Un", None);
    lib.adopt(&draft_lock(), id, &v).unwrap();
    assert_that!(lib.adopt(&draft_lock(), id, &v)).is_ok();
}

#[test]
fn adopt_restores_a_version_missing_from_the_library() {
    let dir = fresh_dir("oku-slides-drafts-");
    let lib = widgets(dir.path());
    let id = Uuid::new_v4();
    let first = save(&lib, id, None, None, "Un");
    let gone = files(id, "Ancien", None);
    lib.adopt(&draft_lock(), id, &gone).unwrap();
    assert_that!(lib.version(id, &gone.version()).is_some()).is_true();
    assert_that!(lib.current(id)).is_equal_to(Some(first));
}

/// Deux onglets (deux instances) : l'un enregistre sans arrêt, l'autre adopte ; rien d'adopté ne disparaît.
#[test]
fn adopt_then_save_concurrent_never_rewrites_adopted() {
    let dir = fresh_dir("oku-slides-drafts-");
    let id = Uuid::new_v4();
    let (editor, deck) = (widgets(dir.path()), widgets(dir.path()));
    let start = Barrier::new(2);
    let adopted: Vec<PackVersion> = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            start.wait();
            let mut draft = None;
            for n in 0..40 {
                draft = Some(save(&editor, id, draft.as_ref(), None, &format!("v{n}")));
            }
        });
        start.wait();
        let mut adopted = Vec::new();
        while !writer.is_finished() {
            std::thread::yield_now();
            let lock = draft_lock();
            if let Some(current) = deck.current_version(id) {
                deck.adopt(&lock, id, &current.files).unwrap();
                adopted.push(current.version);
            }
        }
        writer.join().unwrap();
        adopted
    });
    for v in &adopted {
        assert_that!(deck.version(id, v).is_some()).is_true();
    }
}

/// Un thème suit la même règle : adopté, son enregistrement suivant ouvre une fille.
#[test]
fn theme_save_draft_adopted_opens_child_version() {
    let dir = fresh_dir("oku-slides-drafts-themes-");
    let lib = ThemeLibrary::new(Some(dir.path().to_owned()), Storage::local().unwrap());
    let id = Uuid::new_v4();
    let theme = |name: &str, parent: Option<&PackVersion>| {
        ThemeManifest {
            id,
            name: name.into(),
            namespace: ns("marque"),
            extends: Vec::new(),
            description: String::new(),
            parent: parent.cloned(),
            created: "2026-10-24T10:00:00Z".into(),
            tokens: serde_json::json!({"color": {"accent": "#ff0000"}}).as_object().cloned().unwrap(),
        }
        .to_files()
        .unwrap()
    };
    let lock = draft_lock();
    let first = lib.save_draft(&lock, id, &DraftWrite::New, &theme("Un", None)).unwrap();
    lib.adopt(&lock, id, &theme("Un", None)).unwrap();
    let write = lib.draft_write(&lock, id, Some(&first));
    assert_that!(write).is_equal_to(DraftWrite::Child(first.clone()));
    lib.save_draft(&lock, id, &write, &theme("Deux", Some(&first))).unwrap();
    assert_that!(lib.version(id, &first).is_some()).is_true();
}
