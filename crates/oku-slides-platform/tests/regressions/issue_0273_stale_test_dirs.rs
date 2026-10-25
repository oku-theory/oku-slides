//! t-273 : les dossiers des tests de non-régression portaient un nom
//! prévisible, `$TMPDIR/oku-slides-regression-0163-<nom>-<pid>`. nextest lance
//! un processus par test et macOS recycle les PID : un test retombait sur le
//! dossier d'un lancement précédent, avec ses fichiers et liens périmés.

use std::path::PathBuf;

use speculoos::prelude::*;

use crate::temp_dir;

/// Le dossier qu'aurait choisi l'ancien schéma dans ce processus, planté avec
/// un fichier périmé. Supprimé à la fin s'il a été créé ici.
struct Stale(Option<PathBuf>);

impl Stale {
    fn plant(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("oku-slides-regression-0163-{name}-{}", std::process::id()));
        let planted = std::fs::create_dir(&dir).is_ok().then(|| {
            std::fs::write(dir.join("state.json"), b"{}")
                .unwrap_or_else(|e| panic!("fichier périmé impossible à planter : {e}"));
            dir
        });
        Self(planted)
    }
}

impl Drop for Stale {
    fn drop(&mut self) {
        if let Some(dir) = &self.0 {
            std::fs::remove_dir_all(dir).ok();
        }
    }
}

#[test]
fn temp_dir_with_a_stale_dir_from_a_recycled_pid_is_empty() {
    let _stale = Stale::plant("stale");
    let dir = temp_dir("stale");
    let entries = std::fs::read_dir(dir.path()).map_or(0, Iterator::count);
    asserting("le dossier d'un test est neuf").that(&entries).is_equal_to(0);
    asserting("et il existe déjà : créé, pas seulement nommé").that(&dir.path().is_dir()).is_true();
}

#[test]
fn temp_dir_is_removed_when_dropped() {
    let dir = temp_dir("drop");
    let path = dir.path().to_path_buf();
    std::fs::write(path.join("state.json"), b"{}").unwrap_or_else(|e| panic!("fichier de test : {e}"));
    drop(dir);
    asserting("le dossier est supprimé à la fin du test").that(&path.exists()).is_false();
}
