//! t-273 : le dossier de données de la plateforme de test portait un nom
//! prévisible, `$TMPDIR/oku-slides-mcp-<pid>-<n>`. nextest lance un processus
//! par test (presque toujours `<n> = 0`) et macOS recycle les PID : un test
//! retombait sur le dossier d'un lancement précédent, avec son `resume/`
//! périmé, et ce dossier n'était jamais supprimé.

use std::path::PathBuf;

use speculoos::prelude::*;

use crate::common::{platform, start};

/// Les dossiers qu'aurait choisis l'ancien schéma dans ce processus, plantés
/// avec un `resume/` périmé. Seuls ceux créés ici sont supprimés à la fin.
struct Stale(Vec<PathBuf>);

impl Stale {
    fn plant() -> Self {
        let pid = std::process::id();
        let planted = (0..64)
            .map(|n| std::env::temp_dir().join(format!("oku-slides-mcp-{pid}-{n}")))
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

fn data_of(platform: &oku_slides_session::Platform) -> PathBuf {
    platform.settings.paths.data.clone().unwrap_or_else(|| panic!("plateforme de test sans dossier de données"))
}

#[test]
fn platform_with_stale_dirs_from_a_recycled_pid_gets_a_fresh_data_dir() {
    let _stale = Stale::plant();
    let (p, _data) = platform();
    let data = data_of(&p);
    asserting("le dossier de données d'un test est neuf").that(&data.join("resume").exists()).is_false();
    asserting("et il existe déjà : créé, pas seulement nommé").that(&data.is_dir()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn harness_removes_its_data_dir_when_dropped() {
    let h = start().await;
    let data = h.data().to_owned();
    asserting("le dossier de données vit autant que le serveur").that(&data.is_dir()).is_true();
    drop(h);
    asserting("puis il est supprimé avec lui").that(&data.exists()).is_false();
}
