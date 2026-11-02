//! The tab's last render error and last import error, kept for the `diagnose` tool.
//!
//! In memory only, one line each (`Error::one_line`): never resumed nor persisted.

use crate::Session;

/// The last failures the tab showed: a deck render, an import.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LastErrors {
    /// The last deck render's error; cleared by a successful one.
    render: Option<String>,
    /// The last failed import started from this tab.
    import: Option<String>,
}

impl Session {
    /// Records the outcome of a deck render: its error on one line, or none after a success.
    pub(crate) fn note_render<T>(&mut self, outcome: &crate::Result<T>) {
        self.last_errors.render = outcome.as_ref().err().map(crate::SessionError::one_line);
    }

    /// Records a failed import started from this tab, on one line.
    pub(crate) fn note_import_error(&mut self, error: &crate::SessionError) {
        self.last_errors.import = Some(error.one_line());
    }

    /// The last deck render's error, on one line; `None` once a render succeeded.
    #[must_use]
    pub fn last_render_error(&self) -> Option<&str> {
        self.last_errors.render.as_deref()
    }

    /// The last failed import started from this tab, on one line.
    #[must_use]
    pub fn last_import_error(&self) -> Option<&str> {
        self.last_errors.import.as_deref()
    }
}

#[cfg(test)]
#[path = "last_errors_tests.rs"]
mod last_errors_tests;
