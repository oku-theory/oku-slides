//! Aperçus des layouts (`Request::Layouts`) : un rendu par pack de layouts, en cache.
//!
//! Le document montré (le deck, ou en mode pack le deck de travail) liste
//! ses packs de layouts épinglés ; chaque layout s'y rend à sa valeur
//! d'exemple, au style du document (en mode pack, le brouillon du thème).
//! Le cache garde les aperçus d'une version de pack tant que le style ne
//! change pas.

use snafu::ResultExt;

use oku_slides_core::{Deck, PackVersion};

use crate::error::RenderSnafu;
use crate::pack::PackManifest;
use crate::pack_mode::{layout_slides, styled_like};
use crate::param_fields::kind_name;
use crate::{LayoutPackView, LayoutView, LayoutsView, Result, Session, new_renderer};

/// Les aperçus d'une version de pack de layouts, et le deck d'aperçu qui porte leur style.
struct Previews {
    version: PackVersion,
    deck: Deck,
    views: Vec<LayoutView>,
}

/// Aperçus en cache, un par version de pack de layouts.
#[derive(Default)]
pub(crate) struct PreviewCache(Vec<Previews>);

impl PreviewCache {
    /// Oublie tous les aperçus (polices, packs écartés, bibliothèque changés).
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}

/// Même style : même thème, mêmes jetons, paramètres et variables.
fn same_style(a: &Deck, b: &Deck) -> bool {
    a.theme == b.theme && a.tokens == b.tokens && a.params == b.params && a.vars == b.vars
}

impl Session {
    /// Les layouts de chaque pack de layouts épinglé par le document montré, rendus à son style.
    ///
    /// Dans l'ordre des espaces de noms ; un pack écarté, absent ou illisible
    /// n'y est pas (journalisé), les autres restent. En cache tant que rien ne change
    /// (version du pack, thème, jetons, paramètres, variables). Un pack qui
    /// fait échouer l'aperçu (délai, mémoire, arrêt) : repli, avec son avis,
    /// et aperçu refait une fois.
    pub fn layouts_view(&mut self) -> Result<LayoutsView> {
        let pins: Vec<_> = self.doc.sources.deck().layouts.iter().map(|(ns, pin)| (ns.clone(), pin.clone())).collect();
        let versions: Vec<&PackVersion> = pins.iter().map(|(_, pin)| &pin.version).collect();
        self.pack_cache.0.retain(|p| versions.contains(&&p.version));
        let mut packs = Vec::with_capacity(pins.len());
        for (namespace, pin) in pins {
            // Un pack écarté, absent ou illisible est laissé de côté seul : son avis le dit déjà.
            if self.widgets_disabled.contains_key(&namespace) {
                tracing::warn!(pack = %pin.id, namespace = %namespace, "pack de layouts écarté : hors des aperçus");
                continue;
            }
            let Some(files) = self.doc.sources.packs.get(&pin.version).cloned() else {
                tracing::warn!(pack = %pin.id, version = %pin.version, "version du pack de layouts absente : hors des aperçus");
                continue;
            };
            let manifest = match PackManifest::from_files(&files) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(pack = %pin.id, error = %e, "manifeste du pack de layouts illisible : hors des aperçus");
                    continue;
                }
            };
            let style = self.style_deck();
            let cached = self.pack_cache.0.iter().find(|p| p.version == pin.version && same_style(&p.deck, style));
            let layouts = match cached {
                Some(p) => p.views.clone(),
                None => {
                    let deck = styled_like(style, layout_slides(&manifest, &files, &self.localizer));
                    let views = self.with_fallback(|_| Ok(vec![deck.clone()]), |s| s.render_previews(&deck))?;
                    self.pack_cache.0.retain(|p| p.version != pin.version);
                    self.pack_cache.0.push(Previews { version: pin.version, deck, views: views.clone() });
                    views
                }
            };
            // Écarté par le repli de son propre aperçu : laissé de côté aussi.
            if self.widgets_disabled.contains_key(&namespace) {
                tracing::warn!(pack = %pin.id, namespace = %namespace, "pack de layouts écarté par son aperçu");
                continue;
            }
            packs.push(LayoutPackView { namespace, name: manifest.name, layouts });
        }
        Ok(LayoutsView { packs })
    }

    /// Rend `deck`, les slides des layouts au style du document, dans l'emplacement des aperçus.
    fn render_previews(&mut self, deck: &Deck) -> Result<Vec<LayoutView>> {
        let set = self.usable_packs()?;
        let layer = self.theme_layer(deck)?;
        let renderer = match &mut self.pack_renderer {
            Some(r) => r,
            None => self.pack_renderer.insert(new_renderer(
                &self.worker,
                self.render_fonts.clone(),
                &self.localizer,
                &set,
                &self.registry,
            )?),
        };
        let compiled =
            crate::deck_theme::render_themed(renderer, deck, &layer, &self.doc.sources.assets).context(RenderSnafu)?;
        let layout = compiled.layout();
        deck.slides
            .iter()
            .map(|slide| {
                let page = layout.page_of(&slide.id).unwrap_or(1);
                let svg = compiled.svg(page - 1).context(RenderSnafu)?;
                let label = self.registry.get(&slide.kind).and_then(|w| w.label.as_deref());
                Ok(LayoutView {
                    kind: slide.kind.clone(),
                    label: kind_name(&slide.kind, label, &self.localizer),
                    slide: slide.clone(),
                    svg: svg.into(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "pack_mode_previews_tests.rs"]
mod pack_mode_previews_tests;
