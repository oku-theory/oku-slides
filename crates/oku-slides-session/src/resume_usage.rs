//! The resume's copy of a deck's agent usage: `deck/usage.jsonl`.
//!
//! Untrusted on read like the rest of the resume: a bounded read, bad lines
//! set aside (logged), never a resume that fails because of it.

use std::path::Path;

use oku_slides_core::{USAGE_FILE, USAGE_MAX_BYTES, UsageLedger};
use oku_slides_platform::storage::StorageErrorKind;
use snafu::ResultExt;

use super::{AccessSnafu, Result, Resume};

impl Resume {
    /// Writes `usage` as `<sub>/usage.jsonl`; an empty ledger removes the file.
    pub(crate) fn write_usage(&self, sub: &str, usage: &UsageLedger) -> Result<()> {
        let path = self.dir.join(sub).join(USAGE_FILE);
        if usage.is_empty() {
            return self.storage.delete(&path).context(AccessSnafu { path: &path });
        }
        self.put(&path, usage.to_jsonl())
    }

    /// Reads `dir/usage.jsonl`: missing, empty; unreadable or too large, empty and logged.
    pub(super) fn read_usage(&self, dir: &Path) -> UsageLedger {
        let path = dir.join(USAGE_FILE);
        let bytes = match self.storage.read_limited(&path, USAGE_MAX_BYTES) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == StorageErrorKind::NotFound => return UsageLedger::default(),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %oku_slides_platform::errors::one_line(&e),
                    "resume usage ledger unreadable: set aside"
                );
                return UsageLedger::default();
            }
        };
        let (ledger, issues) = UsageLedger::from_jsonl(&bytes);
        for issue in &issues {
            tracing::warn!(issue = %issue, "resume usage line set aside");
        }
        ledger
    }
}
