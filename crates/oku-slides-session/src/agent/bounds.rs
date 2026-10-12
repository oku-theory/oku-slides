//! Bornes des textes et couleurs venus de l'agent, partagées par les tours, `report` et `render`.

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

/// Refuse un texte de l'agent trop long (`max` caractères) ou contenant un contrôle invisible.
pub fn check_text(what: &str, text: &str, max: usize) -> Result<()> {
    let n = text.chars().count();
    ensure!(n <= max, AgentFormSnafu { reason: AgentFormReason::TooLong { at: what.to_owned(), count: n, max } });
    ensure!(
        !text.chars().any(is_hidden_control),
        AgentFormSnafu { reason: AgentFormReason::HiddenCharacters { at: what.to_owned() } }
    );
    Ok(())
}

/// Refuse des options aux identifiants en double (la valeur choisie doit en désigner une seule, sans
/// dédoublonnage silencieux), ou dont l'identifiant, le libellé ou l'indice dépasse les bornes.
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

/// Refuse une couleur qui n'est pas `#rgb`, `#rgba`, `#rrggbb` ou `#rrggbbaa` (la valeur
/// citée dans le refus est coupée à [`LABEL_MAX`] caractères).
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
