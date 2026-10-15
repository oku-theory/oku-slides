//! Widgets du deck : la base, surchargée par les widgets valides de la
//! version de pack qu'il embarque (`deck.templates`, cherchée dans
//! `SourceBundle.packs`, jamais dans la bibliothèque) ; chaque fichier est
//! sondé à part.
//!
//! Invariant : le registre de la session (`Session::registry`), celui des
//! sources du document ouvert et celui du rendu du deck sont le même, celui
//! du pack `Session::widgets_templates`, qui est celui du deck ouvert.
//! Tout ce qui change `deck.templates` (ops, annuler, rétablir, fusion,
//! restauration, autre document) passe par ici.
//!
//! Repli : un pack qui fait échouer un rendu (délai, mémoire, arrêt du
//! processus de rendu), celui du deck ou d'un autre emplacement (aperçus,
//! variantes, polices), est écarté pour la session (`fall_back_to_base`) ;
//! la base le remplace, sans changer `deck.templates` ni le pack suivi.
//!
//! Sonde inachevée : des fichiers laissés non sondés (budget épuisé) le
//! sont au rendu suivant du deck, qui résout de nouveau ses widgets, un
//! budget par rendu, jusqu'à ce qu'il n'en reste plus. Un fichier qui
//! dépasse le délai est gardé cassé : il ne compte plus parmi eux. Un
//! fichier non sondé n'est pas dit cassé (avis `session-widgets-broken`) ;
//! la sonde qui le juge cassé l'annonce alors, lui seul.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use snafu::{OptionExt, ResultExt};

use oku_slides_core::{Deck, Moved, Op, PackFiles, PackRef, PackVersion, WidgetFile, WidgetRegistry};
use oku_slides_i18n::Localize;
use oku_slides_render::{FileProbe, PackWidgets, RenderError, RenderErrorKind};
use oku_slides_render_worker::RemoteCompiled;

use crate::deck_theme::render_themed;
use crate::error::{NotRenderedSnafu, OpSnafu, RegistrySnafu, RenderSnafu, SourcesSnafu, UnknownPackSnafu};
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::{Result, Session, charts};

/// Les widgets résolus pour un pack de modèles.
#[derive(Debug, Clone)]
pub(crate) struct DeckWidgets {
    /// La référence de pack dont ils viennent.
    pub(crate) templates: Option<PackRef>,
    /// Le pack qui surcharge la base ; `None` : la base seule.
    pub(crate) pack: Option<PackWidgets>,
    pub(crate) registry: Arc<WidgetRegistry>,
    /// Les fichiers du pack non sondés (budget épuisé).
    pub(crate) unprobed: BTreeSet<WidgetFile>,
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
    /// Les widgets de la version de pack `templates`, lue dans `packs` (les
    /// sources du document) : la base, surchargée par ses widgets valides.
    ///
    /// Version absente des sources : la base, et l'avis `session-pack-missing`.
    /// Pas de widget : la base, sans avis. Chaque fichier est sondé à part :
    /// un widget cassé est écarté (avis `session-widgets-broken`), les autres
    /// restent ; un fichier non sondé (budget épuisé) n'y est pas dit cassé.
    /// Pack écarté pour la session ou registre illisible : la base et l'avis
    /// de son erreur. Seule la base qui ne se lit pas est une erreur.
    pub(crate) fn resolve_widgets(
        &self,
        templates: Option<&PackRef>,
        packs: &BTreeMap<PackVersion, PackFiles>,
    ) -> Result<(DeckWidgets, Option<String>)> {
        self.resolve_widgets_among(templates, packs, None)
    }

    /// `resolve_widgets`, l'avis des cassés réduit à `among` s'il est donné.
    fn resolve_widgets_among(
        &self,
        templates: Option<&PackRef>,
        packs: &BTreeMap<PackVersion, PackFiles>,
        among: Option<&BTreeSet<WidgetFile>>,
    ) -> Result<(DeckWidgets, Option<String>)> {
        let unprobed_base = |notice, unprobed| -> Result<(DeckWidgets, Option<String>)> {
            let registry = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
            Ok((DeckWidgets { templates: templates.cloned(), pack: None, registry, unprobed }, notice))
        };
        let base = |notice| unprobed_base(notice, BTreeSet::new());
        let Some(r) = templates else { return base(None) };
        let Some(files) = packs.get(&r.version) else {
            tracing::warn!(pack = %r.id, version = %r.version, "version du pack absente des sources : widgets de base");
            return base(Some(self.localizer.tr("session-pack-missing")));
        };
        if files.widgets().is_empty() {
            return base(None);
        }
        let probed =
            PackWidgets::new(r.version.hex(), files.widgets().clone()).and_then(|pack| self.worker.probe_pack(&pack));
        let probe = match probed {
            Ok(probe) => probe,
            Err(e) => {
                tracing::warn!(pack = %r.id, error = %oku_slides_platform::errors::one_line(&e), "pack de widgets illisible : widgets de base");
                return base(Some(e.localize(&self.localizer)));
            }
        };
        let notice = self.broken_widgets_notice(&probe.files, &probe.unprobed, among);
        let unprobed = probe.unprobed;
        match probe.pack {
            Some(pack) if self.worker.is_faulty(&pack) => {
                // Nommé par son manifeste, pas par son empreinte.
                let name = PackManifest::from_files(files).map_or_else(|_| pack.id().to_owned(), |m| m.name);
                base(Some(self.localizer.tr_with("render-pack-disabled", &[("pack", name.as_str().into())])))
            }
            Some(pack) => {
                let registry = probe.registry;
                Ok((DeckWidgets { templates: Some(r.clone()), pack: Some(pack), registry, unprobed }, notice))
            }
            None => unprobed_base(notice, unprobed),
        }
    }

    /// L'avis des widgets écartés de `files`, s'il y en a.
    ///
    /// `_commun` n'y est pas : ses widgets dépendants y sont déjà. Ni les
    /// fichiers `unprobed`, pas encore jugés ; seulement ceux de `among`
    /// s'il est donné. Muet pendant une écriture de widgets : le panneau et
    /// l'agent ont les statuts.
    pub(crate) fn broken_widgets_notice(
        &self,
        files: &BTreeMap<WidgetFile, FileProbe>,
        unprobed: &BTreeSet<WidgetFile>,
        among: Option<&BTreeSet<WidgetFile>>,
    ) -> Option<String> {
        if self.widget_edit {
            return None;
        }
        let announced = |n: &WidgetFile| !n.is_common() && !unprobed.contains(n) && among.is_none_or(|a| a.contains(n));
        let names: Vec<&str> =
            files.iter().filter(|(n, p)| announced(n) && !p.is_valid()).map(|(n, _)| n.as_str()).collect();
        (!names.is_empty())
            .then(|| self.localizer.tr_with("session-widgets-broken", &[("names", names.join(", ").into())]))
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
            // Sans emplacement ouvert, rien à poser : il s'ouvrira avec ces widgets.
            if let Some(renderer) = &mut self.renderer
                && let Err(e) = renderer.set_widgets(widgets.pack.as_ref(), widgets.registry.clone())
            {
                // Le monde Typst a pu changer en partie : les widgets d'avant reviennent.
                if let Err(back) = renderer.set_widgets(self.pack.as_ref(), self.registry.clone()) {
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
            unprobed: std::mem::replace(&mut self.widgets_unprobed, widgets.unprobed),
        })
    }

    /// Applique une transaction : rendue d'abord, validée ensuite. L'historique
    /// du replica est définitif ; un rendu raté n'y laisse aucune trace.
    /// Les graphiques ajoutés sont d'abord mis sous forme canonique (données
    /// dans `dataset`, en-têtes traduits).
    ///
    /// Une transaction qui change de pack de modèles (`set_templates`) se
    /// valide avec les widgets du nouveau pack, résolus avant ses ops : elle
    /// peut ajouter un de ses widgets. Le dernier `set_templates` l'emporte,
    /// comme dans le deck obtenu. En cas d'échec, le registre d'avant revient.
    pub fn apply(&mut self, label: &str, mut ops: Vec<Op>, merge: Option<&str>) -> Result<()> {
        // En mode modèles, choisir un thème fait partir le brouillon de ce thème.
        if let Some(id) = self.drafted_theme_choice(&ops) {
            return self.set_edited_theme(id);
        }
        // `commit_deck_widgets` a pris la séance avant d'appeler ici, et la reprend.
        self.end_widget_session();
        self.ensure_editable()?;
        // `set_templates` à `null` : le pack par défaut, référencé dans sa version courante.
        for op in &mut ops {
            if let Op::SetTemplates { templates: t @ None } = op {
                let version = self.library.current(DEFAULT_PACK).context(UnknownPackSnafu { id: DEFAULT_PACK })?;
                *t = Some(PackRef { id: DEFAULT_PACK, version });
            }
        }
        self.check_token_ops(&ops)?;
        self.embed_op_themes(&mut ops)?;
        let templates = ops.iter().rev().find_map(|op| match op {
            Op::SetTemplates { templates } => Some(templates.clone()),
            _ => None,
        });
        let (widgets, notice) = match templates {
            Some(t) if t != self.widgets_templates => {
                // La version voulue entre dans les sources avant d'être lue (en ajout seul).
                if let Some(r) = &t {
                    self.embed_from_library(r)?;
                }
                let (widgets, notice) = self.resolve_widgets(t.as_ref(), &self.doc.sources.packs)?;
                (Some(widgets), notice)
            }
            _ => (None, None),
        };
        let registry = widgets.as_ref().map_or_else(|| self.registry.clone(), |w| w.registry.clone());
        charts::canonicalize_charts(&mut ops, &self.chart_labels, &registry);
        let mut next = self.doc.sources.deck().clone();
        let moves = oku_slides_core::apply_all_moving(&mut next, ops, &registry).context(OpSnafu)?;
        let previous = widgets.map(|w| self.install_widgets(w)).transpose()?;
        self.doc.sources.set_registry(registry);
        // Un repli pendant le rendu pose son avis : il ne vaut plus si les widgets d'avant reviennent.
        let notice_before = self.doc.notice.clone();
        let compiled = match self.commit(next, label, merge, &moves) {
            Ok(compiled) => compiled,
            Err(e) => {
                // Rien n'est appliqué : le registre d'avant revient aussi.
                if let Some(previous) = previous {
                    match self.install_widgets(previous) {
                        Ok(_) => {
                            self.doc.sources.set_registry(self.registry.clone());
                            self.doc.notice = notice_before;
                        }
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

    /// Rend `next`, puis l'inscrit dans les sources sous `label`. Les
    /// éléments de `moves` gardent leur nœud d'une slide à l'autre.
    fn commit(&mut self, next: Deck, label: &str, merge: Option<&str>, moves: &[Moved]) -> Result<RemoteCompiled> {
        let compiled = self.render_deck(&next)?;
        #[cfg(test)]
        if std::mem::take(&mut self.failpoints.sources_apply) {
            return crate::error::NotRenderedSnafu.fail()?;
        }
        self.doc.sources.apply_moving(next, label, merge, moves).context(SourcesSnafu)?;
        Ok(compiled)
    }

    /// Rend `deck` avec les widgets installés. Si leur pack a fait échouer
    /// le rendu (délai, mémoire, arrêt du processus de rendu), les widgets de
    /// base les remplacent (voir `fall_back_to_base`) et le rendu est refait une fois.
    pub(crate) fn render_deck(&mut self, deck: &Deck) -> Result<RemoteCompiled> {
        self.probe_remaining();
        let tokens = self.theme_tokens(deck);
        self.open_deck_renderer()?;
        let renderer = self.renderer.as_mut().context(NotRenderedSnafu)?;
        let e = match render_themed(renderer, deck, tokens.as_ref(), &self.doc.sources.assets) {
            Ok(compiled) => return Ok(compiled),
            Err(e) => e,
        };
        if !self.fall_back_or_log(&e) {
            return Err(e).context(RenderSnafu)?;
        }
        self.open_deck_renderer()?;
        let renderer = self.renderer.as_mut().context(NotRenderedSnafu)?;
        Ok(render_themed(renderer, deck, tokens.as_ref(), &self.doc.sources.assets).context(RenderSnafu)?)
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
        let pack =
            self.pack_name().unwrap_or_else(|| self.pack.as_ref().map(|p| p.id().to_owned()).unwrap_or_default());
        tracing::warn!(pack, error = %oku_slides_platform::errors::one_line(e), "pack de widgets écarté : widgets de base");
        // Un pack déjà écarté ailleurs : la cause (`PackDisabled`) redirait le pack, par son empreinte.
        let notice = match e.kind() {
            RenderErrorKind::PackDisabled => {
                self.localizer.tr_with("render-pack-disabled", &[("pack", pack.as_str().into())])
            }
            _ => self.localizer.tr_with(
                "session-pack-disabled",
                &[("pack", pack.as_str().into()), ("reason", e.localize(&self.localizer).into())],
            ),
        };
        let base = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
        let templates = self.widgets_templates.clone();
        let unprobed = BTreeSet::new();
        self.install_widgets(DeckWidgets { templates, pack: None, registry: base, unprobed })?;
        self.doc.sources.set_registry(self.registry.clone());
        self.set_notice(notice);
        Ok(true)
    }

    /// [`Self::fall_back_to_base`] ; s'il échoue, journalisé et faux : l'appelant
    /// rend l'erreur d'origine `e`, pas celle du repli.
    fn fall_back_or_log(&mut self, e: &RenderError) -> bool {
        self.fall_back_to_base(e).unwrap_or_else(|back| {
            tracing::error!(error = %back.one_line(), "repli sur les widgets de base impossible");
            false
        })
    }

    /// Résout de nouveau les widgets du document si des fichiers restent à sonder.
    ///
    /// Seulement quand le pack installé est celui du document ouvert (pas
    /// en historique, ni au milieu d'un changement de pack). L'avis ne nomme
    /// que les fichiers jusque-là non sondés que cette sonde dit cassés.
    ///
    /// Au mieux : un échec (résolution, installation) est journalisé, les
    /// widgets installés restent et la sonde est reprise au rendu suivant.
    fn probe_remaining(&mut self) {
        let current = self.doc.sources.deck().templates == self.widgets_templates;
        if self.widgets_unprobed.is_empty() || !current || matches!(self.mode, crate::Mode::History { .. }) {
            return;
        }
        let among = self.widgets_unprobed.clone();
        let templates = self.widgets_templates.clone();
        let resolved = self.resolve_widgets_among(templates.as_ref(), &self.doc.sources.packs, Some(&among));
        let installed = resolved.and_then(|(widgets, notice)| {
            tracing::debug!(unprobed = widgets.unprobed.len(), "widgets du pack sondés de nouveau");
            self.install_widgets(widgets).map(|_| notice)
        });
        match installed {
            Ok(notice) => {
                self.doc.sources.set_registry(self.registry.clone());
                if let Some(notice) = notice {
                    self.set_notice(notice);
                }
            }
            Err(e) => tracing::warn!(error = %e.one_line(), "nouvelle sonde des widgets non installée"),
        }
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
        let fell_back = e.render_error().is_some_and(|re| self.fall_back_or_log(re));
        if fell_back { attempt(self) } else { Err(e) }
    }

    /// Le registre suit le pack du document ouvert : s'il a changé
    /// (annuler, rétablir, fusion, restauration, modèles), ses widgets sont
    /// résolus et installés, et les sources les reçoivent. Un pack cassé
    /// laisse la base et pose son avis.
    pub(crate) fn sync_widgets(&mut self) -> Result<()> {
        let templates = self.doc.sources.deck().templates.clone();
        if templates != self.widgets_templates {
            let (widgets, notice) = self.resolve_widgets(templates.as_ref(), &self.doc.sources.packs)?;
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

#[cfg(test)]
#[path = "deck_widgets_fallback_tests.rs"]
mod deck_widgets_fallback_tests;
