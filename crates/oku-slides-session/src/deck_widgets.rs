//! Widgets du deck : les widgets valides de chaque pack qu'il épingle, de
//! widgets (`deck.widgets`) et de layouts (`deck.layouts`), versions cherchées
//! dans `SourceBundle.packs`, jamais dans la bibliothèque, sous leur nom
//! qualifié ; chaque fichier est sondé à part, dans la sorte de son pack.
//!
//! Invariant : le registre de la session (`Session::registry`), celui des
//! sources du document ouvert et celui du rendu du deck sont le même, celui
//! des packs `Session::widgets_pins`, qui sont ceux du deck ouvert.
//! Tout ce qui change `deck.widgets` ou `deck.layouts` (ops, annuler, rétablir, fusion,
//! restauration, autre document) passe par ici.
//!
//! Un pack est écarté seul : version absente des sources, épingle qui ne
//! dit pas son manifeste (`check_pack_id`), sonde illisible ou pack devenu
//! fautif (voir `deck_fallback`). Une version livrée de `base` ou d'`essentiel`
//! (`is_trusted_pack`) n'est jamais écartée pour une faute (lecture, sonde,
//! rendu) : l'échec remonte ; elle l'est encore si sa version manque aux
//! sources ou si son épingle ment. Ceux qui requièrent un pack écarté le
//! sont aussi (`deck_closure::dependents`) ; un avis
//! `session-packs-disabled` les nomme. Le deck ne change pas. Un
//! assemblage raté du registre n'accuse aucun pack : l'erreur remonte, les
//! widgets installés restent.
//!
//! Sonde inachevée : des fichiers laissés non sondés (budget épuisé) le
//! sont au rendu suivant du deck, qui résout de nouveau ses widgets, un
//! budget par rendu, jusqu'à ce qu'il n'en reste plus. Un fichier qui
//! dépasse le délai est gardé cassé : il ne compte plus parmi eux. Un
//! fichier non sondé n'est pas dit cassé (avis `session-widgets-broken`) ;
//! la sonde qui le juge cassé l'annonce alors, lui seul.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use snafu::{ResultExt, ensure};

use oku_slides_core::{
    Deck, Moved, Namespace, Op, PackFiles, PackPin, PackVersion, WidgetFile, WidgetRegistry, qualify,
};
use oku_slides_i18n::Localize;
use oku_slides_render::{FileProbe, PackSet, PackWidgets, is_trusted_pack};
use oku_slides_render_worker::RemoteCompiled;

use crate::deck_closure::{closure_ops, combined_pins, dependents};
use crate::deck_pack::{check_pack_id, empty_registry, in_process_pack, only_builtins};
use crate::error::{OpSnafu, RegistrySnafu, RenderSnafu, SourcesSnafu, TrustedPackBrokenSnafu};
use crate::pack::PackManifest;
use crate::{Result, Session, charts};

/// Les widgets résolus pour les packs épinglés d'un deck.
#[derive(Debug, Clone)]
pub(crate) struct DeckWidgets {
    /// Les packs épinglés dont ils viennent, des deux sortes (voir `combined_pins`).
    pub(crate) pins: BTreeMap<Namespace, PackPin>,
    /// Les packs utilisables, réduits à leurs widgets valides.
    pub(crate) set: PackSet,
    pub(crate) registry: Arc<WidgetRegistry>,
    /// Par pack, les fichiers non sondés (budget épuisé).
    pub(crate) unprobed: BTreeMap<Namespace, BTreeSet<WidgetFile>>,
    /// Les packs écartés, et leur nom.
    pub(crate) disabled: BTreeMap<Namespace, String>,
}

/// Pannes simulées par les tests, chacune consommée par le premier passage.
#[cfg(test)]
#[derive(Debug, Default)]
#[expect(clippy::struct_excessive_bools, reason = "one simulated failure per flag, independent of the others")]
pub(crate) struct Failpoints {
    /// `SourceBundle::apply` échoue dans `Session::apply`, après le rendu.
    pub(crate) sources_apply: bool,
    /// L'installation de widgets échoue.
    pub(crate) install_widgets: bool,
    /// Le rendu témoin avec les seuls packs de confiance (`deck_fallback`) échoue d'un échec borné.
    pub(crate) control_render: bool,
    /// L'assemblage du registre des packs échoue.
    pub(crate) registry: bool,
}

/// Les packs épinglés après `ops`, appliquées à `pins` (les deux sortes en une carte).
fn pins_after(pins: &BTreeMap<Namespace, PackPin>, ops: &[Op]) -> BTreeMap<Namespace, PackPin> {
    let mut out = pins.clone();
    for op in ops {
        match op {
            Op::AddPack { namespace, pin, .. } => {
                out.insert(namespace.clone(), pin.clone());
            }
            Op::RemovePack { namespace, .. } => {
                out.remove(namespace);
            }
            _ => {}
        }
    }
    out
}

impl Session {
    /// Les widgets des packs `pins` (les deux sortes, voir `combined_pins`), lus dans
    /// `packs` (les sources du document) : les widgets valides de chaque pack utilisable.
    ///
    /// Un pack dont la version manque aux sources, dont l'épingle ne dit pas
    /// le manifeste, dont la sonde échoue ou devenu fautif est écarté, avec
    /// ceux qui le requièrent ; l'avis `session-packs-disabled` les nomme.
    /// Chaque fichier est sondé à part : un widget cassé est écarté (avis
    /// `session-widgets-broken`), les autres restent ; un fichier non sondé
    /// (budget épuisé) n'y est pas dit cassé. L'assemblage raté du registre
    /// est une erreur, qui n'écarte aucun pack.
    pub(crate) fn resolve_widgets(
        &self,
        pins: &BTreeMap<Namespace, PackPin>,
        packs: &BTreeMap<PackVersion, PackFiles>,
    ) -> Result<(DeckWidgets, Option<String>)> {
        self.resolve_widgets_among(pins, packs, None)
    }

    /// `resolve_widgets`, l'avis des cassés réduit à `among` s'il est donné.
    pub(crate) fn resolve_widgets_among(
        &self,
        pins: &BTreeMap<Namespace, PackPin>,
        packs: &BTreeMap<PackVersion, PackFiles>,
        among: Option<&BTreeMap<Namespace, BTreeSet<WidgetFile>>>,
    ) -> Result<(DeckWidgets, Option<String>)> {
        let name_of = |pin: &PackPin, ns: &Namespace| {
            let manifest = packs.get(&pin.version).and_then(|f| PackManifest::from_files(f).ok());
            manifest.map_or_else(|| ns.to_string(), |m| m.name)
        };
        let none = BTreeSet::new();
        let (mut usable, mut disabled, mut unprobed, mut notices) =
            (Vec::new(), BTreeMap::new(), BTreeMap::new(), Vec::new());
        for (ns, pin) in pins {
            let Some(files) = packs.get(&pin.version) else {
                tracing::warn!(pack = %pin.id, version = %pin.version, "version du pack absente des sources : écarté");
                disabled.insert(ns.clone(), ns.to_string());
                continue;
            };
            // Une épingle qui ne dit pas le manifeste de sa version (espace de noms, id, besoins) l'écarte.
            if let Err(e) = check_pack_id(ns, pin, files) {
                tracing::warn!(pack = %pin.id, error = %e.one_line(), "épingle de pack incohérente : écarté");
                disabled.insert(ns.clone(), name_of(pin, ns));
                continue;
            }
            if files.widgets().is_empty() {
                continue;
            }
            // Un pack intégré courant se résout ici, sans sonde : aucun processus de rendu lancé.
            // De confiance, il n'est jamais écarté : illisible, l'erreur remonte.
            if let Some(builtin) = in_process_pack(&pin.version) {
                usable.push(builtin.context(RegistrySnafu)?);
                continue;
            }
            let pack = PackWidgets::from_files(ns.clone(), files);
            let trusted = pack.as_ref().is_ok_and(is_trusted_pack);
            let probe = match pack.and_then(|pack| self.worker.probe_pack(&pack)) {
                Ok(probe) => probe,
                // Une version livrée d'un pack intégré n'est jamais écartée : l'échec de sa sonde remonte.
                Err(e) if trusted => return Err(e).context(RegistrySnafu)?,
                Err(e) => {
                    tracing::warn!(pack = %pin.id, error = %oku_slides_platform::errors::one_line(&e), "pack de widgets illisible : écarté");
                    notices.push(e.localize(&self.localizer));
                    disabled.insert(ns.clone(), name_of(pin, ns));
                    continue;
                }
            };
            // Une version livrée dont un widget ne compile plus (t-1055 : l'ancien `base` qui importe
            // `card` de `lib.typ`) : jamais écartée, l'erreur remonte.
            if trusted {
                let broken: Vec<String> =
                    probe.files.iter().filter(|(_, p)| !p.is_valid()).map(|(n, _)| qualify(ns, n.as_str())).collect();
                ensure!(broken.is_empty(), TrustedPackBrokenSnafu { names: broken.join(", ") });
            }
            let shown = among.map(|a| a.get(ns).unwrap_or(&none));
            notices.extend(self.broken_widgets_notice(ns, &probe.files, &probe.unprobed, shown));
            if !probe.unprobed.is_empty() {
                unprobed.insert(ns.clone(), probe.unprobed);
            }
            match probe.pack {
                Some(pack) if self.worker.is_faulty(&pack) => {
                    disabled.insert(ns.clone(), name_of(pin, ns));
                }
                Some(pack) => usable.push(pack),
                None => {}
            }
        }
        let off: BTreeSet<Namespace> = disabled.keys().cloned().collect();
        for ns in dependents(pins, &off) {
            let name = pins.get(&ns).map_or_else(|| ns.to_string(), |pin| name_of(pin, &ns));
            disabled.insert(ns, name);
        }
        usable.retain(|p| !disabled.contains_key(p.namespace()));
        let set = PackSet::new(usable).context(RegistrySnafu)?;
        // Un assemblage raté n'accuse aucun pack : l'erreur remonte, les widgets installés restent.
        let registry = self.registry_for(&set)?;
        let notice = self.joined_notice(notices, &disabled);
        Ok((DeckWidgets { pins: pins.clone(), set, registry, unprobed, disabled }, notice))
    }

    /// Le registre de `set` : vide sans pack, le registre intégré s'il tient exactement les packs
    /// intégrés courants (sans lancer le processus de rendu dans ces deux cas).
    pub(crate) fn registry_for(&self, set: &PackSet) -> Result<Arc<WidgetRegistry>> {
        #[cfg(test)]
        if !set.is_empty() && self.failpoints.registry {
            let e =
                oku_slides_render::RegistryError::worker(oku_slides_render::RenderError::worker_lost("panne simulée"));
            return Err(e).context(RegistrySnafu)?;
        }
        if set.is_empty() {
            return Ok(empty_registry());
        }
        if only_builtins(set) {
            return Ok(oku_slides_render::builtin_registry(&self.app_fonts).context(RegistrySnafu)?);
        }
        Ok(self.worker.registry_of(set).context(RegistrySnafu)?)
    }

    /// `notices`, suivis de celui des packs écartés `disabled`, en un avis.
    fn joined_notice(&self, mut notices: Vec<String>, disabled: &BTreeMap<Namespace, String>) -> Option<String> {
        if !disabled.is_empty() {
            notices.push(self.packs_disabled_notice(disabled.values().map(String::as_str)));
        }
        (!notices.is_empty()).then(|| notices.join("\n\n"))
    }

    /// L'avis `session-packs-disabled` qui nomme les packs `names`.
    pub(crate) fn packs_disabled_notice<'a>(&self, names: impl Iterator<Item = &'a str>) -> String {
        let names: Vec<&str> = names.collect();
        self.localizer.tr_with("session-packs-disabled", &[("packs", names.join(", ").into())])
    }

    /// L'avis des widgets écartés de `files`, ceux du pack `ns`, s'il y en a.
    ///
    /// `_commun` n'y est pas : ses widgets dépendants y sont déjà. Ni les
    /// fichiers `unprobed`, pas encore jugés ; seulement ceux de `among`
    /// s'il est donné. Muet pendant une écriture de widgets : le panneau et
    /// l'agent ont les statuts. Les widgets y sont nommés qualifiés.
    pub(crate) fn broken_widgets_notice(
        &self,
        ns: &Namespace,
        files: &BTreeMap<WidgetFile, FileProbe>,
        unprobed: &BTreeSet<WidgetFile>,
        among: Option<&BTreeSet<WidgetFile>>,
    ) -> Option<String> {
        if self.widget_edit {
            return None;
        }
        let announced = |n: &WidgetFile| !n.is_common() && !unprobed.contains(n) && among.is_none_or(|a| a.contains(n));
        let names: Vec<String> =
            files.iter().filter(|(n, p)| announced(n) && !p.is_valid()).map(|(n, _)| qualify(ns, n.as_str())).collect();
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
        if !Arc::ptr_eq(&widgets.registry, &self.registry) || widgets.set != self.packs {
            // Sans emplacement ouvert, rien à poser : il s'ouvrira avec ces widgets.
            if let Some(renderer) = &mut self.renderer
                && let Err(e) = renderer.set_widgets(&widgets.set, widgets.registry.clone())
            {
                // Le monde Typst a pu changer en partie : les widgets d'avant reviennent.
                if let Err(back) = renderer.set_widgets(&self.packs, self.registry.clone()) {
                    tracing::error!(error = %oku_slides_platform::errors::one_line(&back), "widgets d'avant non réinstallés");
                }
                return Err(e).context(RenderSnafu)?;
            }
            self.pack_renderer = None;
            self.variant_renderer = None;
            self.pack_cache.clear();
            self.seed_cache.clear();
            self.svg_cache.clear();
        }
        Ok(DeckWidgets {
            pins: std::mem::replace(&mut self.widgets_pins, widgets.pins),
            set: std::mem::replace(&mut self.packs, widgets.set),
            registry: std::mem::replace(&mut self.registry, widgets.registry),
            unprobed: std::mem::replace(&mut self.widgets_unprobed, widgets.unprobed),
            disabled: std::mem::replace(&mut self.widgets_disabled, widgets.disabled),
        })
    }

    /// Applies a transaction: rendered first, validated next.
    ///
    /// The replica history is final; a failed render leaves no trace there.
    /// Added charts are first put in canonical form (data in `dataset`,
    /// translated headers).
    ///
    /// Each `AddPack` is preceded by the requirements the deck does not pin
    /// (`closure_ops`), and each pinned version is embedded first. A
    /// transaction that changes the pinned packs is validated with their
    /// widgets, resolved before its ops: it may add one of their widgets. On
    /// failure, the previous registry comes back.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, if an op is rejected (pack mode, core validation), if a
    /// pack cannot be embedded, or if rendering fails: nothing changes then.
    pub fn apply(&mut self, label: &str, mut ops: Vec<Op>, merge: Option<&str>) -> Result<()> {
        // En mode pack, choisir un thème fait partir le brouillon de ce thème.
        if let Some(ns) = self.drafted_theme_choice(&ops) {
            return self.set_edited_theme(&ns?);
        }
        // `commit_deck_widgets` a pris la séance avant d'appeler ici, et la reprend.
        self.end_widget_session();
        self.ensure_editable()?;
        // Les slides d'aperçu d'un pack de widgets sont en lecture seule ; les
        // slides du deck de travail sont les exemples du pack : leur liste ne change pas.
        self.ensure_no_preview_ops(&ops)?;
        self.ensure_pack_slides(&ops)?;
        if ops.iter().any(|op| matches!(op, Op::AddPack { .. })) {
            ops = closure_ops(&self.library, self.doc.sources.deck(), &ops)?;
        }
        self.check_token_ops(&ops)?;
        self.embed_op_themes(&mut ops)?;
        let pins = pins_after(&combined_pins(self.doc.sources.deck()), &ops);
        let (widgets, notice) = if pins == self.widgets_pins {
            (None, None)
        } else {
            // Les versions voulues entrent dans les sources avant d'être lues (en ajout seul).
            for op in &ops {
                if let Op::AddPack { namespace, pin, .. } = op {
                    self.embed_from_library(namespace, pin)?;
                }
            }
            let (widgets, notice) = self.resolve_widgets(&pins, &self.doc.sources.packs)?;
            (Some(widgets), notice)
        };
        let registry = widgets.as_ref().map_or_else(|| self.registry.clone(), |w| w.registry.clone());
        charts::canonicalize_charts(&mut ops, &self.chart_labels, &registry);
        let mut next = self.doc.sources.deck().clone();
        // Mode pack : une op sur la slide d'un layout s'écrit dans son exemple (`layout_examples`).
        let pack_ops = matches!(self.mode, crate::Mode::Pack { .. }).then(|| ops.clone());
        let moves = oku_slides_core::apply_all_moving(&mut next, ops, &registry).context(OpSnafu)?;
        // Une image collée que le deck cite désormais entre dans les sources (en ajout seul).
        self.embed_op_assets(&next, &registry)?;
        if let Some(pack_ops) = pack_ops
            && self.commit_layout_examples(&next, &pack_ops, label, merge, &moves)?
        {
            return Ok(());
        }
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
        // Des épinglages changés (`AddPack`, `RemovePack`, `SetTheme`) : la pastille des packs se recalcule.
        self.refresh_pack_updates_if_pins_changed();
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

    /// Résout de nouveau les widgets du document si des fichiers restent à sonder.
    ///
    /// Seulement quand les packs installés sont ceux du document ouvert (pas
    /// en historique, ni au milieu d'un changement de packs). L'avis ne nomme
    /// que les fichiers jusque-là non sondés que cette sonde dit cassés.
    ///
    /// Au mieux : un échec (résolution, installation) est journalisé, les
    /// widgets installés restent et la sonde est reprise au rendu suivant.
    pub(crate) fn probe_remaining(&mut self) {
        let current = combined_pins(self.doc.sources.deck()) == self.widgets_pins;
        if self.widgets_unprobed.is_empty() || !current || matches!(self.mode, crate::Mode::History { .. }) {
            return;
        }
        let among = self.widgets_unprobed.clone();
        let pins = self.widgets_pins.clone();
        let resolved = self.resolve_widgets_among(&pins, &self.doc.sources.packs, Some(&among));
        let installed = resolved.and_then(|(widgets, notice)| {
            tracing::debug!(unprobed = widgets.unprobed.len(), "widgets des packs sondés de nouveau");
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

    /// Le registre suit les packs du document ouvert : s'ils ont changé
    /// (annuler, rétablir, fusion, restauration, mode pack), leurs widgets sont
    /// résolus et installés, et les sources les reçoivent. Un pack cassé est
    /// écarté seul et pose son avis.
    pub(crate) fn sync_widgets(&mut self) -> Result<()> {
        let pins = combined_pins(self.doc.sources.deck());
        if pins != self.widgets_pins {
            let (widgets, notice) = self.resolve_widgets(&pins, &self.doc.sources.packs)?;
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
pub(crate) mod deck_widgets_tests;

#[cfg(test)]
#[path = "deck_widgets_fallback_tests.rs"]
mod deck_widgets_fallback_tests;

#[cfg(test)]
#[path = "deck_widgets_pins_tests.rs"]
mod deck_widgets_pins_tests;
