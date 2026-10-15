//! Ce que le rapport dit d'une police : substitut, absence, droits restreints.

use oku_slides_i18n::Localizer;

use crate::fonts::{FontChoice, FontDecision};

/// Ce que le rapport dit d'une police : substitut, absence (fichiers trop
/// gros, droits restreints ou introuvable, avec la police de repli retenue
/// nommée, ou sans repli), droits restreints ; `None` pour une police reprise
/// telle quelle.
pub(crate) fn font_notice(c: &FontChoice, l: &Localizer) -> Option<String> {
    let family = ("family", c.family.as_str().into());
    match (&c.decision, c.used.as_deref(), c.restricted) {
        (FontDecision::Substituted { .. }, Some(by), restricted) => {
            let id = if restricted { "import-font-restricted-substituted" } else { "import-font-substituted" };
            Some(l.tr_with(id, &[family, ("by", by.into())]))
        }
        (FontDecision::Missing, _, restricted) => {
            // Le repli est nommé : « police du corps » serait faux (circulaire)
            // quand la famille absente est elle-même la police du corps.
            let Some(by) = c.fallback.as_deref() else {
                return Some(l.tr_with("import-font-missing-no-fallback", &[family]));
            };
            let id = if c.too_large {
                "import-font-too-large"
            } else if restricted {
                "import-font-restricted-missing"
            } else {
                "import-font-missing"
            };
            Some(l.tr_with(id, &[family, ("by", by.into())]))
        }
        (_, _, true) => Some(l.tr_with("import-font-restricted", &[family])),
        (_, _, false) => None,
    }
}
