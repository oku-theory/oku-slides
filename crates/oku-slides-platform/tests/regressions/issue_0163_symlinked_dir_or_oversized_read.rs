//! t-163 : un dossier d'onglet `t-9` lien vers un dossier ailleurs ne fait
//! pas supprimer ce dossier ; un fichier trop gros n'est pas lu en entier.

use oku_slides_platform::storage::{Storage, StorageErrorKind};
use speculoos::prelude::*;

use crate::temp_dir;

#[cfg(unix)]
#[test]
fn delete_dir_symlinked_tab_dir_leaves_the_victim_intact() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("link");
    let base = tmp.path();
    let victim = base.join("victim");
    let tabs = base.join("resume/tabs");
    s.write(&victim.join("precious.txt"), b"keep".to_vec()).unwrap();
    s.create_dir_all(&tabs).unwrap();
    std::os::unix::fs::symlink(&victim, tabs.join("t-9")).unwrap();

    s.delete_dir(&tabs.join("t-9")).unwrap();

    assert_that!(s.read(&victim.join("precious.txt")).unwrap()).is_equal_to(b"keep".to_vec());
    assert_that!(tabs.join("t-9").symlink_metadata().is_err()).is_true();
}

#[test]
fn read_limited_oversized_file_is_refused() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("big");
    let dir = tmp.path();
    let path = dir.join("state.json");
    s.write(&path, vec![b' '; 2048]).unwrap();

    let err = s.read_limited(&path, 1024).unwrap_err();

    assert_that!(err.kind()).is_equal_to(StorageErrorKind::TooLarge);
}
