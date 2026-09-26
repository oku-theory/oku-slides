use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use speculoos::prelude::*;

use super::*;

/// Un dossier neuf avec deux fichiers et un sous-dossier, supprimé à la fin.
struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "oku-slides-app-launch-files-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap_or_else(|e| panic!("dossier de test : {e}"));
        std::fs::write(dir.join("a.pdf"), b"").unwrap_or_else(|e| panic!("fichier de test : {e}"));
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn launch_files_keeps_absolute_paths_and_joins_relative_ones_to_cwd() {
    let dir = Dir::new();
    let files = launch_files([PathBuf::from("a.pdf"), dir.0.join("a.pdf")].into_iter(), &dir.0);
    assert_that!(files).is_equal_to(vec![dir.0.join("a.pdf"), dir.0.join("a.pdf")]);
}

#[test]
fn launch_files_drops_directories_and_missing_files() {
    let dir = Dir::new();
    let files =
        launch_files([PathBuf::from("sub"), PathBuf::from("absent.pdf"), PathBuf::from("a.pdf")].into_iter(), &dir.0);
    assert_that!(files).is_equal_to(vec![dir.0.join("a.pdf")]);
}

#[test]
fn session_bus_with_an_address_is_present() {
    assert_that!(session_bus(Some(std::ffi::OsStr::new("unix:path=/run/user/1000/bus")), None)).is_true();
}

#[test]
fn session_bus_with_a_runtime_dir_holding_a_bus_is_present() {
    let dir = Dir::new();
    std::fs::write(dir.0.join("bus"), b"").unwrap_or_else(|e| panic!("bus de test : {e}"));
    assert_that!(session_bus(None, Some(dir.0.as_os_str()))).is_true();
}

#[test]
fn session_bus_with_a_runtime_dir_without_bus_is_absent() {
    let dir = Dir::new();
    assert_that!(session_bus(None, Some(dir.0.as_os_str()))).is_false();
}

#[test]
fn session_bus_without_address_nor_runtime_dir_is_absent() {
    assert_that!(session_bus(None, None)).is_false();
}
