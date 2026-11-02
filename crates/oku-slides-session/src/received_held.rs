//! The versions a document holds, and whether the user's library has them.
//!
//! One walk over the pins (`held_versions`, packs then themes) and one rule
//! for "received, not installed": [`Session::library_has`]. The review lists
//! what the library lacks, the pack tab and the deck-mode widget write refuse
//! it (`PackNotInstalled`), and the install checks a theme's parents with it.

use uuid::Uuid;

use oku_slides_core::{CapabilityKind, Namespace, PackFiles, PackKind, PackPin, PackVersion, SourceBundle};

use crate::guide_origin::is_shipped;
use crate::library_kind::VersionManifest;
use crate::pack_receive::is_library_pack;
use crate::theme::ThemeManifest;
use crate::{PackLibrary, Result, Session, ThemeLibrary};

/// A version the open document pins, with its embedded files.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeldVersion<'a> {
    /// Id of the pack or theme.
    pub(crate) id: Uuid,
    /// The pinned version.
    pub(crate) version: &'a PackVersion,
    /// The namespace the deck pins it under.
    pub(crate) namespace: &'a Namespace,
    /// Widgets, layouts or theme.
    pub(crate) kind: CapabilityKind,
    /// The namespaces the pin requires (a theme's parents).
    pub(crate) requires: &'a [Namespace],
    /// The version's files, embedded in the document's sources.
    pub(crate) files: &'a PackFiles,
}

impl<'a> HeldVersion<'a> {
    /// The pack pin `pin` of kind `kind` under `namespace`, with its embedded `files`.
    pub(crate) fn of_pack(kind: PackKind, namespace: &'a Namespace, pin: &'a PackPin, files: &'a PackFiles) -> Self {
        Self { id: pin.id, version: &pin.version, namespace, kind: kind.into(), requires: &pin.requires, files }
    }

    /// True for a theme.
    pub(crate) fn is_theme(&self) -> bool {
        self.kind == CapabilityKind::Theme
    }
}

impl Session {
    /// Every version the deck pins, packs then themes, in pin order, the deck's own included.
    ///
    /// A pin whose files the sources do not hold is left out: the sources report it.
    pub(crate) fn held_versions(&self) -> Vec<HeldVersion<'_>> {
        Self::held_in(&self.doc.sources)
    }

    /// The same walk over any sources, such as a pack tab's preview deck.
    pub(crate) fn held_in(sources: &SourceBundle) -> Vec<HeldVersion<'_>> {
        let deck = sources.deck();
        let packs = deck.all_pins().filter_map(|(kind, namespace, pin)| {
            let files = sources.packs.get(&pin.version)?;
            Some(HeldVersion::of_pack(kind, namespace, pin, files))
        });
        let themes = deck.themes.iter().filter_map(|(namespace, pin)| {
            let files = sources.themes.get(&pin.version)?;
            Some(HeldVersion {
                id: pin.id,
                version: &pin.version,
                namespace,
                kind: CapabilityKind::Theme,
                requires: &pin.extends,
                files,
            })
        });
        packs.chain(themes).collect()
    }

    /// True when the library has `held`, or needs not: deck-only, shipped, or listed.
    ///
    /// The one rule for a received version not installed yet: false for it.
    pub(crate) fn library_has(&self, held: &HeldVersion<'_>) -> bool {
        !may_install(held) || is_shipped(held.id, held.version, held.is_theme()) || self.lists(held)
    }

    /// True if the library of `held`'s kind lists its version.
    pub(crate) fn lists(&self, held: &HeldVersion<'_>) -> bool {
        if held.is_theme() {
            self.themes.lists(held.id, held.version)
        } else {
            self.library.lists(held.id, held.version)
        }
    }

    /// True if the library of `held`'s kind has any version of its pack or theme.
    pub(crate) fn knows(&self, held: &HeldVersion<'_>) -> bool {
        if held.is_theme() { self.themes.knows(held.id) } else { self.library.knows(held.id) }
    }

    /// Checks that `held` could enter its library, writing nothing.
    ///
    /// # Errors
    ///
    /// The refusal of the library's write: `DeckOnly`, `NamespaceTaken`, `NamespaceReserved`,
    /// `NamespaceChanged`, `PackKindChanged`, an unreadable manifest or no library folder.
    pub(crate) fn check_install(&self, held: &HeldVersion<'_>) -> Result<()> {
        if held.is_theme() {
            let manifest = <ThemeManifest as VersionManifest>::from_files(held.files)?;
            self.library.check_theme(held.id, &manifest.namespace)?;
            self.themes.check_receive(held.id, held.files)
        } else {
            self.library.check_receive(held.id, held.files)
        }
    }

    /// The first parent of theme `held` that cannot enter the library; `None` when all can.
    ///
    /// A parent the deck does not hold, a built-in or one the library has passes; one
    /// unreadable, refused by the library, or with such a parent itself, blocks. `path`
    /// holds the themes being walked: a cycle blocks.
    pub(crate) fn blocked_parent<'h>(
        &self,
        held: &HeldVersion<'h>,
        all: &[HeldVersion<'h>],
        path: &mut Vec<Uuid>,
    ) -> Option<&'h Namespace> {
        if !held.is_theme() {
            return None;
        }
        if path.contains(&held.id) {
            return Some(held.namespace);
        }
        path.push(held.id);
        let blocked = held.requires.iter().find(|ns| {
            let Some(parent) = all.iter().find(|h| h.is_theme() && h.namespace == *ns) else { return false };
            if is_builtin(parent) || self.library_has(parent) {
                return false;
            }
            let readable = crate::received_review::read_capabilities(parent).is_some();
            !readable || self.check_install(parent).is_err() || self.blocked_parent(parent, all, path).is_some()
        });
        path.pop();
        blocked
    }
}

/// False for a deck-only pack: it never enters the library, so it is not offered.
pub(crate) fn may_install(held: &HeldVersion<'_>) -> bool {
    held.is_theme() || is_library_pack(held.files)
}

/// True for a pack or theme the app ships, or a version it ships: never entering the library.
pub(crate) fn is_builtin(held: &HeldVersion<'_>) -> bool {
    let theme = held.is_theme();
    let builtin = if theme { ThemeLibrary::is_builtin(held.id) } else { PackLibrary::is_builtin(held.id) };
    builtin || is_shipped(held.id, held.version, theme)
}
