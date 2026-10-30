//! Bounds on texts and colors from the agent, shared by turns, `report` and `render`.
//!
//! [`label`] serves the whole crate: it cleans deck, manifest and request strings.

use oku_slides_a2ui::ChoiceOption;
use snafu::ensure;

use super::AgentFormReason;
use crate::Result;
use crate::error::AgentFormSnafu;

/// Longueur d'une note ou d'un titre (caractères) : notes de la timeline, titres et faits d'un reçu.
pub const NOTE_CHARS: usize = 120;
/// Titre, fait, badge, résumé de détail : comme une note.
pub const TEXT_MAX: usize = NOTE_CHARS;
/// Explication (`why`), texte libre d'une carte, narration gardée d'un tour.
pub const WHY_MAX: usize = 4000;
/// Libellé d'un bouton ou d'une suite.
pub const LABEL_MAX: usize = 60;
/// Consigne envoyée par une suite.
pub const PROMPT_MAX: usize = 2000;
/// Arguments ou résultat d'un appel d'outil, montrés au dépliage du tour.
pub const DETAIL_CHARS: usize = 1500;

/// Contrôles d'ordre bidirectionnel, caractères invisibles et caractères « tag » (U+E0000..U+E007F,
/// un texte invisible que lit un LLM) : un libellé ne doit pas se lire autrement qu'il ne s'envoie.
/// U+200C (ZWNJ) et U+200D (ZWJ) restent permis : ils servent aux écritures indiennes et aux émojis composés.
fn is_hidden_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
            | '\u{200B}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' | '\u{180E}' | '\u{00AD}'
            | '\u{E0000}'..='\u{E007F}'
    )
}

/// Texte de l'agent affichable : sans caractères invisibles, coupé à `max`.
///
/// La coupure compte le « … » final. Pour un texte montré tel quel, sans être refusé
/// (détail d'un outil, texte entier d'une note).
#[must_use]
pub fn clip(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let mut kept = text.chars().filter(|c| !is_hidden_control(*c));
    let mut out: String = kept.by_ref().take(max).collect();
    if kept.next().is_some() {
        out.pop();
        out.push('…');
    }
    out
}

/// Invisible or blank-rendering code points that `label` removes besides the hidden controls.
///
/// Default-ignorable code points, the joiners U+200C and U+200D that `clip` keeps for
/// scripts and emoji, and the code points that draw blanks (Braille blank, Hangul fillers).
fn is_label_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{034F}' | '\u{115F}' | '\u{1160}' | '\u{17B4}'..='\u{17B5}' | '\u{180B}'..='\u{180F}'
            | '\u{200C}' | '\u{200D}' | '\u{2065}' | '\u{206A}'..='\u{206F}' | '\u{2800}'
            | '\u{3164}' | '\u{FE00}'..='\u{FE0F}' | '\u{FFA0}' | '\u{FFF9}'..='\u{FFFB}'
            | '\u{1D159}'..='\u{1D15A}' | '\u{1D173}'..='\u{1D17A}' | '\u{E0100}'..='\u{E01EF}'
    )
}

/// Quotation marks: the Unicode `Pi` and `Pf` categories, the ASCII, full-width and CJK quotes, the backtick.
fn is_quote(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '`' | '\u{AB}' | '\u{BB}' | '\u{2018}'..='\u{201F}' | '\u{2039}' | '\u{203A}'
            | '\u{2E02}'..='\u{2E05}' | '\u{2E09}' | '\u{2E0A}' | '\u{2E0C}' | '\u{2E0D}'
            | '\u{2E1C}' | '\u{2E1D}' | '\u{2E20}' | '\u{2E21}' | '\u{FF02}' | '\u{FF07}'
            | '\u{300C}'..='\u{300F}' | '\u{301D}'..='\u{301F}'
    )
}

/// Text made safe for a title, a dialog or a log line.
///
/// Controls (and U+2028, U+2029) become spaces; hidden controls and invisible or
/// blank-rendering code points are removed; whitespace runs collapse to one space and the
/// ends are trimmed; every quotation mark becomes U+02BC, a letter that no locale quotes
/// with, so a translation cannot be closed early by the text it quotes; the result is cut to
/// [`LABEL_MAX`] characters ([`clip`]). An empty result is `"…"`.
#[must_use]
pub fn label(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .filter_map(|c| {
            if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
                Some(' ')
            } else if is_hidden_control(c) || is_label_invisible(c) {
                None
            } else if is_quote(c) {
                Some('\u{02BC}')
            } else {
                Some(c)
            }
        })
        .collect();
    let words = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let clipped = clip(&words, LABEL_MAX);
    if clipped.is_empty() { "…".to_owned() } else { clipped }
}

/// Rejects agent text that is too long (`max` characters) or holds an invisible control.
///
/// # Errors
///
/// Fails (`AgentForm`) if `text` exceeds `max` characters or contains an invisible or
/// bidirectional control.
pub fn check_text(what: &str, text: &str, max: usize) -> Result<()> {
    let n = text.chars().count();
    ensure!(n <= max, AgentFormSnafu { reason: AgentFormReason::TooLong { at: what.to_owned(), count: n, max } });
    ensure!(
        !text.chars().any(is_hidden_control),
        AgentFormSnafu { reason: AgentFormReason::HiddenCharacters { at: what.to_owned() } }
    );
    Ok(())
}

/// Rejects options with duplicate ids, or an id, label or index out of bounds.
///
/// The chosen value must designate exactly one option, with no silent deduplication.
///
/// # Errors
///
/// Fails (`AgentForm`) on a duplicate id, or an id, label or index out of bounds.
pub fn check_options(what: &str, options: &[ChoiceOption]) -> Result<()> {
    let mut seen = std::collections::HashSet::with_capacity(options.len());
    for (i, option) in options.iter().enumerate() {
        check_text(&format!("{what}[{i}].id"), &option.id, LABEL_MAX)?;
        check_text(&format!("{what}[{i}].label"), &option.label, LABEL_MAX)?;
        if let Some(hint) = &option.hint {
            check_text(&format!("{what}[{i}].hint"), hint, TEXT_MAX)?;
        }
        ensure!(
            seen.insert(option.id.as_str()),
            AgentFormSnafu {
                reason: AgentFormReason::DuplicateId { at: format!("{what}[{i}]"), id: option.id.clone() }
            }
        );
    }
    Ok(())
}

/// Rejects a color that is not `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`.
///
/// The value quoted in the rejection is cut to [`LABEL_MAX`] characters.
///
/// # Errors
///
/// Fails (`AgentForm`, `NotHexColor`) if `color` is not an accepted hexadecimal color.
pub fn check_hex(what: &str, color: &str) -> Result<()> {
    let hex = color.strip_prefix('#').unwrap_or_default();
    ensure!(
        matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()),
        AgentFormSnafu {
            reason: AgentFormReason::NotHexColor {
                at: what.to_owned(),
                value: color.chars().take(LABEL_MAX).collect()
            }
        }
    );
    Ok(())
}

#[cfg(test)]
#[path = "bounds_tests.rs"]
mod bounds_tests;
