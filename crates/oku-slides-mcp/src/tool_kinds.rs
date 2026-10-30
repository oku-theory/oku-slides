//! Which tab kinds list a tool: a small bitset over `DocumentKind`.

use oku_slides_session::DocumentKind;

/// The tab kinds that list a tool, one bit per [`DocumentKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Kinds(u8);

impl Kinds {
    /// Deck tabs only.
    const DECK: Self = Self(1);
    /// Help tabs only.
    const HELP: Self = Self(2);
    /// Deck and help tabs (a help tab is a read-only deck).
    pub(crate) const DECK_OR_HELP: Self = Self::DECK.or(Self::HELP);
    /// Pack tabs.
    pub(crate) const PACK: Self = Self(4);
    /// Theme tabs.
    pub(crate) const THEME: Self = Self(8);
    /// Every tab kind.
    pub(crate) const ALL: Self = Self::DECK_OR_HELP.or(Self::PACK).or(Self::THEME);

    /// The kinds of `self` and of `other`.
    pub(crate) const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// True if a tab of `kind` lists the tool.
    pub(crate) fn has(self, kind: DocumentKind) -> bool {
        let bit = match kind {
            DocumentKind::Deck => Self::DECK,
            DocumentKind::Help => Self::HELP,
            DocumentKind::Pack => Self::PACK,
            DocumentKind::Theme => Self::THEME,
        };
        self.0 & bit.0 != 0
    }
}

#[cfg(test)]
#[path = "tool_kinds_tests.rs"]
mod tool_kinds_tests;
