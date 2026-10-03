//! Résultats d'un moteur en continu appliqués au texte de la dictée.

use crate::clean;
use crate::stream::Event;

/// Applique un résultat : renvoie `false` si l'événement n'en est pas un.
///
/// Un texte provisoire remplace l'aperçu ; un texte définitif s'ajoute aux
/// phrases et vide l'aperçu.
pub(crate) fn apply(phrases: &mut Vec<Option<String>>, partial: &mut String, event: &Event) -> bool {
    match event {
        Event::Partial(text) => {
            *partial = clean(text);
            true
        }
        Event::Final { text, .. } => {
            let text = clean(text);
            if !text.is_empty() {
                phrases.push(Some(text));
            }
            partial.clear();
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "results_tests.rs"]
mod results_tests;
