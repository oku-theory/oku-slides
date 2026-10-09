//! Chemins donnés par l'agent : absolus, sans `..`, normalisés.

use std::path::{Component, Path, PathBuf};

use snafu::ensure;

use crate::tool_error::{PathParentSnafu, PathRelativeSnafu, ToolError};

/// Le chemin, s'il est absolu et ne remonte pas ; les `.` sont retirés. Le refus ne dit pas
/// le chemin à l'agent (il reste au journal).
pub(crate) fn checked(path: &str) -> Result<PathBuf, ToolError> {
    let p = Path::new(path);
    ensure!(p.is_absolute(), PathRelativeSnafu { path });
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => return PathParentSnafu { path }.fail(),
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
