//! Reading a resume JSON file whose version lies in a range.
//!
//! `state.json` is read at version 3 (deck, help, in-deck workshop) or 4 (a
//! pack tab); any other version is an error, checked before the rest is read.

use std::ops::RangeInclusive;

use serde::de::DeserializeOwned;
use snafu::{ResultExt, ensure};

use super::{AccessSnafu, JsonSnafu, MAX_JSON_BYTES, Result, Resume, VersionSnafu, Versioned};

impl Resume {
    /// Rereads `name` if it exists; a version outside `versions` is an error.
    ///
    /// Above [`MAX_JSON_BYTES`], the file is unreadable, without being read.
    pub(crate) fn read_json_versions<T: DeserializeOwned>(
        &self,
        name: &str,
        versions: RangeInclusive<u32>,
    ) -> Result<Option<T>> {
        let path = self.dir.join(name);
        if !self.storage.exists(&path).context(AccessSnafu { path: &path })? {
            return Ok(None);
        }
        let bytes = self.storage.read_limited(&path, MAX_JSON_BYTES).context(AccessSnafu { path: &path })?;
        // La version se lit à part : une version future peut avoir changé de
        // forme, illisible en `T` alors que sa version l'est bien.
        let versioned: Versioned = serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?;
        ensure!(versions.contains(&versioned.version), VersionSnafu { version: versioned.version });
        Ok(Some(serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?))
    }
}

#[cfg(test)]
#[path = "resume_versions_tests.rs"]
mod resume_versions_tests;
