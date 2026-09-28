use std::path::PathBuf;

use speculoos::prelude::*;

use super::*;

/// Un dossier neuf avec deux fichiers et un sous-dossier, supprimé à la fin.
///
/// Créé atomiquement sous un nom aléatoire (t-273) : jamais celui d'un
/// lancement précédent, même quand macOS recycle le PID.
struct Dir(tempfile::TempDir);

impl Dir {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("oku-slides-app-launch-files-")
            .tempdir()
            .unwrap_or_else(|e| panic!("dossier de test : {e}"));
        std::fs::create_dir(dir.path().join("sub")).unwrap_or_else(|e| panic!("dossier de test : {e}"));
        std::fs::write(dir.path().join("a.pdf"), b"").unwrap_or_else(|e| panic!("fichier de test : {e}"));
        Self(dir)
    }

    fn path(&self) -> &std::path::Path {
        self.0.path()
    }
}

/// t-273 : les dossiers s'appelaient `oku-slides-app-launch-files-<pid>-<n>`.
/// nextest lance un processus par test (`<n> = 0`) et macOS recycle les PID :
/// un test retombait sur le dossier d'un lancement interrompu, avec son `bus`
/// périmé (`session_bus_with_a_runtime_dir_without_bus_is_absent` échouait).
#[test]
fn dir_new_with_stale_dirs_from_a_recycled_pid_is_fresh() {
    let pid = std::process::id();
    let planted: Vec<PathBuf> = (0..64)
        .map(|n| std::env::temp_dir().join(format!("oku-slides-app-launch-files-{pid}-{n}")))
        .filter(|dir| std::fs::create_dir(dir).is_ok())
        .inspect(|dir| std::fs::write(dir.join("bus"), b"").unwrap_or_else(|e| panic!("bus périmé : {e}")))
        .collect();
    let dir = Dir::new();
    let stale = dir.path().join("bus").exists();
    for p in &planted {
        std::fs::remove_dir_all(p).ok();
    }
    asserting("le dossier d'un test est neuf").that(&stale).is_false();
}

#[test]
fn launch_files_keeps_absolute_paths_and_joins_relative_ones_to_cwd() {
    let dir = Dir::new();
    let files = launch_files([PathBuf::from("a.pdf"), dir.path().join("a.pdf")].into_iter(), dir.path());
    assert_that!(files).is_equal_to(vec![dir.path().join("a.pdf"), dir.path().join("a.pdf")]);
}

#[test]
fn launch_files_drops_directories_and_missing_files() {
    let dir = Dir::new();
    let files = launch_files(
        [PathBuf::from("sub"), PathBuf::from("absent.pdf"), PathBuf::from("a.pdf")].into_iter(),
        dir.path(),
    );
    assert_that!(files).is_equal_to(vec![dir.path().join("a.pdf")]);
}

#[test]
fn session_bus_with_an_address_is_present() {
    assert_that!(session_bus(Some(std::ffi::OsStr::new("unix:path=/run/user/1000/bus")), None)).is_true();
}

#[test]
fn session_bus_with_a_runtime_dir_holding_a_bus_is_present() {
    let dir = Dir::new();
    std::fs::write(dir.path().join("bus"), b"").unwrap_or_else(|e| panic!("bus de test : {e}"));
    assert_that!(session_bus(None, Some(dir.path().as_os_str()))).is_true();
}

#[test]
fn session_bus_with_a_runtime_dir_without_bus_is_absent() {
    let dir = Dir::new();
    assert_that!(session_bus(None, Some(dir.path().as_os_str()))).is_false();
}

#[test]
fn session_bus_without_address_nor_runtime_dir_is_absent() {
    assert_that!(session_bus(None, None)).is_false();
}
