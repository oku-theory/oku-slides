//! Recherche d'un programme externe, par son chemin ou dans `PATH`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::storage::Storage;

/// Le programme `name` s'il existe, comme le trouverait son lancement.
///
/// Un nom avec un séparateur (chemin absolu ou relatif) est pris tel quel ; un nom seul
/// est cherché dans les dossiers absolus de `path_var` (la variable `PATH`), dans l'ordre :
/// une entrée vide ou relative est sautée. Seul un fichier compte, pas un dossier. Ne
/// vérifie pas que le fichier est exécutable : son lancement le dira.
pub fn find_program(storage: &Storage, name: &Path, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let is_file = |p: &Path| match storage.is_file(p) {
        Ok(found) => found,
        Err(e) => {
            tracing::debug!(path = %p.display(), error = %e, "programme candidat illisible : sauté");
            false
        }
    };
    if name.components().count() > 1 || name.is_absolute() {
        return is_file(name).then(|| name.to_path_buf());
    }
    let dirs = path_var.map(std::env::split_paths)?;
    dirs.filter(|d| d.is_absolute()).map(|d| d.join(name)).find(|p| is_file(p))
}

#[cfg(test)]
#[path = "program_tests.rs"]
mod program_tests;
