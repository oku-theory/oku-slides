//! Widgets du deck : la base, surchargée par le `widgets.typ` du pack de
//! ses modèles (`deck.templates`, l'ensemble par défaut s'il n'en choisit
//! pas).
//!
//! Invariant : le registre de la session (`Session::registry`), celui des
//! sources du document ouvert et celui du rendu du deck sont le même, celui
//! de l'ensemble `Session::widgets_templates`, qui est celui du deck ouvert.
//! Tout ce qui change `deck.templates` (ops, annuler, rétablir, fusion,
//! restauration, autre document) passe par ici.

use std::sync::Arc;

use snafu::ResultExt;

use oku_slides_core::{Deck, Op, WidgetRegistry};
use oku_slides_i18n::Localize;
use oku_slides_render::{Compiled, PackWidgets};

use crate::error::{OpSnafu, RegistrySnafu, RenderSnafu, SourcesSnafu};
use crate::{DEFAULT_SET, Result, Session, charts};

/// Les widgets résolus pour un ensemble de modèles.
#[derive(Debug, Clone)]
pub(crate) struct DeckWidgets {
    /// L'ensemble (`deck.templates`) dont ils viennent.
    pub(crate) templates: Option<String>,
    /// Le pack qui surcharge la base ; `None` : la base seule.
    pub(crate) pack: Option<PackWidgets>,
    pub(crate) registry: Arc<WidgetRegistry>,
}

/// Pannes simulées par les tests, chacune consommée par le premier passage.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct Failpoints {
    /// `SourceBundle::apply` échoue dans `Session::apply`, après le rendu.
    pub(crate) sources_apply: bool,
    /// L'installation de widgets échoue.
    pub(crate) install_widgets: bool,
}

impl Session {
    /// Les widgets de l'ensemble `templates` : la base, surchargée par le
    /// `widgets.typ` de son pack s'il en a un.
    ///
    /// Un pack cassé (Typst qui ne compile pas, schéma refusé) n'est pas
    /// une erreur : il est journalisé, la base sert, et l'avis rendu (le
    /// message de l'erreur, localisé comme une erreur de rendu) est à
    /// montrer. Seule la base qui ne se lit pas est une erreur.
    pub(crate) fn resolve_widgets(&mut self, templates: Option<&str>) -> Result<(DeckWidgets, Option<String>)> {
        let id = templates.unwrap_or(DEFAULT_SET);
        let mut notice = None;
        if let Some(source) = self.library.widgets(id) {
            let loaded = PackWidgets::new(id, source).and_then(|pack| Ok((self.registries.get(Some(&pack))?, pack)));
            match loaded {
                Ok((registry, pack)) => {
                    let templates = templates.map(str::to_owned);
                    return Ok((DeckWidgets { templates, pack: Some(pack), registry }, None));
                }
                Err(e) => {
                    tracing::warn!(pack = id, error = %oku_slides_platform::errors::one_line(&e), "pack de widgets cassé : widgets de base");
                    notice = Some(e.localize(&self.localizer));
                }
            }
        }
        let registry = self.registries.get(None).context(RegistrySnafu)?;
        Ok((DeckWidgets { templates: templates.map(str::to_owned), pack: None, registry }, notice))
    }

    /// Installe `widgets` dans le rendu du deck et la session ; les aperçus
    /// et SVG faits avec les anciens sont oubliés. Les sources ne sont pas
    /// touchées : à l'appelant de leur donner le registre (le document qui
    /// les recevra n'est pas toujours encore en place). Rend les widgets
    /// d'avant ; en cas d'échec, rien ne change.
    pub(crate) fn install_widgets(&mut self, widgets: DeckWidgets) -> Result<DeckWidgets> {
        #[cfg(test)]
        if std::mem::take(&mut self.failpoints.install_widgets) {
            return crate::error::NotRenderedSnafu.fail()?;
        }
        if !Arc::ptr_eq(&widgets.registry, &self.registry) || widgets.pack != self.pack {
            if let Err(e) = self.renderer.set_widgets(widgets.pack.as_ref(), widgets.registry.clone()) {
                // Le monde Typst a pu changer en partie : les widgets d'avant reviennent.
                if let Err(back) = self.renderer.set_widgets(self.pack.as_ref(), self.registry.clone()) {
                    tracing::error!(error = %oku_slides_platform::errors::one_line(&back), "widgets d'avant non réinstallés");
                }
                return Err(e).context(RenderSnafu)?;
            }
            self.template_renderer = None;
            self.variant_renderer = None;
            self.template_cache = None;
            self.svg_cache.clear();
        }
        Ok(DeckWidgets {
            templates: std::mem::replace(&mut self.widgets_templates, widgets.templates),
            pack: std::mem::replace(&mut self.pack, widgets.pack),
            registry: std::mem::replace(&mut self.registry, widgets.registry),
        })
    }

    /// Applique une transaction : rendue d'abord, validée ensuite. L'historique
    /// du replica est définitif ; un rendu raté n'y laisse aucune trace.
    /// Les graphiques ajoutés sont d'abord mis sous forme canonique (données
    /// dans `dataset`, en-têtes traduits).
    ///
    /// Une transaction qui change d'ensemble de modèles (`set_templates`) se
    /// valide avec les widgets du nouveau pack, résolus avant ses ops : elle
    /// peut ajouter un de ses widgets. Le dernier `set_templates` l'emporte,
    /// comme dans le deck obtenu. En cas d'échec, le registre d'avant revient.
    pub fn apply(&mut self, label: &str, mut ops: Vec<Op>) -> Result<()> {
        self.ensure_not_in_history()?;
        let templates = ops.iter().rev().find_map(|op| match op {
            Op::SetTemplates { templates } => Some(templates.clone()),
            _ => None,
        });
        let (widgets, notice) = match templates {
            Some(t) if t != self.widgets_templates => {
                let (widgets, notice) = self.resolve_widgets(t.as_deref())?;
                (Some(widgets), notice)
            }
            _ => (None, None),
        };
        let registry = widgets.as_ref().map_or_else(|| self.registry.clone(), |w| w.registry.clone());
        charts::canonicalize_charts(&mut ops, &self.chart_labels, &registry);
        let mut next = self.doc.sources.deck().clone();
        oku_slides_core::apply_all(&mut next, ops, &registry).context(OpSnafu)?;
        let previous = widgets.map(|w| self.install_widgets(w)).transpose()?;
        self.doc.sources.set_registry(registry);
        let compiled = match self.commit(next, label) {
            Ok(compiled) => compiled,
            Err(e) => {
                // Rien n'est appliqué : le registre d'avant revient aussi.
                if let Some(previous) = previous {
                    match self.install_widgets(previous) {
                        Ok(_) => self.doc.sources.set_registry(self.registry.clone()),
                        Err(back) => tracing::error!(error = %back.one_line(), "registre d'avant non réinstallé"),
                    }
                }
                return Err(e);
            }
        };
        self.install(compiled);
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        self.doc.dirty = true;
        self.persist(true);
        Ok(())
    }

    /// Rend `next`, puis l'inscrit dans les sources sous `label`.
    fn commit(&mut self, next: Deck, label: &str) -> Result<Compiled> {
        let compiled = self.renderer.render(&next, &self.doc.sources.assets).context(RenderSnafu)?;
        #[cfg(test)]
        if std::mem::take(&mut self.failpoints.sources_apply) {
            return crate::error::NotRenderedSnafu.fail()?;
        }
        self.doc.sources.apply(next, label).context(SourcesSnafu)?;
        Ok(compiled)
    }

    /// Le registre suit l'ensemble du document ouvert : s'il a changé
    /// (annuler, rétablir, fusion, restauration, modèles), ses widgets sont
    /// résolus et installés, et les sources les reçoivent. Un pack cassé
    /// laisse la base et pose son avis.
    pub(crate) fn sync_widgets(&mut self) -> Result<()> {
        let templates = self.doc.sources.deck().templates.clone();
        if templates != self.widgets_templates {
            let (widgets, notice) = self.resolve_widgets(templates.as_deref())?;
            self.install_widgets(widgets)?;
            if let Some(notice) = notice {
                self.set_notice(notice);
            }
        }
        self.doc.sources.set_registry(self.registry.clone());
        Ok(())
    }
}

#[cfg(test)]
#[path = "deck_widgets_tests.rs"]
mod deck_widgets_tests;
