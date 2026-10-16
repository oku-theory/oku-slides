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
            // Sans repli, la cause (trop gros, licence, introuvable) reste dite.
            let (id, by) = match (c.fallback.as_deref(), c.too_large, restricted) {
                (Some(by), true, _) => ("import-font-too-large", Some(by)),
                (Some(by), false, true) => ("import-font-restricted-missing", Some(by)),
                (Some(by), false, false) => ("import-font-missing", Some(by)),
                (None, true, _) => ("import-font-too-large-no-fallback", None),
                (None, false, true) => ("import-font-restricted-no-fallback", None),
                (None, false, false) => ("import-font-missing-no-fallback", None),
            };
            let args = match by {
                Some(by) => vec![family, ("by", by.into())],
                None => vec![family],
            };
            Some(l.tr_with(id, &args))
        }
        (_, _, true) => Some(l.tr_with("import-font-restricted", &[family])),
        (_, _, false) => None,
    }
}
