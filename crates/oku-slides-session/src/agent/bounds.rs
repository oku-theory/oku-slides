//! Bornes des textes et couleurs venus de l'agent, partagées par les tours, `report` et `render`.

use snafu::ensure;

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

/// Refuse un texte de l'agent trop long (`max` caractères) ou contenant un contrôle invisible.
pub fn check_text(what: &str, text: &str, max: usize) -> Result<()> {
    let n = text.chars().count();
    ensure!(n <= max, AgentFormSnafu { reason: format!("{what}: {n} characters, at most {max}") });
    ensure!(
        !text.chars().any(is_hidden_control),
        AgentFormSnafu { reason: format!("{what}: invisible or bidirectional control characters are not allowed") }
    );
    Ok(())
}

/// Refuse une couleur qui n'est pas `#rgb`, `#rgba`, `#rrggbb` ou `#rrggbbaa`.
pub fn check_hex(what: &str, color: &str) -> Result<()> {
    let hex = color.strip_prefix('#').unwrap_or_default();
    ensure!(
        matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()),
        AgentFormSnafu { reason: format!("{what}: {color} is not a hex color") }
    );
    Ok(())
}

#[cfg(test)]
#[path = "bounds_tests.rs"]
mod bounds_tests;
