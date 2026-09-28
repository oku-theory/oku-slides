//! Polices du rendu du deck : celles de l'app et celles du deck. De
//! nouvelles polices font un nouveau monde Typst, donc un nouveau rendu.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::WidgetRegistry;
use oku_slides_render::{Fonts, PackWidgets, Renderer};

use crate::{Result, Session, new_renderer};

/// Un rendu prêt pour de nouvelles polices, pas encore installé.
pub(crate) struct FontsRenderer {
    /// Polices de l'app et du deck.
    fonts: Fonts,
    renderer: Renderer,
    /// Nombre de polices du deck parmi elles.
    deck_fonts: usize,
}

impl Session {
    /// Nouvelles polices dans les sources : nouveau monde Typst. À l'appelant
    /// de dire si elles ont changé : comparer les polices coûte, reconstruire
    /// le rendu plus encore.
    pub(crate) fn refresh_fonts(&mut self) -> Result<()> {
        let fonts = self.fonts_renderer(&self.doc.sources.fonts, self.pack.as_ref(), &self.registry)?;
        self.install_fonts(fonts);
        Ok(())
    }

    /// Les polices du rendu (app + `fonts` du deck) et leur rendu, sans rien
    /// installer. Les fichiers restent aussi dans les sources (ils sont
    /// enregistrés avec le deck), d'où la copie.
    /// Les widgets du rendu sont ceux de `pack` (de registre `registry`).
    pub(crate) fn fonts_renderer(
        &self,
        fonts: &BTreeMap<String, Vec<u8>>,
        pack: Option<&PackWidgets>,
        registry: &Arc<WidgetRegistry>,
    ) -> Result<FontsRenderer> {
        let render_fonts = self.app_fonts.with(fonts.values().cloned());
        let renderer = new_renderer(render_fonts.clone(), &self.localizer, pack, registry)?;
        Ok(FontsRenderer { fonts: render_fonts, renderer, deck_fonts: fonts.len() })
    }

    /// Installe des polices et leur rendu (voir `fonts_renderer`) ; les
    /// aperçus et SVG en cache, faits avec les anciennes, sont oubliés.
    pub(crate) fn install_fonts(&mut self, built: FontsRenderer) {
        self.render_fonts = built.fonts;
        self.renderer = built.renderer;
        self.render_deck_fonts = built.deck_fonts;
        self.template_renderer = None;
        self.variant_renderer = None;
        self.template_cache = None;
        self.svg_cache.clear();
    }
}
