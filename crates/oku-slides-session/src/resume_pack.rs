//! Resuming a pack tab, checked against the library.
//!
//! `state.json` and the tab folder are untrusted. A pack tab's preview must
//! hold no file, hash or asset; its `pack/` must be the pack the state names,
//! of its kind, with the base it names. What the library knows (name,
//! built-in, scope) is recomputed; a session version that does not fit is
//! cleared. A pack the library lost is restored orphaned (read-only); a
//! lease held by another editor restores the tab read-only, never sets it
//! aside. A pack tab that fails a check is set aside (`ResumeError::PackTab`).
//! A version-4 state held a theme draft in its working deck: that deck takes
//! the user deck's style back, and a changed draft is said lost
//! (`notice-theme-draft-dropped`). Workshops reopen on their deck (`resume_workshop.rs`).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::workshop::LegacyHost;
use super::{
    DECK_DIR, DocIn, MAX_JSON_BYTES, PACK_DIR, PACK_TAB_STATE_VERSION, PackTabSnafu, Result, Resume, SavedMode,
    SessionSnafu, State,
};
use crate::agent::bounds::label;
use crate::deck_closure::combined_pins;
use crate::deck_pack_tab::{SendBase, TabLink};
use crate::deck_theme::manifest_of;
use crate::error::SourcesSnafu;
use crate::pack::PackManifest;
use crate::pack_lease::PackLease;
use crate::pack_mode::embed_reached_themes;
use crate::pack_receive::is_library_pack;
use crate::pack_scratch::PackScratch;
use crate::theme_edit::{ThemeEdit, draft_differs};
use crate::views::{PackMode, UiState};
use crate::{Document, Mode, PackLibrary, Session, TabDocument};

/// A saved pack mode with what older versions and the outbox add.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SavedPack {
    /// The pack being edited.
    #[serde(flatten)]
    pub edit: PackMode,
    /// The host an older state saved, read by `classify`; never written.
    #[serde(default, rename = "host", skip_serializing)]
    pub(super) legacy: Option<LegacyHost>,
    /// The base of a deck-pack tab's save its owner had not acknowledged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unforwarded: Option<SendBase>,
}

impl SavedPack {
    /// A pack state to write, from the live mode.
    #[must_use]
    pub(crate) fn new(edit: PackMode, unforwarded: Option<SendBase>) -> Self {
        Self { edit, legacy: None, unforwarded }
    }
}

/// The pack state `s` saves for `edit`.
pub(super) fn saved_pack(s: &Session, edit: &PackMode) -> SavedPack {
    SavedPack::new(edit.clone(), s.deck_pack_unforwarded())
}

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

/// A version-4 pack tab state: its working deck had the theme draft as its style.
#[derive(Debug)]
pub(super) struct Legacy {
    /// The draft it held, if the state named one.
    draft: Option<ThemeEdit>,
}

/// The legacy part of `state`: `Some` only for a version-4 pack tab state.
pub(super) fn legacy_of(state: &State) -> Option<Legacy> {
    (state.version == PACK_TAB_STATE_VERSION).then(|| Legacy { draft: state.theme_edit.clone() })
}

/// What the library says of a resumed pack tab's pack.
#[derive(Debug)]
enum PackCheck {
    /// A deck-only pack of a deck-pack tab: no lease.
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
/// `cached` is the tab's document in `workspace.json`: a deck-pack tab's
/// names its owner and pack (checked by `check_pack_doc`, fail closed).
///
/// # Errors
///
/// `PackTab` for every inconsistency of the folder or the state; the tail's
/// own errors (`Session`).
///
/// # Blocking
///
/// Reads the tab folder and the library, and renders.
pub(super) fn restore_pack_tab(
    s: &mut Session,
    resume: &Resume,
    state: State,
    cached: Option<&TabDocument>,
) -> Result<()> {
    let legacy = legacy_of(&state);
    let State { mode: SavedMode::Pack(saved), ui, .. } = state else {
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
    snafu::ensure!(pack_doc.sources.packs.contains_key(&saved.edit.base), PackTabSnafu { reason: "base missing" });
    let preview = resume.read_doc(DECK_DIR, &s.doc_defaults, max, &s.registry).map_err(|e| {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "aperçu de reprise illisible");
        PackTabSnafu { reason: "unreadable preview" }.build()
    })?;
    let bare = preview.path.is_none() && preview.saved_hash.is_none() && preview.sources.assets.is_empty();
    snafu::ensure!(bare, PackTabSnafu { reason: "preview document with a file or assets" });
    let pins = combined_pins(preview.sources.deck());
    let (widgets, notice) = s.resolve_widgets(&pins, &preview.sources.packs).context(SessionSnafu)?;
    s.install_document(preview, widgets, notice).context(SessionSnafu)?;
    s.finish_pack_restore(saved, pack_doc, legacy, ui, cached)
}

impl Session {
    /// A pack tab resumed on `pack_doc` for `saved`, its preview set aside.
    ///
    /// Checked against the library and `cached` (`check_pack_doc`), the
    /// lease taken if free (`restore_lease`), a tab that never saved freezing
    /// its base, a `legacy` working deck restyled, a deck-pack tab linked with
    /// its unforwarded save due again (or `notice-deck-pack-unforwarded-lost`),
    /// then rendered.
    ///
    /// # Errors
    ///
    /// `PackTab` when its pack does not match; `Session` (restyle, widgets,
    /// render, freezing).
    ///
    /// # Blocking
    ///
    /// Reads the library and renders.
    pub(super) fn finish_pack_restore(
        &mut self,
        saved: SavedPack,
        mut pack_doc: Document,
        legacy: Option<Legacy>,
        ui: UiState,
        cached: Option<&TabDocument>,
    ) -> Result<()> {
        let SavedPack { mut edit, unforwarded, .. } = saved;
        let check = self.check_pack_doc(&mut edit, &pack_doc, cached, unforwarded.as_ref());
        if let PackCheck::Mismatch(reason) = check {
            return PackTabSnafu { reason }.fail();
        }
        let (lease, notice) = self.restore_lease(&edit, &check);
        // The library's bytes of the base, never `pack/`'s; a read-only tab freezes it later.
        if let Some(lease) = lease.as_ref().filter(|_| edit.session_version.is_none()) {
            self.freeze_base(lease, &edit.base).context(SessionSnafu)?;
        }
        let dropped = match legacy {
            Some(legacy) => self.restyle_legacy(&mut pack_doc, legacy.draft.as_ref()).context(SessionSnafu)?,
            None => false,
        };
        let preview = std::mem::replace(&mut self.doc, pack_doc);
        let orphaned = matches!(check, PackCheck::Orphaned);
        let link = cached.and_then(TabDocument::deck_tab).map(|owner| Box::new(TabLink::new(owner)));
        let resent = unforwarded.filter(|_| link.is_some());
        self.mode =
            Mode::Pack { preview: Box::new(preview), edit, scratch: PackScratch::default(), lease, orphaned, link };
        if let Some(base) = resent {
            self.resend_unforwarded(base);
        }
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

    /// Makes a resumed deck-pack tab's unforwarded save due again, from its base's files.
    ///
    /// The files come from `pack/`, which `check_pack_doc` requires to hold
    /// the base; an undo back to the parent leaves no session version but
    /// keeps that base. A pack document that pins another version loses the
    /// save: `notice-deck-pack-unforwarded-lost`, logged.
    fn resend_unforwarded(&mut self, base: SendBase) {
        let edited = match &self.mode {
            Mode::Pack { edit, .. } => Some(edit.base.clone()),
            _ => None,
        };
        let pinned = self.edited_pin().map(|(_, pin)| pin.version.clone());
        let version = edited.filter(|v| pinned.as_ref() == Some(v));
        if let Some(files) = version.and_then(|v| self.doc.sources.packs.get(&v).cloned()) {
            self.resume_outbox(base, &files);
        } else {
            tracing::warn!(pack = %base.pack, "deck-pack tab resumed without its unforwarded save");
            self.set_notice(self.localizer.tr("notice-deck-pack-unforwarded-lost"));
        }
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
    /// cleared unless it is the base and descends from the saved parent. A
    /// pack tab of a deck-only pack must be a deck-pack tab (`cached`) naming
    /// that pack (the unforwarded save's base pack while there is one), and
    /// a deck-pack tab a deck-only pack.
    fn check_pack_doc(
        &self,
        edit: &mut PackMode,
        pack_doc: &Document,
        cached: Option<&TabDocument>,
        unforwarded: Option<&SendBase>,
    ) -> PackCheck {
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
        let deck_pack = match cached {
            Some(TabDocument::DeckPack { pack, .. }) => Some(*pack),
            _ => None,
        };
        if !is_library_pack(base) {
            if listed {
                return PackCheck::Mismatch("deck-scoped base of a library pack");
            }
            match deck_pack {
                None => return PackCheck::Mismatch("deck-only pack in a pack tab"),
                Some(_) if !edit.deck_only => return PackCheck::Mismatch("deck-pack tab state not deck-only"),
                Some(pack) if pack != unforwarded.map_or(edit.pack, |base| base.pack) => {
                    return PackCheck::Mismatch("deck-pack tab of another pack");
                }
                Some(_) => clear_foreign_session_version(edit, saved_manifest),
            }
            recompute(edit, &manifest, true);
            return PackCheck::DeckOnly;
        }
        if deck_pack.is_some() {
            return PackCheck::Mismatch("library pack in a deck-pack tab");
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
        clear_foreign_session_version(edit, saved_manifest);
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
                match PackLease::acquire(scope, edit.pack, &name, &name) {
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

/// Clears `edit`'s session version unless it is the base and descends from the saved parent.
///
/// `saved_manifest` is the session version's manifest, read before.
fn clear_foreign_session_version<E>(edit: &mut PackMode, saved_manifest: Option<std::result::Result<PackManifest, E>>) {
    let saved_parent = saved_manifest.and_then(Result::ok).map(|m| m.parent);
    let saved = edit.session_version.as_ref();
    if saved.is_some() && (saved != Some(&edit.base) || saved_parent.as_ref() != Some(&edit.parent)) {
        tracing::warn!(pack = %edit.pack, "version de séance reprise étrangère : la prochaine sera neuve");
        edit.session_version = None;
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
