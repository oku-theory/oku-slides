//! A pack tab chooses the theme its working deck is previewed in, silently.
//!
//! The choice is no undo step and no unsaved work, and writes no pack file or
//! library version: it is the tab's view, persisted with the tab (`pack/`).

use std::collections::BTreeMap;

use snafu::OptionExt;

use oku_slides_core::Op;

use crate::Session;
use crate::error::{Result, UnknownThemeSnafu};

impl Session {
    /// In a pack tab, previews the working deck in theme `theme`, silently.
    ///
    /// `theme` is the namespace [`crate::library_lookup::find_theme`] resolves. The pack's
    /// lease is checked like any edit: a free one is taken, its base frozen.
    ///
    /// # Errors
    ///
    /// `UnknownTheme`; the lease errors of `ensure_pack_lease` (`PackOpenInTab`,
    /// `UnknownPack`, `PackVersionMissing`); those of `apply_silent_ops`.
    pub(crate) fn set_preview_theme(&mut self, theme: &str) -> Result<()> {
        self.ensure_pack_lease()?;
        let ns = crate::library_lookup::find_theme(&self.themes, theme).context(UnknownThemeSnafu { theme })?;
        self.apply_silent_ops(vec![Op::SetTheme { theme: Some(ns), themes: BTreeMap::new() }])
    }
}

#[cfg(test)]
#[path = "preview_theme_tests.rs"]
mod preview_theme_tests;
