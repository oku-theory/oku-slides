//! Resuming a theme tab: its draft, checked against the library, on a new specimen deck.
//!
//! A theme tab writes no document, only `state.json` at version 5: the
//! draft (`SavedMode::Theme`), the interface state and the draft's tokens.
//! The state is untrusted. The tab must hold the theme `workspace.json`
//! cached for it; its base version must be in the library (a theme or base
//! gone makes the tab unreadable, never orphaned); the draft's namespace
//! must be its base's (`ThemeBase`); its parents must pass
//! `check_theme_extends`; its parent must be the base or an ancestor of it,
//! and is absent only when the base has none; the tokens that differ from
//! the base's must pass a non-strict check on the specimen deck's schema.
//! The base's pin and a session version that is not the base are
//! recomputed, logged. Only then are the tokens applied (no undo step), the
//! lease retaken (held elsewhere: read-only, `notice-theme-edited-elsewhere`)
//! and an unwritten tab's base frozen.

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::Op;

use super::{EDITOR_STATE_VERSION, Result, SavedMode, SessionSnafu, State, ThemeBaseSnafu, ThemeTabSnafu};
use crate::agent::bounds::label;
use crate::error::SourcesSnafu;
use crate::pack_lease::PackLease;
use crate::resume_writer::{AgentWrite, Job, Writer};
use crate::theme::ThemeManifest;
use crate::theme_closure::pin_of;
use crate::theme_edit::{draft_differs, tokens_value};
use crate::theme_schema::{default_tokens, merge};
use crate::theme_tab::ThemeTab;
use crate::{Mode, Relation, Session, TabDocument};

/// Restores a theme tab from `state`, refusing it unless it holds `cached`'s theme.
///
/// # Errors
///
/// `ThemeTab` or `ThemeBase` for every inconsistency of the state against
/// the library; `Session` for the extends, token, render and freezing errors.
///
/// # Blocking
///
/// Reads the theme library, renders, and may take `DRAFTS` for the lease and the freeze.
pub(super) fn restore_theme_tab(s: &mut Session, state: State, cached: Option<&TabDocument>) -> Result<()> {
    let State { version, mode: SavedMode::Theme(mut edit), ui, draft_tokens, .. } = state else {
        return ThemeTabSnafu { reason: "not a theme tab state" }.fail();
    };
    ensure!(version == EDITOR_STATE_VERSION, ThemeTabSnafu { reason: "theme tab state before version 5" });
    let id = edit.draft.base.id;
    let held = cached.is_none_or(|doc| *doc == TabDocument::Theme { theme: id });
    ensure!(held, ThemeTabSnafu { reason: "cached document of another theme" });
    let tokens = draft_tokens.context(ThemeTabSnafu { reason: "no draft tokens" })?;
    let base = s.check_theme_state(&mut edit)?;
    // The tokens the draft starts from: none for a child of a built-in.
    let origin = if edit.draft.child() { Map::new() } else { base.tokens.clone() };
    let unwritten = edit.session_version.is_none();
    s.install_theme_tab(edit, origin.clone(), None).context(SessionSnafu)?;
    if let Mode::Theme { edit, .. } = &s.mode {
        s.check_theme_extends(&edit.draft, &edit.draft.extends).context(SessionSnafu)?;
    }
    s.check_draft_tokens(&origin, &tokens).context(SessionSnafu)?;
    s.apply_draft_tokens(&tokens).context(SessionSnafu)?;
    // No document keeps the flag: unsaved work is a draft that differs from its base.
    s.doc.dirty = match &s.mode {
        Mode::Theme { edit, .. } => draft_differs(&edit.draft, &base, &tokens),
        _ => false,
    };
    s.retake_theme_lease(unwritten)?;
    s.rerender().context(SessionSnafu)?;
    s.ui = ui.sanitized(false);
    Ok(())
}

/// The leaves of `draft` whose value differs from `origin`'s, as a token tree.
fn differing(draft: &Map<String, Value>, origin: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for (key, value) in draft {
        match (value, origin.get(key)) {
            (v, Some(o)) if v == o => {}
            (Value::Object(d), Some(Value::Object(o))) => {
                let inner = differing(d, o);
                if !inner.is_empty() {
                    out.insert(key.clone(), Value::Object(inner));
                }
            }
            (v, _) => {
                out.insert(key.clone(), v.clone());
            }
        }
    }
    out
}

impl Session {
    /// Checks `edit` against the library before anything is installed; its base's manifest.
    ///
    /// Recomputes the base's pin from the library, and clears a session version that is not the base.
    fn check_theme_state(&self, edit: &mut ThemeTab) -> Result<ThemeManifest> {
        let (id, version) = (edit.draft.base.id, edit.draft.base.version.clone());
        let files = self.themes.version(id, &version).context(ThemeTabSnafu { reason: "base version unknown" })?;
        let base = ThemeManifest::from_files(&files).ok().context(ThemeTabSnafu { reason: "unreadable base" })?;
        ensure!(base.id == id, ThemeTabSnafu { reason: "base of another theme" });
        ensure!(base.namespace == edit.draft.namespace, ThemeBaseSnafu { namespace: edit.draft.namespace.as_str() });
        let recomputed = pin_of(&base, version.clone());
        if recomputed != edit.draft.base {
            tracing::warn!(theme = %id, "resumed theme pin recomputed from the library");
            edit.draft.base = recomputed;
        }
        let base_version = &version;
        match &edit.parent {
            Some(parent) => {
                let relation = self.themes.relation(id, parent, base_version);
                let ancestor = matches!(relation, Relation::Same | Relation::Ancestor);
                ensure!(ancestor, ThemeTabSnafu { reason: "parent not an ancestor of the base" });
            }
            None => ensure!(base.parent.is_none(), ThemeTabSnafu { reason: "no parent for a base that has one" }),
        }
        if edit.session_version.as_ref().is_some_and(|v| v != base_version) {
            tracing::warn!(theme = %id, "resumed session version is not the base: the next write is new");
            edit.session_version = None;
        }
        Ok(base)
    }

    /// Checks the draft tokens that differ from `origin`, as one non-strict `SetTokens`.
    ///
    /// The rule of opening a theme: a token the specimen deck's schema does
    /// not know is kept, a known color or palette must be valid.
    fn check_draft_tokens(&self, origin: &Map<String, Value>, tokens: &Map<String, Value>) -> crate::Result<()> {
        let deck = self.doc.sources.deck();
        let mut lower = default_tokens().clone();
        merge(&mut lower, &self.theme_layer(deck)?);
        merge(&mut lower, origin);
        let schema = (self.token_schema(deck), self.schema_namespaces(deck));
        let ops = [Op::SetTokens { tokens: Value::Object(differing(tokens, origin)) }];
        self.check_token_ops_on(&schema, &lower, &deck.tokens, &ops, false)
    }

    /// The specimen deck takes `tokens`, with no undo step.
    fn apply_draft_tokens(&mut self, tokens: &Map<String, Value>) -> crate::Result<()> {
        let mut deck = self.doc.sources.deck().clone();
        deck.tokens = tokens_value(tokens.clone());
        Ok(self.doc.sources.apply_silent(deck).context(SourcesSnafu)?)
    }

    /// Retakes the theme's lease; held elsewhere, the tab is read-only and says so.
    ///
    /// Holding it, a tab that never wrote freezes its base.
    fn retake_theme_lease(&mut self, unwritten: bool) -> Result<()> {
        let Mode::Theme { edit, .. } = &self.mode else { return Ok(()) };
        let Some(scope) = self.library.theme_scope() else { return Ok(()) };
        let (id, name) = (edit.draft.base.id, label(&edit.draft.name));
        let lease = match PackLease::acquire(scope, id, &name, &name) {
            Ok(lease) => lease,
            Err(e) => {
                if let Some(tab) = e.lease_holder() {
                    let args = [("theme", name.as_str().into()), ("tab", tab.into())];
                    let notice = self.localizer.tr_with("notice-theme-edited-elsewhere", &args);
                    self.set_notice(notice);
                } else {
                    tracing::error!(error = %e.one_line(), "theme tab lease refused on resume");
                }
                return Ok(());
            }
        };
        if unwritten {
            self.freeze_theme_base(&lease).context(SessionSnafu)?;
        }
        if let Mode::Theme { lease: held, .. } = &mut self.mode {
            *held = Some(lease);
        }
        Ok(())
    }

    /// Sends a theme tab's state to `writer`: version 5, its draft and tokens, no document.
    pub(super) fn persist_theme(&self, writer: &Writer, agent: Option<AgentWrite>) {
        let Mode::Theme { edit, .. } = &self.mode else { return };
        let state = State {
            version: EDITOR_STATE_VERSION,
            mode: SavedMode::Theme(edit.clone()),
            ui: self.ui.clone(),
            help_theme: None,
            theme_edit: None,
            draft_tokens: Some(self.edited_tokens()),
        };
        writer.send(Job { docs: Vec::new(), state, agent });
    }
}

#[cfg(test)]
#[path = "resume_theme_tests.rs"]
mod resume_theme_tests;
