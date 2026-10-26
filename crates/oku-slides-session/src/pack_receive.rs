//! Versions de pack reçues avec un fichier, et les packs du deck seul.
//!
//! À l'ouverture et à la fusion, chaque version embarquée absente de la
//! bibliothèque y est ajoutée en silence (un id inconnu crée le pack) ; sa
//! version courante ne change jamais. Un pack du deck seul
//! (`PackScope::Deck`) n'est jamais reçu dans la bibliothèque. Ce que le deck
//! épingle par rapport à la bibliothèque se lit dans le panneau des packs
//! (`deck_packs`). Les thèmes écartés de la bibliothèque sont signalés ici
//! une fois par processus.

use std::sync::atomic::{AtomicBool, Ordering};

use oku_slides_core::PackFiles;

use crate::Session;
use crate::pack::PackManifest;

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

    /// Ajoute en silence à la bibliothèque les versions embarquées qu'elle n'a pas.
    ///
    /// Une version déjà connue ou nouvelle d'un pack connu ne change jamais
    /// sa version courante ; un pack inconnu est créé, celle-ci courante.
    /// Une version du deck seul reste dans le deck.
    pub(crate) fn receive_packs(&self) {
        for files in self.doc.sources.packs.values() {
            let id = match PackManifest::from_files(files) {
                Ok(m) if !m.scope.is_library() => continue,
                Ok(m) => m.id,
                Err(e) => {
                    tracing::warn!(version = %files.version(), error = %e, "pack reçu illisible");
                    continue;
                }
            };
            if let Err(e) = self.library.receive_version(id, files) {
                tracing::warn!(pack = %id, error = %e.one_line(), "pack reçu non ajouté à la bibliothèque");
            }
        }
    }

    /// Vrai si le deck épingle un pack qui n'appartient qu'à lui (`PackScope::Deck`) : jamais dans la bibliothèque.
    ///
    /// Lu à chaque instantané : la seule clé `scope`, jamais tout le manifeste.
    pub(crate) fn deck_pack_is_deck_only(&self) -> bool {
        let deck = self.doc.sources.deck();
        deck.all_pins().filter_map(|(_, _, pin)| self.doc.sources.packs.get(&pin.version)).any(is_deck_only)
    }

    /// Vrai si le deck embarque le pack `id` en version du deck seul.
    pub(crate) fn embeds_deck_only_pack(&self, id: uuid::Uuid) -> bool {
        let deck_only = self.user_blobs().packs.values().filter(|f| is_deck_only(f));
        deck_only.filter_map(|f| PackManifest::from_files(f).ok()).any(|m| m.id == id)
    }
}

#[cfg(test)]
#[path = "pack_receive_tests.rs"]
mod pack_receive_tests;
