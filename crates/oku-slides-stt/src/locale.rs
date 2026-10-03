//! Langue du moteur d'Apple : il en exige une, il ne la détecte pas.

use crate::stream::Availability;

/// Première langue connue du moteur : la choisie, sinon l'interface, sinon le système.
///
/// Renvoie l'étiquette retenue (`fr`, `fr-FR`) et sa disponibilité ; `None`
/// si le moteur n'en connaît aucune. Une langue choisie n'a pas de repli.
pub(crate) fn resolve(
    chosen: Option<&str>,
    ui: &str,
    system: Option<&str>,
    availability: impl Fn(&str) -> Availability,
) -> Option<(String, Availability)> {
    let candidates: Vec<&str> = match chosen {
        Some(language) => vec![language],
        None => [Some(ui), system].into_iter().flatten().collect(),
    };
    candidates.into_iter().find_map(|tag| match availability(tag) {
        Availability::Unsupported => None,
        found => Some((tag.to_owned(), found)),
    })
}

/// Code de langue d'une étiquette : `fr-FR` → `fr`.
pub(crate) fn language(tag: &str) -> String {
    tag.split(['-', '_']).next().unwrap_or(tag).to_ascii_lowercase()
}

#[cfg(test)]
#[path = "locale_tests.rs"]
mod locale_tests;
