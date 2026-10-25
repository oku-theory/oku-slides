//! Rendu du deck : son emplacement, ouvert au premier rendu, et ses polices
//! (celles de l'app et celles du deck). De nouvelles polices font un nouveau
//! monde Typst, donc un nouveau rendu.
//!
//! Démarrage paresseux (t-342) : une session d'onglet (`Session::unrendered`)
//! n'ouvre son emplacement, et donc ne lance son processus de rendu, qu'au
//! premier rendu ; l'espace de travail le demande à la première requête de
//! l'onglet (`ensure_rendered`). Un fichier ouvert ou un onglet repris ne
//! rendent plus le deck vierge avant le leur.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{Result, Session, new_renderer};
use oku_slides_core::WidgetRegistry;
use oku_slides_render::{Fonts, PackSet};
use oku_slides_render_worker::RenderSlot;

/// Un rendu prêt pour de nouvelles polices, pas encore installé.
pub(crate) struct FontsRenderer {
    /// Polices de l'app et du deck.
    fonts: Fonts,
    renderer: RenderSlot,
    /// Nombre de polices du deck parmi elles.
    deck_fonts: usize,
}

impl Session {
    /// Rend le deck s'il ne l'a jamais été (session d'onglet neuve).
    pub(crate) fn ensure_rendered(&mut self) -> Result<()> {
        if self.compiled.is_none() {
            self.rerender()?;
        }
        Ok(())
    }

    /// Ouvre l'emplacement du rendu du deck s'il ne l'est pas encore, avec
    /// les polices et les widgets installés (un pack devenu fautif : la base).
    pub(crate) fn open_deck_renderer(&mut self) -> Result<()> {
        if self.renderer.is_none() {
            let set = self.usable_packs()?;
            let slot = new_renderer(&self.worker, &self.render_fonts, &self.localizer, &set, &self.registry)?;
            self.renderer = Some(slot);
        }
        Ok(())
    }

    /// Nouvelles polices dans les sources : nouveau monde Typst. À l'appelant
    /// de dire si elles ont changé : comparer les polices coûte, reconstruire
    /// le rendu plus encore.
    ///
    /// Un pack devenu fautif ailleurs : repli sur la base d'abord.
    pub(crate) fn refresh_fonts(&mut self) -> Result<()> {
        let set = self.usable_packs()?;
        let fonts = self.fonts_renderer(&self.doc.sources.fonts, &set, &self.registry)?;
        self.install_fonts(fonts);
        Ok(())
    }

    /// Les polices du rendu (app + `fonts` du deck) et leur rendu, sans rien
    /// installer. Les fichiers restent aussi dans les sources (ils sont
    /// enregistrés avec le deck), d'où la copie.
    /// Les widgets du rendu sont ceux des packs `set` (de registre `registry`).
    pub(crate) fn fonts_renderer(
        &self,
        fonts: &BTreeMap<String, Vec<u8>>,
        set: &PackSet,
        registry: &Arc<WidgetRegistry>,
    ) -> Result<FontsRenderer> {
        let render_fonts = self.app_fonts.with(fonts.values().cloned());
        let renderer = new_renderer(&self.worker, &render_fonts, &self.localizer, set, registry)?;
        Ok(FontsRenderer { fonts: render_fonts, renderer, deck_fonts: fonts.len() })
    }

    /// Installe des polices et leur rendu (voir `fonts_renderer`) ; les
    /// aperçus et SVG en cache, faits avec les anciennes, sont oubliés.
    pub(crate) fn install_fonts(&mut self, built: FontsRenderer) {
        self.render_fonts = built.fonts;
        self.renderer = Some(built.renderer);
        self.render_deck_fonts = built.deck_fonts;
        self.pack_renderer = None;
        self.variant_renderer = None;
        self.pack_cache.clear();
        self.seed_cache.clear();
        self.svg_cache.clear();
    }
}

#[cfg(test)]
#[path = "render_fonts_tests.rs"]
mod render_fonts_tests;
