//! Outillage partagé par les tests d'intégration de `oku-slides-stt`
//! (`dictation.rs`, `regressions/`).

use std::path::Path;

/// Un dossier de travail neuf, supprimé à la fin du test.
///
/// Créé atomiquement sous un nom aléatoire (t-273) : jamais celui d'un
/// lancement précédent, même quand macOS recycle le PID.
#[derive(Debug)]
pub struct Workdir(tempfile::TempDir);

impl std::ops::Deref for Workdir {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.0.path()
    }
}

pub fn workdir(name: &str) -> Workdir {
    let dir = tempfile::Builder::new()
        .prefix(&format!("oku-slides-dictation-{name}-"))
        .tempdir()
        .unwrap_or_else(|e| panic!("dossier de test impossible : {e}"));
    Workdir(dir)
}
