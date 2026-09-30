//! Widgets du deck : la base, surchargée par le `widgets.typ` du pack de
//! ses modèles (`deck.templates`, l'ensemble par défaut s'il n'en choisit
//! pas).
//!
//! Invariant : le registre de la session (`Session::registry`), celui des
//! sources du document ouvert et celui du rendu du deck sont le même, celui
//! de l'ensemble `Session::widgets_templates`, qui est celui du deck ouvert.
//! Tout ce qui change `deck.templates` (ops, annuler, rétablir, fusion,
//! restauration, autre document) passe par ici.
//!
//! Repli : un pack qui fait échouer un rendu (délai, mémoire, arrêt du
//! processus de rendu), celui du deck ou d'un autre emplacement (aperçus,
//! variantes, polices), est écarté pour la session (`fall_back_to_base`) ;
//! la base le remplace, sans changer `deck.templates` ni l'ensemble suivi.

use std::sync::Arc;

use snafu::ResultExt;

use oku_slides_core::{Deck, Op, WidgetRegistry};
use oku_slides_i18n::Localize;
use oku_slides_render::{PackWidgets, RenderError};
use oku_slides_render_worker::RemoteCompiled;

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
    /// Un pack cassé (Typst qui ne compile pas, schéma refusé, sonde qui
    /// dépasse ses bornes ou pack déjà écarté) n'est pas une erreur : il est journalisé, la base sert, et l'avis rendu (le
    /// message de l'erreur, localisé comme une erreur de rendu) est à
    /// montrer. Seule la base qui ne se lit pas est une erreur.
    pub(crate) fn resolve_widgets(&mut self, templates: Option<&str>) -> Result<(DeckWidgets, Option<String>)> {
        let id = templates.unwrap_or(DEFAULT_SET);
        let mut notice = None;
        if let Some(source) = self.library.widgets(id) {
            let loaded = PackWidgets::new(id, source).and_then(|pack| Ok((self.worker.probe(&pack)?, pack)));
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
        let registry = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
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
    pub fn apply(&mut self, label: &str, mut ops: Vec<Op>, merge: Option<&str>) -> Result<()> {
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
        let compiled = match self.commit(next, label, merge) {
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
    fn commit(&mut self, next: Deck, label: &str, merge: Option<&str>) -> Result<RemoteCompiled> {
        let compiled = self.render_deck(&next)?;
        #[cfg(test)]
        if std::mem::take(&mut self.failpoints.sources_apply) {
            return crate::error::NotRenderedSnafu.fail()?;
        }
        self.doc.sources.apply(next, label, merge).context(SourcesSnafu)?;
        Ok(compiled)
    }

    /// Rend `deck` avec les widgets installés. Si leur pack a fait échouer
    /// le rendu (délai, mémoire, arrêt du processus de rendu), les widgets de
    /// base les remplacent (voir `fall_back_to_base`) et le rendu est refait une fois.
    pub(crate) fn render_deck(&mut self, deck: &Deck) -> Result<RemoteCompiled> {
        let e = match self.renderer.render(deck, &self.doc.sources.assets) {
            Ok(compiled) => return Ok(compiled),
            Err(e) => e,
        };
        if !self.fall_back_to_base(&e)? {
            return Err(e).context(RenderSnafu)?;
        }
        Ok(self.renderer.render(deck, &self.doc.sources.assets).context(RenderSnafu)?)
    }

    /// Repli : si `e` est un échec borné et le pack installé fautif, les
    /// widgets de base le remplacent (rendu du deck, session, sources) et un
    /// avis nomme le pack et la cause. Vrai si le repli a eu lieu.
    ///
    /// Tout emplacement y passe : deck, aperçus de modèles, variantes, polices.
    pub(crate) fn fall_back_to_base(&mut self, e: &RenderError) -> Result<bool> {
        let faulty = self.pack.as_ref().is_some_and(|p| self.worker.is_faulty(p));
        if !(e.kind().is_bounded() && faulty) {
            return Ok(false);
        }
        let pack = self.pack.as_ref().map(|p| p.id().to_owned()).unwrap_or_default();
        tracing::warn!(pack, error = %oku_slides_platform::errors::one_line(e), "pack de widgets écarté : widgets de base");
        let notice = self.localizer.tr_with(
            "session-pack-disabled",
            &[("pack", pack.as_str().into()), ("reason", e.localize(&self.localizer).into())],
        );
        let base = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
        self.install_widgets(DeckWidgets { templates: self.widgets_templates.clone(), pack: None, registry: base })?;
        self.doc.sources.set_registry(self.registry.clone());
        self.set_notice(notice);
        Ok(true)
    }

    /// Le pack installé, sauf s'il est devenu fautif ailleurs : le repli sur
    /// la base est fait d'abord, et `None` rendu.
    pub(crate) fn usable_pack(&mut self) -> Result<Option<PackWidgets>> {
        if let Some(pack) = &self.pack
            && self.worker.is_faulty(pack)
        {
            let e = RenderError::pack_disabled(pack.id());
            self.fall_back_to_base(&e)?;
        }
        Ok(self.pack.clone())
    }

    /// `attempt`, refait une fois après le repli sur la base si le pack
    /// installé l'a fait échouer (voir `fall_back_to_base`).
    pub(crate) fn with_fallback<T>(&mut self, mut attempt: impl FnMut(&mut Self) -> Result<T>) -> Result<T> {
        let e = match attempt(self) {
            Ok(v) => return Ok(v),
            Err(e) => e,
        };
        let fell_back = match e.render_error() {
            Some(re) => self.fall_back_to_base(re)?,
            None => false,
        };
        if fell_back { attempt(self) } else { Err(e) }
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
