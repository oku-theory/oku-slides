//! t-273 : les dossiers de travail des tests de dictée portaient un nom
//! prévisible, `$TMPDIR/oku-slides-dictation-<nom>-<pid>-<n>`. nextest lance
//! un processus par test (presque toujours `<n> = 0`) et macOS recycle les
//! PID : un test retombait sur le dossier d'un lancement précédent, avec ses
//! modèles et son WAV périmés.

use std::path::PathBuf;

use speculoos::prelude::*;

use crate::common::workdir;

/// Les dossiers qu'aurait choisis l'ancien schéma dans ce processus, plantés
/// avec un modèle périmé. Seuls ceux créés ici sont supprimés à la fin.
struct Stale(Vec<PathBuf>);

impl Stale {
    fn plant(name: &str) -> Self {
        let pid = std::process::id();
        let planted = (0..64)
            .map(|n| std::env::temp_dir().join(format!("oku-slides-dictation-{name}-{pid}-{n}")))
            .filter(|dir| std::fs::create_dir(dir).is_ok())
            .inspect(|dir| {
                std::fs::create_dir(dir.join("models"))
                    .unwrap_or_else(|e| panic!("modèle périmé impossible à planter : {e}"));
            })
            .collect();
        Self(planted)
    }
}

impl Drop for Stale {
    fn drop(&mut self) {
        for dir in &self.0 {
            std::fs::remove_dir_all(dir).ok();
        }
    }
}

#[test]
fn workdir_with_stale_dirs_from_a_recycled_pid_is_empty() {
    let _stale = Stale::plant("regression");
    let dir = workdir("regression");
    let entries = std::fs::read_dir(&*dir).map(Iterator::count).unwrap_or(0);
    asserting("le dossier de travail d'un test est neuf").that(&entries).is_equal_to(0);
    asserting("et il existe déjà : créé, pas seulement nommé").that(&dir.is_dir()).is_true();
}

#[test]
fn workdir_is_removed_when_dropped() {
    let dir = workdir("drop");
    let path = dir.to_path_buf();
    std::fs::write(path.join("voix.wav"), b"").unwrap_or_else(|e| panic!("fichier de test : {e}"));
    drop(dir);
    asserting("le dossier de travail est supprimé à la fin du test").that(&path.exists()).is_false();
}
