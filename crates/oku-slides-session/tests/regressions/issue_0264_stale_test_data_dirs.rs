//! t-264 : les dossiers de données des tests portaient un nom prévisible,
//! `$TMPDIR/oku-slides-session-<pid>-<n>`. nextest lance un processus par test
//! (presque toujours `<n> = 0`) et macOS recycle les PID : un test retombait
//! sur le dossier d'un lancement interrompu, avec son `resume/` périmé, et
//! échouait au hasard (« reprise refusée », `resume/` déjà présent). Et
//! `session()` lâchait son dossier dès la session créée : supprimé trop tôt,
//! recréé par la session, puis jamais nettoyé.

use std::path::PathBuf;

use speculoos::prelude::*;

use crate::common::{data_dir, session};

/// Les dossiers qu'aurait choisis l'ancien schéma dans ce processus, plantés
/// avec un `resume/` périmé. Seuls ceux créés ici sont supprimés à la fin.
struct Stale(Vec<PathBuf>);

impl Stale {
    fn plant() -> Self {
        let pid = std::process::id();
        let planted = (0..64)
            .map(|n| std::env::temp_dir().join(format!("oku-slides-session-{pid}-{n}")))
            .filter(|dir| std::fs::create_dir(dir).is_ok())
            .inspect(|dir| {
                std::fs::create_dir(dir.join("resume"))
                    .unwrap_or_else(|e| panic!("resume/ périmé impossible à planter : {e}"));
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
fn data_dir_with_stale_dirs_from_a_recycled_pid_is_empty() {
    let _stale = Stale::plant();
    let data = data_dir();
    let entries = std::fs::read_dir(&*data).map_or(0, Iterator::count);
    asserting("le dossier de données d'un test est neuf").that(&entries).is_equal_to(0);
    asserting("et il existe déjà : créé, pas seulement nommé").that(&data.is_dir()).is_true();
}

#[test]
fn session_keeps_its_data_dir_until_dropped() {
    let s = session();
    let data = s.data().to_owned();
    asserting("le dossier de données vit autant que la session").that(&data.is_dir()).is_true();
    drop(s);
    asserting("puis il est supprimé avec elle").that(&data.exists()).is_false();
}
