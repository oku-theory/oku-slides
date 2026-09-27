//! Chemins donnés par l'agent : absolus, sans `..`, normalisés.

use std::path::{Component, Path, PathBuf};

/// Le chemin, s'il est absolu et ne remonte pas ; les `.` sont retirés.
pub(crate) fn checked(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err(format!("path must be absolute: {path}"));
    }
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => return Err(format!("path must not contain '..': {path}")),
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
