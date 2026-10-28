//! An owner deck receives its deck-pack tabs' saves: validate, then apply.
//!
//! `receive_deck_pack` checks a send against what the owner knows of the
//! link (`validate_send`), never against what the tab says of itself: the
//! pin it holds, its scope and namespace, the parent the link expects, the
//! sources budget. A copy (decision 9) replaces the original's pins. Only
//! then the owner pins the version, decided on a clone first (`apply_send`),
//! in one undo step per editing session (`deck_pack_merge_key`). The answer
//! maps onto the tab's outbox in one place, `settle_answer`.

use snafu::ResultExt;

use oku_slides_core::{Namespace, PackFiles, PackPin, PackVersion, SourceBundle, SourcesError};
use oku_slides_i18n::Localize;

use crate::agent::bounds::label;
use crate::deck_closure::{closure_with_files, combined_pins};
use crate::deck_pack_link::{DeckLink, LinkParent};
use crate::deck_pack_tab::DeckPackSend;
use crate::error::{DeckPackMismatchSnafu, SourcesSnafu, WrongDocumentSnafu};
use crate::library_packs::PackLibrary;
use crate::pack::PackManifest;
use crate::pack_mode::{embed_packs, pin_deck};
use crate::pack_receive::deck_scope;
use crate::{Mode, Result, Session, SessionError};

/// What the owner did with a send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Received {
    /// Applied as (part of) one undo step.
    Applied,
    /// Already applied: acknowledge it.
    Delivered,
    /// Older than the last applied send: ignored.
    Stale,
    /// In an agent turn or history: retried at the owner's pull.
    Busy,
}

/// The merge key of one deck-pack tab's editing session.
pub(crate) fn deck_pack_merge_key(tab: &str) -> String {
    format!("pack-adopt:{tab}")
}

/// Settles the tab's send `seq` with the owner's `answer` (decision 7).
pub(crate) fn settle_answer(tab: &mut Session, seq: u64, answer: &Result<Received>) {
    match answer {
        Ok(Received::Applied | Received::Delivered) => tab.acknowledge_deck_pack(seq, Ok(())),
        Ok(Received::Stale) => tab.release_stale_deck_pack(seq),
        Ok(Received::Busy) => tab.park_deck_pack(seq),
        Err(e) => tab.acknowledge_deck_pack(seq, Err(e)),
    }
}

/// A send `validate_send` accepted, and how to apply it.
#[derive(Debug)]
struct Accepted {
    /// The link it is applied for.
    link: DeckLink,
    /// The deck key that pins the link's version.
    namespace: Namespace,
    /// The sent version's manifest.
    manifest: PackManifest,
}

/// What `validate_send` decided.
#[derive(Debug)]
enum Checked {
    /// An answer without any change.
    Answer(Received),
    /// To apply.
    Apply(Box<Accepted>),
}

/// Bytes a pack version adds to the sources, as the source zip holds them.
fn files_bytes(files: &PackFiles) -> usize {
    let manifest = files.pack_json().map_or(usize::MAX, |json| json.len());
    files.widgets().values().map(String::len).fold(manifest, usize::saturating_add)
}

/// The sources' bytes, uncompressed, as their budget counts them.
///
/// # Errors
///
/// `Sources` when the replica does not export: a real fault, never a refusal.
fn source_bytes(sources: &SourceBundle) -> Result<usize> {
    let replica = sources.replica().snapshot().map_err(SourcesError::from).context(SourcesSnafu)?.len();
    let blobs = sources.assets.values().chain(sources.fonts.values()).map(Vec::len);
    let packs = sources.packs.values().chain(sources.themes.values()).map(files_bytes);
    Ok(blobs.chain(packs).fold(replica, usize::saturating_add))
}

/// The refusal of a send of `pack`, logged with `why`.
fn mismatch(pack: &str, why: &str) -> SessionError {
    tracing::warn!(pack, why, "deck-pack save refused by its owner deck");
    DeckPackMismatchSnafu { pack }.build().into()
}

impl Session {
    /// Validates then applies a linked tab's send (`deck_pack_receive.rs`).
    ///
    /// # Errors
    ///
    /// `WrongDocument` outside a deck; `DeckPackMismatch` for a send this deck
    /// does not take (nothing changes then); `PackOpenInTab` when it would move
    /// another link's pack; `Sources` and the widget and render errors of the apply.
    ///
    /// # Blocking
    ///
    /// Resolves the new widgets and renders the deck.
    pub(crate) fn receive_deck_pack(&mut self, tab: &str, send: &DeckPackSend) -> Result<Received> {
        match self.validate_send(tab, send)? {
            Checked::Answer(received) => Ok(received),
            Checked::Apply(accepted) => self.apply_send(tab, send, *accepted),
        }
    }

    /// Whether this deck takes `send` from `tab`, from what it knows alone.
    fn validate_send(&self, tab: &str, send: &DeckPackSend) -> Result<Checked> {
        match self.mode {
            Mode::Deck => {}
            Mode::History { .. } => return Ok(Checked::Answer(Received::Busy)),
            _ => return WrongDocumentSnafu { kind: self.document_kind() }.fail()?,
        }
        let manifest = PackManifest::from_files(&send.files).ok();
        let sent_name = manifest.as_ref().map_or_else(|| send.base.pack.to_string(), |m| label(&m.name));
        let Some(link) = self.deck_links.get(tab) else { return Err(mismatch(&sent_name, "unlinked tab")) };
        let name = self.linked_name(link);
        let refuse = |why: &str| Err(mismatch(&name, why));
        if self.user_deck_uid() != link.deck_uid {
            return refuse("the owner opened another document");
        }
        match link.last_seq {
            Some(last) if send.seq < last => return Ok(Checked::Answer(Received::Stale)),
            Some(last) if send.seq == last => return Ok(Checked::Answer(Received::Delivered)),
            _ => {}
        }
        let version = send.files.version();
        if version == link.expect {
            return Ok(Checked::Answer(Received::Delivered));
        }
        if self.doc.sources.replica().in_group() {
            return Ok(Checked::Answer(Received::Busy));
        }
        let deck = self.doc.sources.deck();
        let pinned = deck.all_pins().find(|(_, _, pin)| pin.id == link.pack && pin.version == link.expect);
        let Some((kind, namespace, _)) = pinned else { return refuse("the pin moved from what the link expects") };
        let files = self.doc.sources.packs.get(&link.expect);
        if files.and_then(deck_scope) != Some(true) || self.library.current(link.pack).is_some() {
            return refuse("the pinned version is not deck-only");
        }
        if send.base.pack != link.pack || !link.parent.accepts(send.base.parent.as_ref()) {
            return refuse("the send's base is not the link's");
        }
        let Some(manifest) = manifest else { return refuse("unreadable manifest") };
        let fits = !manifest.scope.is_library() && send.files.kind() == kind && manifest.namespace == *namespace;
        if !fits {
            return refuse("scope, kind or namespace differs");
        }
        let same = manifest.id == link.pack
            && (manifest.parent == send.base.parent || send.base.parent.as_ref() == Some(&version));
        let copy = manifest.id != link.pack && manifest.parent.is_none() && self.fresh_pack_id(&manifest);
        if !same && !copy {
            return refuse("neither the same pack nor a fresh copy");
        }
        let total = source_bytes(&self.doc.sources)?.saturating_add(files_bytes(&send.files));
        let max = self.source_limits.total_bytes;
        if total > max {
            tracing::warn!(?total, max, "deck-pack save over the sources budget");
            return refuse("over the sources budget");
        }
        let namespace = namespace.clone();
        Ok(Checked::Apply(Box::new(Accepted { link: link.clone(), namespace, manifest })))
    }

    /// True when no pin, embedded version, library pack or built-in has `manifest`'s id.
    fn fresh_pack_id(&self, manifest: &PackManifest) -> bool {
        let id = manifest.id;
        let pinned = self.doc.sources.deck().all_pins().any(|(_, _, pin)| pin.id == id);
        let embedded = self.doc.sources.packs.values().any(|f| PackManifest::from_files(f).is_ok_and(|m| m.id == id));
        !pinned && !embedded && !PackLibrary::is_builtin(id) && self.library.current(id).is_none()
    }

    /// Pins the accepted send, decided on a clone of the deck before anything is written.
    fn apply_send(&mut self, tab: &str, send: &DeckPackSend, accepted: Accepted) -> Result<Received> {
        let Accepted { link, namespace, manifest } = accepted;
        let name = self.linked_name(&link);
        let copy = manifest.id != link.pack;
        let version = send.files.version();
        let mut deck = self.doc.sources.deck().clone();
        if copy {
            deck.widgets.retain(|_, pin| pin.id != link.pack);
            deck.layouts.retain(|_, pin| pin.id != link.pack);
        }
        let pin = PackPin { id: manifest.id, version: version.clone(), requires: manifest.requires.clone() };
        let mut held = self.doc.sources.packs.clone();
        held.insert(version.clone(), send.files.clone());
        let edited = (namespace.clone(), pin, send.files.clone());
        let packs = closure_with_files(&self.library, &deck, edited, &held)
            .and_then(|packs| pin_deck(&mut deck, &packs).map(|()| packs))
            .map_err(|e| mismatch(&name, &e.one_line()))?;
        let (before, after) = (combined_pins(self.doc.sources.deck()), combined_pins(&deck));
        let moved = before.iter().any(|(ns, pin)| *ns != namespace && after.get(ns) != Some(pin));
        if moved {
            return Err(mismatch(&name, "it would move another pin"));
        }
        self.refuse_linked_pins_except(&after, Some(tab))?;
        // Accepted: only now does the owner's widget session end (a refusal keeps its merge).
        self.end_widget_session();
        let added: Vec<PackVersion> = packs
            .iter()
            .map(|(_, pin, _)| pin.version.clone())
            .filter(|v| !self.doc.sources.packs.contains_key(v))
            .collect();
        embed_packs(&mut self.doc.sources, packs);
        let label = self.localizer.tr_with("pack-adopt-label", &[("name", manifest.name.as_str().into())]);
        let steps = self.doc.sources.undo_steps();
        if let Err(e) = self.doc.sources.apply(deck, &label, Some(&deck_pack_merge_key(tab))) {
            for v in &added {
                self.doc.sources.packs.remove(v);
            }
            return Err(e).context(SourcesSnafu)?;
        }
        let merged = self.doc.sources.undo_steps() == steps;
        let old = &link.expect;
        let pinned_elsewhere = self.doc.sources.deck().all_pins().any(|(_, _, pin)| pin.version == *old);
        if merged && *old != version && send.base.parent.as_ref() != Some(old) && !pinned_elsewhere {
            // Merged into the step that introduced it: no undo reaches it any more.
            self.doc.sources.packs.remove(old);
        }
        if let Some(stored) = self.deck_links.get_mut(tab) {
            stored.pack = manifest.id;
            stored.parent = LinkParent::Known(if copy { None } else { send.base.parent.clone() });
            stored.expect = version;
            stored.last_seq = Some(send.seq);
        }
        self.doc.dirty = true;
        let shown = self.show_received();
        self.persist(true);
        shown.map(|()| Received::Applied)
    }

    /// Installs the new widgets and renders; a failed render keeps the change, with its notice.
    fn show_received(&mut self) -> Result<()> {
        self.sync_widgets()?;
        self.refresh_pack_updates();
        match self.rerender() {
            Err(e) if e.render_error().is_some() => {
                tracing::warn!(error = %e.one_line(), "deck-pack save applied, render failed");
                self.set_notice(e.localize(&self.localizer));
                Ok(())
            }
            other => other,
        }
    }
}

#[cfg(test)]
#[path = "deck_pack_validate_tests.rs"]
mod deck_pack_validate_tests;

#[cfg(test)]
#[path = "deck_pack_receive_tests.rs"]
mod deck_pack_receive_tests;
