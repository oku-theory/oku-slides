//! Agent instructions, in English only: the agent reads them, not the user.
//!
//! `INSTRUCTIONS` (the shared core), the tab kind's paragraph (`for_kind`) and
//! `WIDGETS` (not in a theme tab) go with a session's first turn; `IMPORT` goes
//! with the one fine-tuning turn of an import (`AgentCommand::Tune`).
//!
//! What the user reads comes in the interface language and deck content in the
//! deck language, both given by each turn's context (`interface language` and
//! `deck language` of `<oku-slides-context>`). The files have no final newline:
//! their text goes out as is. A text names a tool in backticks only when the
//! tab kind it goes to lists that tool (`instructions_tests`).

use oku_slides_session::DocumentKind;

/// Role of the agent, the tools of every tab kind, the language rule.
pub(crate) const INSTRUCTIONS: &str = include_str!("instructions/agent.md");

/// What a deck or help tab does: files, history, presenting, themes and packs.
const DECK: &str = include_str!("instructions/deck.md");

/// What a pack tab does: edit the pack's items.
const PACK: &str = include_str!("instructions/pack.md");

/// What a theme tab does: edit the theme's tokens, parents and name.
const THEME: &str = include_str!("instructions/theme.md");

/// Write a Typst widget and put it in the deck.
pub(crate) const WIDGETS: &str = include_str!("instructions/widgets.md");

/// Fine-tune an imported deck: the bounded loop, sent with the fine-tuning turn only.
pub(crate) const IMPORT: &str = include_str!("instructions/import.md");

/// The paragraph for a tab of `kind`; a help tab reads the deck's.
pub(crate) const fn for_kind(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Deck | DocumentKind::Help => DECK,
        DocumentKind::Pack => PACK,
        DocumentKind::Theme => THEME,
    }
}

#[cfg(test)]
#[path = "instructions_tests.rs"]
mod instructions_tests;
