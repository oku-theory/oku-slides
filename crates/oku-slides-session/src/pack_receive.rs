//! Pack and theme versions received with a file, and the packs of one deck only.
//!
//! Opening or merging a file adds nothing to the library: the versions it
//! brings stay in the deck's sources, listed for review (`received_review`).
//! Only the user's validation installs them (`install_received`, from
//! `Request::AcceptReceived` or the packs panel), marked received; an
//! unknown id creates the pack, and a known one never changes its current
//! version. A deck-only pack (`PackScope::Deck`) never enters the library.
//! Library themes set aside are reported here once per process.

use std::sync::atomic::{AtomicBool, Ordering};

use oku_slides_core::PackFiles;
use snafu::OptionExt;
use uuid::Uuid;

use crate::agent::bounds::label;
use crate::error::{NotHeldSnafu, ReceivedParentMissingSnafu};
use crate::guide_approvals::VersionRef;
use crate::pack::PackManifest;
use crate::received_held::{HeldVersion, is_builtin};
use crate::received_review::{ReceivedReview, name_of};
use crate::{Result, Session};

/// Whether the version `files` belongs to one deck only; `None` for an unreadable scope.
///
/// Read silently: the one source of the rule, for callers that log or fail closed.
pub(crate) fn deck_scope(files: &PackFiles) -> Option<bool> {
    files.scope().ok().map(|scope| !scope.is_library())
}

/// Vrai si la version `files` n'appartient qu'à un deck.
///
/// Une portée illisible ne l'est pas : journalisée, le pack est traité comme de la bibliothèque.
pub(crate) fn is_deck_only(files: &PackFiles) -> bool {
    deck_scope(files).unwrap_or_else(|| {
        tracing::warn!("portée du pack illisible : traité comme de la bibliothèque");
        false
    })
}

/// True when `files` belong to a library pack.
#[must_use]
pub(crate) fn is_library_pack(files: &PackFiles) -> bool {
    !is_deck_only(files)
}

/// Les thèmes écartés de la bibliothèque ont déjà été signalés dans le processus.
static THEMES_SKIPPED_SEEN: AtomicBool = AtomicBool::new(false);

impl Session {
    /// Signale une fois par processus les thèmes de la bibliothèque écartés (`notice-themes-skipped`).
    ///
    /// Un thème d'avant les espaces de noms ne se lit plus : il n'est ni listé ni proposé.
    /// L'avis est gardé (`themes_notice`) pour suivre le document qui remplacerait le deck
    /// vierge avant qu'il soit vu ([`Self::carried_themes_notice`]).
    pub(crate) fn notice_skipped_themes(&mut self) {
        let count = self.themes.skipped();
        if count > 0 && !THEMES_SKIPPED_SEEN.swap(true, Ordering::Relaxed) {
            tracing::warn!(count, "thèmes de la bibliothèque écartés : sans espace de noms ou illisibles");
            let text = self.localizer.tr_with("notice-themes-skipped", &[("count", count.into())]);
            self.themes_notice = Some(text.clone());
            self.set_notice(text);
        }
    }

    /// L'avis des thèmes écartés que le document montré porte encore, pas encore vu : le suivant le reprend.
    ///
    /// Un avis déjà montré (le snapshot le vide) ne revient pas.
    pub(crate) fn carried_themes_notice(&mut self) -> Option<String> {
        let text = self.themes_notice.take()?;
        self.doc.notice.as_deref().is_some_and(|pending| pending.contains(text.as_str())).then_some(text)
    }

    /// Adds the received versions `versions` of the open deck to the library, on the user's request.
    ///
    /// Each must be a version the deck holds (`NotHeld` otherwise); one listed
    /// for install is added, marked received, a theme after the parents it
    /// extends that are listed too; one the library cannot take (its `blocker`)
    /// or already holds is skipped. The deck does not change; the pack updates
    /// and the theme list are read again, whatever the outcome.
    ///
    /// # Errors
    ///
    /// Outside deck mode, for a version the deck does not hold (`NotHeld`), a theme whose
    /// parent is still missing (`ReceivedParentMissing`), or the first failed library write
    /// (the versions written before it stay).
    pub(crate) fn install_received(&mut self, versions: &[VersionRef]) -> Result<()> {
        if versions.is_empty() {
            return Ok(());
        }
        self.ensure_deck_mode()?;
        let installed = self.install_each(versions);
        self.refresh_pack_updates();
        self.theme_list = self.themes.themes();
        installed
    }

    /// Installs each of `versions`, in order, stopping at the first failure.
    fn install_each(&self, versions: &[VersionRef]) -> Result<()> {
        // Read afresh, not from the snapshot's cache: the library decides now.
        let installable: Vec<VersionRef> =
            self.fresh_reviews().iter().filter(|r| r.install).map(ReceivedReview::version_ref).collect();
        let held = self.held_versions();
        let mut done = Vec::new();
        for wanted in versions {
            let found = held.iter().find(|h| h.id == wanted.id && h.version == &wanted.version);
            let held_one = found.with_context(|| NotHeldSnafu { pack: self.known_name(wanted.id) })?;
            self.install_with_parents(held_one, &held, &installable, &mut done)?;
        }
        Ok(())
    }

    /// The name of pack or theme `id`: from a version the deck holds, else the library, else the id.
    fn known_name(&self, id: Uuid) -> String {
        let held = self.held_versions();
        let embedded = held.iter().find(|h| h.id == id).map(|h| name_of(h.files));
        let library = || {
            let pack = self.library.current_version(id).map(|s| label(&s.manifest.name));
            pack.or_else(|| self.themes.current_version(id).map(|s| label(&s.manifest.name)))
        };
        embedded.or_else(library).unwrap_or_else(|| id.to_string())
    }

    /// Installs `one` if listed for install and not `done` yet, a theme's listed parents first.
    ///
    /// A theme whose held parent the library still lacks then is refused (`ReceivedParentMissing`).
    fn install_with_parents(
        &self,
        one: &HeldVersion<'_>,
        held: &[HeldVersion<'_>],
        installable: &[VersionRef],
        done: &mut Vec<VersionRef>,
    ) -> Result<()> {
        let this = VersionRef { id: one.id, version: one.version.clone() };
        if done.contains(&this) || !installable.contains(&this) {
            return Ok(());
        }
        done.push(this);
        if one.is_theme() {
            let parents: Vec<&HeldVersion<'_>> =
                one.requires.iter().filter_map(|ns| held.iter().find(|h| h.is_theme() && h.namespace == ns)).collect();
            for parent in &parents {
                self.install_with_parents(parent, held, installable, done)?;
            }
            if let Some(missing) = parents.iter().find(|p| !is_builtin(p) && !self.library_has(p)) {
                let (theme, parent) = (label(one.namespace.as_str()), label(missing.namespace.as_str()));
                return ReceivedParentMissingSnafu { theme, parent }.fail()?;
            }
            self.check_install(one)?;
            self.themes.receive_version(one.id, one.files)?;
        } else {
            self.library.receive_version(one.id, one.files)?;
        }
        tracing::info!(id = %one.id, version = %one.version, kind = one.kind.as_str(), "received version installed");
        Ok(())
    }

    /// Validates what the open deck brings: installs `versions`, then approves their awaiting guides.
    ///
    /// The user read them in the dialog. A failed install approves nothing. Every
    /// answered version the deck holds, blocked ones too, then leaves this tab's list.
    ///
    /// # Errors
    ///
    /// As [`Self::install_received`], or a failed write of the approvals.
    pub(crate) fn accept_received(&mut self, versions: &[VersionRef]) -> Result<()> {
        let awaiting: Vec<VersionRef> = self
            .fresh_reviews()
            .iter()
            .filter(|r| !r.guides.is_empty() && versions.contains(&r.version_ref()))
            .map(ReceivedReview::version_ref)
            .collect();
        self.install_received(versions)?;
        self.approve_guides(&awaiting)?;
        self.put_off_received(versions);
        Ok(())
    }

    /// Takes `versions` off this tab's review list, keeping only versions the deck still holds.
    pub(crate) fn put_off_received(&mut self, versions: &[VersionRef]) {
        let held: Vec<VersionRef> =
            self.held_versions().iter().map(|h| VersionRef { id: h.id, version: h.version.clone() }).collect();
        self.received_later.extend(versions.iter().filter(|v| held.contains(v)).cloned());
        self.received_later.retain(|v| held.contains(v));
    }

    /// Vrai si le deck épingle un pack qui n'appartient qu'à lui (`PackScope::Deck`) : jamais dans la bibliothèque.
    ///
    /// Lu à chaque instantané : la seule clé `scope`, jamais tout le manifeste.
    pub(crate) fn deck_pack_is_deck_only(&self) -> bool {
        let deck = self.doc.sources.deck();
        deck.all_pins().filter_map(|(_, _, pin)| self.doc.sources.packs.get(&pin.version)).any(is_deck_only)
    }

    /// Vrai si le deck embarque le pack `id` en version du deck seul.
    pub(crate) fn embeds_deck_only_pack(&self, id: Uuid) -> bool {
        let deck_only = self.user_blobs().packs.values().filter(|f| is_deck_only(f));
        deck_only.filter_map(|f| PackManifest::from_files(f).ok()).any(|m| m.id == id)
    }
}

#[cfg(test)]
#[path = "pack_receive_tests.rs"]
mod pack_receive_tests;
