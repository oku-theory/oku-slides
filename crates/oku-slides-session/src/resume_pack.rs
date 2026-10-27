//! Resuming pack mode: a deck's workshop or a pack tab, checked against the library.
//!
//! `state.json` and the tab folder are untrusted. A pack tab's preview must
//! hold no file, hash or asset; its `pack/` must be the pack the state names,
//! of its kind, with the base it names. What the library knows (name,
//! built-in, scope) is recomputed; a session version that does not fit is
//! cleared. A pack the library lost is restored orphaned (read-only); a
//! lease held by another editor restores the tab read-only, never sets it
//! aside. A pack tab that fails a check is set aside (`ResumeError::PackTab`);
//! a workshop falls back to its deck. A state before version 5 held a theme
//! draft in its working deck: that deck takes the user deck's style back, and
//! a changed draft is said lost (`notice-theme-draft-dropped`).

use serde_json::{Map, Value};
use snafu::ResultExt;

use super::{
    DECK_DIR, DocIn, EDITOR_STATE_VERSION, MAX_JSON_BYTES, PACK_DIR, PackTabSnafu, Result, Resume, SavedMode,
    SessionSnafu, State,
};
use crate::agent::bounds::label;
use crate::deck_closure::combined_pins;
use crate::deck_theme::manifest_of;
use crate::error::SourcesSnafu;
use crate::pack::PackManifest;
use crate::pack_lease::PackLease;
use crate::pack_mode::embed_reached_themes;
use crate::pack_receive::is_library_pack;
use crate::pack_scratch::PackScratch;
use crate::theme_edit::{ThemeEdit, draft_differs};
use crate::views::{PackHost, PackMode, UiState};
use crate::{Document, Mode, PackLibrary, Session};

/// What a saved `document.json` claims, read before the document itself.
#[derive(Debug)]
pub(super) struct DocHeader {
    /// It names a file.
    pub(super) path: bool,
    /// It has a saved hash.
    pub(super) saved_hash: bool,
    /// The files in its `assets/`, dot-files aside.
    pub(super) assets: usize,
}

/// A pack state before version 5: its working deck had the theme draft as its style.
#[derive(Debug)]
pub(super) struct Legacy {
    /// The draft it held, if the state named one.
    draft: Option<ThemeEdit>,
}

/// The legacy part of `state`: `None` from version 5 on.
pub(super) fn legacy_of(state: &State) -> Option<Legacy> {
    (state.version < EDITOR_STATE_VERSION).then(|| Legacy { draft: state.theme_edit.clone() })
}

/// What the library says of a resumed workshop's pack.
#[derive(Debug)]
enum PackCheck {
    /// A deck-only pack: today's path, no lease.
    DeckOnly,
    /// A library pack whose checks passed: lease it.
    Library,
    /// A library pack the library no longer has (or not this base): read-only.
    Orphaned,
    /// An id, kind or scope mismatch, with its reason.
    Mismatch(&'static str),
}

/// Reads `dir`'s `document.json` and counts its assets, without its replica.
///
/// # Errors
///
/// The `document.json` cannot be read (at most `MAX_JSON_BYTES`) or parsed, or `assets/` listed.
///
/// # Blocking
///
/// Reads `document.json` and lists `assets/`.
pub(super) fn read_doc_header(resume: &Resume, dir: &str) -> Result<DocHeader> {
    let dir = resume.dir.join(dir);
    let path = dir.join("document.json");
    let bytes = resume.storage.read_limited(&path, MAX_JSON_BYTES).context(super::AccessSnafu { path: &path })?;
    let raw: DocIn = serde_json::from_slice(&bytes).context(super::JsonSnafu { path: &path })?;
    let assets = dir.join("assets");
    let listed = resume.storage.list_files(&assets).context(super::AccessSnafu { path: &assets })?;
    let named =
        |p: &std::path::PathBuf| p.file_name().is_some_and(|n| !n.is_empty() && !n.to_string_lossy().starts_with('.'));
    Ok(DocHeader {
        path: raw.path.is_some(),
        saved_hash: raw.saved_hash.is_some(),
        assets: listed.iter().filter(|p| named(p)).count(),
    })
}

/// Restores a pack tab from `state`: its preview, its pack, then pack mode.
///
/// # Errors
///
/// `PackTab` for every inconsistency of the folder or the state; the tail's
/// own errors (`Session`).
///
/// # Blocking
///
/// Reads the tab folder and the library, and renders.
pub(super) fn restore_pack_tab(s: &mut Session, resume: &Resume, state: State) -> Result<()> {
    let legacy = legacy_of(&state);
    let State { mode: SavedMode::Pack(edit), ui, .. } = state else {
        return PackTabSnafu { reason: "not a pack tab state" }.fail();
    };
    // An early exit only: the loaded document is checked again below.
    let header = read_doc_header(resume, DECK_DIR)?;
    snafu::ensure!(!header.path, PackTabSnafu { reason: "preview with a path" });
    snafu::ensure!(!header.saved_hash, PackTabSnafu { reason: "preview with a saved hash" });
    snafu::ensure!(header.assets == 0, PackTabSnafu { reason: "preview with assets" });
    let max = s.source_limits.replica();
    let pack_doc = resume.read_doc(PACK_DIR, &s.doc_defaults, max, &s.registry).map_err(|e| {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "pack de reprise illisible");
        PackTabSnafu { reason: "unreadable pack" }.build()
    })?;
    // A blob that does not match its key was dropped on reading: the base is then missing.
    snafu::ensure!(pack_doc.sources.packs.contains_key(&edit.base), PackTabSnafu { reason: "base missing" });
    let preview = resume.read_doc(DECK_DIR, &s.doc_defaults, max, &s.registry).map_err(|e| {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "aperçu de reprise illisible");
        PackTabSnafu { reason: "unreadable preview" }.build()
    })?;
    let bare = preview.path.is_none() && preview.saved_hash.is_none() && preview.sources.assets.is_empty();
    snafu::ensure!(bare, PackTabSnafu { reason: "preview document with a file or assets" });
    let pins = combined_pins(preview.sources.deck());
    let (widgets, notice) = s.resolve_widgets(&pins, &preview.sources.packs).context(SessionSnafu)?;
    s.install_document(preview, widgets, notice).context(SessionSnafu)?;
    s.finish_pack_restore(edit, pack_doc, legacy, ui)
}

impl Session {
    /// Pack mode resumed on `pack_doc` for `edit`, the current document set aside.
    ///
    /// For both hosts: checked against the library (`check_pack_doc`), the
    /// lease taken if free (`restore_lease`), a pack tab that never saved
    /// freezing its base, a `legacy` working deck restyled, then the workshop rendered.
    ///
    /// # Errors
    ///
    /// `PackTab` for a pack tab whose pack does not match; `Session`
    /// (restyle, widgets, render, freezing).
    ///
    /// # Blocking
    ///
    /// Reads the library and renders.
    pub(super) fn finish_pack_restore(
        &mut self,
        mut edit: PackMode,
        mut pack_doc: Document,
        legacy: Option<Legacy>,
        ui: UiState,
    ) -> Result<()> {
        let check = self.check_pack_doc(&mut edit, &pack_doc);
        if let PackCheck::Mismatch(reason) = check {
            if edit.host == PackHost::Tab {
                return PackTabSnafu { reason }.fail();
            }
            tracing::warn!(pack = %edit.pack, reason, "pack de reprise refusé : le deck est repris seul");
            self.ui = ui.sanitized(false);
            return Ok(());
        }
        let (lease, notice) = self.restore_lease(&edit, &check);
        // The library's bytes of the base, never `pack/`'s; a read-only tab freezes it later.
        if let Some(lease) = lease.as_ref().filter(|_| edit.host == PackHost::Tab && edit.session_version.is_none()) {
            self.freeze_base(lease, &edit.base).context(SessionSnafu)?;
        }
        let dropped = match legacy {
            Some(legacy) => self.restyle_legacy(&mut pack_doc, legacy.draft.as_ref()).context(SessionSnafu)?,
            None => false,
        };
        let stash = std::mem::replace(&mut self.doc, pack_doc);
        let orphaned = matches!(check, PackCheck::Orphaned);
        self.mode = Mode::Pack { stash: Box::new(stash), edit, scratch: PackScratch::default(), lease, orphaned };
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        if dropped {
            self.set_notice(self.localizer.tr("notice-theme-draft-dropped"));
        }
        self.sync_widgets().context(SessionSnafu)?;
        self.rerender().context(SessionSnafu)?;
        self.ui = ui.sanitized(true);
        Ok(())
    }

    /// Gives a legacy working deck `doc` the user deck's style back; true if `draft` changed.
    ///
    /// The theme, pinned themes and token overrides come from the current
    /// document (the deck, not yet set aside), with the theme versions they
    /// reach; no undo step. A draft whose base is unreadable, or whose tokens
    /// are neither an object nor null, counts as changed.
    fn restyle_legacy(&self, doc: &mut Document, draft: Option<&ThemeEdit>) -> crate::Result<bool> {
        let mut deck = doc.sources.deck().clone();
        let tokens = match &deck.tokens {
            Value::Object(map) => Some(map.clone()),
            Value::Null => Some(Map::new()),
            _ => {
                tracing::warn!("legacy working deck tokens are not an object: its theme draft counts as changed");
                None
            }
        };
        let base = |d: &ThemeEdit| {
            let held = [&doc.sources.themes, &self.doc.sources.themes];
            held.into_iter().find_map(|t| manifest_of(&d.base, t.get(&d.base.version)?).ok())
        };
        let differs = |d: &ThemeEdit, tokens: &Map<String, Value>| base(d).is_none_or(|m| draft_differs(d, &m, tokens));
        let changed = draft.is_some_and(|d| tokens.as_ref().is_none_or(|t| differs(d, t)));
        let user = self.doc.sources.deck();
        (deck.theme, deck.themes, deck.tokens) = (user.theme.clone(), user.themes.clone(), user.tokens.clone());
        doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        embed_reached_themes(&mut doc.sources, &self.doc.sources.themes);
        Ok(changed)
    }

    /// Checks `pack_doc` against `edit` and the library; recomputes what the library knows.
    ///
    /// `edit`'s name, built-in and scope are recomputed, its session version
    /// cleared unless it is the base and descends from the saved parent.
    fn check_pack_doc(&self, edit: &mut PackMode, pack_doc: &Document) -> PackCheck {
        let packs = &pack_doc.sources.packs;
        let Some(base) = packs.get(&edit.base) else { return PackCheck::Mismatch("base missing") };
        let Ok(manifest) = PackManifest::from_files(base) else {
            return PackCheck::Mismatch("unreadable base manifest");
        };
        if manifest.id != edit.pack {
            return PackCheck::Mismatch("base of another pack");
        }
        if base.kind() != edit.kind {
            return PackCheck::Mismatch("kind differs");
        }
        // The version saved, compared before it may be cleared below.
        let saved = edit.session_version.clone();
        let pinned = pack_doc.sources.deck().all_pins().find(|(_, _, p)| p.id == edit.pack).map(|(_, _, p)| &p.version);
        if pinned != Some(&edit.base) && (saved.is_none() || pinned != saved.as_ref()) {
            return PackCheck::Mismatch("edited pin is neither the base nor the saved version");
        }
        let saved_manifest = saved.as_ref().and_then(|v| packs.get(v)).map(PackManifest::from_files);
        if saved_manifest.as_ref().is_some_and(|m| !m.as_ref().is_ok_and(|m| m.id == edit.pack)) {
            return PackCheck::Mismatch("saved version of another pack");
        }
        let listed = self.library.current(edit.pack).is_some();
        if !is_library_pack(base) {
            if edit.host == PackHost::Tab {
                return PackCheck::Mismatch(if listed {
                    "deck-scoped base of a library pack"
                } else {
                    "deck-only pack in a pack tab"
                });
            }
            if listed {
                return PackCheck::Mismatch("deck-scoped base of a library pack");
            }
            recompute(edit, &manifest, true);
            return PackCheck::DeckOnly;
        }
        if listed && self.library.kind_of(edit.pack) != edit.kind {
            return PackCheck::Mismatch("kind differs from the library");
        }
        let check = if !listed || !self.library.lists(edit.pack, &edit.base) {
            PackCheck::Orphaned
        } else if edit.parent.as_ref().is_some_and(|p| {
            !matches!(
                self.library.relation(edit.pack, p, &edit.base),
                crate::Relation::Same | crate::Relation::Ancestor
            )
        }) {
            return PackCheck::Mismatch("parent not an ancestor of the base");
        } else {
            PackCheck::Library
        };
        let saved_parent = saved_manifest.and_then(Result::ok).map(|m| m.parent);
        if saved.is_some() && (saved.as_ref() != Some(&edit.base) || saved_parent.as_ref() != Some(&edit.parent)) {
            tracing::warn!(pack = %edit.pack, "version de séance reprise étrangère : la prochaine sera neuve");
            edit.session_version = None;
        }
        recompute(edit, &manifest, false);
        check
    }

    /// The lease a resumed editor takes, and the notice it leaves.
    ///
    /// A lease held by another editor restores it read-only, with
    /// `notice-pack-edited-elsewhere`; an orphaned one gets `notice-pack-deleted`.
    fn restore_lease(&self, edit: &PackMode, check: &PackCheck) -> (Option<PackLease>, Option<String>) {
        let name = label(&edit.name);
        match check {
            PackCheck::DeckOnly | PackCheck::Mismatch(_) => (None, None),
            PackCheck::Orphaned => {
                (None, Some(self.localizer.tr_with("notice-pack-deleted", &[("pack", name.as_str().into())])))
            }
            PackCheck::Library => {
                let Some(scope) = self.library.lease_scope() else { return (None, None) };
                // Host `Deck`: the deck is the current document, not yet set aside.
                let holder =
                    if edit.host == PackHost::Tab { name.clone() } else { label(&self.document_name(&self.doc)) };
                match PackLease::acquire(scope, edit.pack, &name, &holder) {
                    Ok(lease) => (Some(lease), None),
                    Err(e) => {
                        if let Some(tab) = e.lease_holder() {
                            let args = [("pack", name.as_str().into()), ("tab", tab.into())];
                            return (None, Some(self.localizer.tr_with("notice-pack-edited-elsewhere", &args)));
                        }
                        tracing::error!(error = %e.one_line(), "bail de reprise refusé");
                        (None, None)
                    }
                }
            }
        }
    }
}

/// Recomputes `edit`'s name, built-in flag and scope from the library's facts, logging a change.
fn recompute(edit: &mut PackMode, manifest: &PackManifest, deck_only: bool) {
    let builtin = PackLibrary::is_builtin(edit.pack);
    if edit.name != manifest.name || edit.builtin != builtin || edit.deck_only != deck_only {
        tracing::warn!(pack = %edit.pack, "pack de reprise : nom, intégré ou portée recalculés");
    }
    edit.name.clone_from(&manifest.name);
    edit.builtin = builtin;
    edit.deck_only = deck_only;
}

#[cfg(test)]
#[path = "resume_pack_tests.rs"]
mod resume_pack_tests;

#[cfg(test)]
#[path = "resume_pack_refusal_tests.rs"]
mod resume_pack_refusal_tests;

#[cfg(test)]
#[path = "resume_pack_legacy_tests.rs"]
mod resume_pack_legacy_tests;
