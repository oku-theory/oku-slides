//! Les versions qu'épingle un deck sont figées dans la bibliothèque.
//!
//! Un deck en mode deck adopte chaque version de la bibliothèque qu'il
//! épingle, packs et thèmes : la bibliothèque ne la remplacera plus. Le
//! deck de travail du mode pack, le deck mis de côté qui épingle la
//! version de la séance et le brouillon d'une séance de widgets n'adoptent
//! rien : fermer le mode pack ou finir la séance adopte.

use crate::library::draft_lock;
use crate::pack_receive::is_deck_only;
use crate::{Mode, Session};

impl Session {
    /// Fige chaque version de la bibliothèque qu'épingle le deck, en mode deck seulement.
    ///
    /// Un échec est journalisé : le deck embarque la version, rien n'est
    /// perdu ; seul un enregistrement à venir pourrait la remplacer.
    pub(crate) fn adopt_pins(&self) {
        if !matches!(self.mode, Mode::Deck) {
            return;
        }
        let lock = draft_lock();
        let blobs = &self.doc.sources;
        let deck = blobs.deck();
        for (_, _, pin) in deck.all_pins() {
            // Le brouillon d'une séance de widgets n'est pas adopté : la séance le remplace encore.
            if self.is_widget_draft(pin) {
                continue;
            }
            // Déjà figée : rien à faire. Une version inconnue de la bibliothèque reste au deck : une version
            // reçue reste reçue (load précède receive_packs), un brouillon remplacé entre-temps n'est que dans
            // le deck, rien n'est perdu. L'index suffit : pas de chargement sous le verrou.
            if self.library.is_frozen(pin.id, &pin.version) || !self.library.lists(pin.id, &pin.version) {
                continue;
            }
            let Some(files) = blobs.packs.get(&pin.version).filter(|f| !is_deck_only(f)) else { continue };
            if let Err(e) = self.library.adopt(&lock, pin.id, files) {
                tracing::warn!(pack = %pin.id, error = %e.one_line(), "version épinglée non figée");
            }
        }
        for pin in deck.themes.values() {
            if self.themes.is_frozen(pin.id, &pin.version) || !self.themes.lists(pin.id, &pin.version) {
                continue;
            }
            let Some(files) = blobs.themes.get(&pin.version) else { continue };
            if let Err(e) = self.themes.adopt(&lock, pin.id, files) {
                tracing::warn!(theme = %pin.id, error = %e.one_line(), "thème épinglé non figé");
            }
        }
    }
}

#[cfg(test)]
#[path = "deck_adopt_tests.rs"]
mod deck_adopt_tests;
