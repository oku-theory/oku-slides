//! Grille d'ajout : widgets d'élément des packs épinglés, leur départ, et les slides qui en acceptent.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use oku_slides_core::{
    Category, Namespace, PackFiles, PackKind, PackPin, PackVersion, ParamType, SlotMode, Style, WidgetFile, qualify,
    split_kind,
};

use crate::error::{PackManifestSnafu, PackVersionMissingSnafu};
use crate::example_text::localized_examples;
use crate::pack::PackManifest;
use crate::param_fields::{kind_description, kind_name};
use crate::widgets_edit::trial_fits;
use crate::{Mode, Result, Session};

/// Le contenu de départ d'un élément ajouté depuis la grille.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ElementSeed {
    /// Les valeurs d'exemple du pack, traduites, que le schéma admet.
    pub params: Map<String, Value>,
    /// Le style de départ du manifeste (`styles`), s'il en a un.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
}

/// Un widget d'élément à ajouter : kind, nom montré, description et départ.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AddWidget {
    /// Le kind qualifié (`base/card`).
    pub name: String,
    /// Le nom montré, dans la langue de l'utilisateur.
    pub label: String,
    /// Ce que montre le widget, en une phrase.
    pub description: Option<String>,
    /// Le contenu de départ de l'élément.
    pub seed: ElementSeed,
}

/// Les widgets à ajouter d'un pack de widgets épinglé.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AddPackView {
    /// L'espace de noms du pack.
    pub namespace: String,
    /// Le nom du pack.
    pub name: String,
    /// Ses widgets d'élément, dans l'ordre du registre.
    pub widgets: Vec<AddWidget>,
}

/// Ce qu'une version de pack donne à la grille dans une langue : son nom et le départ de chaque widget.
#[derive(Debug, Clone)]
struct PackSeeds {
    /// Le nom du pack (`PackManifest::name`).
    name: String,
    /// Le départ de chaque widget d'élément connu du registre, par fichier.
    seeds: BTreeMap<WidgetFile, ElementSeed>,
}

/// Les départs déjà calculés, par version de pack et langue : un snapshot ne relit pas leur manifeste.
///
/// Borné par les packs épinglés : tout changement du jeu de packs (une autre version comprise) le
/// vide (`install_widgets`), et une entrée sert une fois par snapshot (le clone est la sortie).
#[derive(Debug, Default)]
pub(crate) struct SeedCache(Mutex<HashMap<(PackVersion, String), PackSeeds>>);

impl SeedCache {
    /// Oublie tous les départs (bibliothèque, packs écartés ou polices changés).
    pub(crate) fn clear(&mut self) {
        self.0.get_mut().unwrap_or_else(PoisonError::into_inner).clear();
    }
}

/// Le premier slot libre déclaré par un layout : `Some(accepts)`, ou `None` s'il n'en a pas.
fn first_free_slot(params: &[oku_slides_core::ParamSchema]) -> Option<Option<Vec<String>>> {
    params.iter().find_map(|p| match &p.ty {
        ParamType::Slot { mode: SlotMode::Free, accepts } => Some(accepts.clone()),
        _ => None,
    })
}

impl Session {
    /// Les packs de la grille d'ajout, dans l'ordre des épinglages.
    ///
    /// Chaque pack de widgets épinglé par le document montré, par espace de noms ; en
    /// édition d'un pack de widgets, lui seul, depuis son manifeste de travail. Un widget
    /// que le registre ne tient pas pour un élément (cassé, de slide) n'y est pas, ni un
    /// pack sans widget à ajouter.
    pub(crate) fn add_packs(&self) -> Vec<AddPackView> {
        let edited = match &self.mode {
            Mode::History { .. } => return Vec::new(),
            Mode::Pack { edit, .. } if edit.kind == PackKind::Widgets => self.edited_pin().map(|(ns, _)| ns),
            Mode::Pack { .. } | Mode::Deck | Mode::Help { .. } => None,
        };
        let pins = self.doc.sources.deck().widgets.iter().filter(|(ns, _)| edited.is_none_or(|e| e == *ns));
        pins.filter_map(|(ns, pin)| self.add_pack_view(ns, pin)).collect()
    }

    /// La section de la grille du pack `ns` épinglé en `pin` ; `None` sans widget à ajouter.
    fn add_pack_view(&self, ns: &Namespace, pin: &PackPin) -> Option<AddPackView> {
        let seeds = match self.pack_seeds(ns, pin) {
            Ok(seeds) => seeds,
            Err(e) => {
                tracing::warn!(pack = %pin.id, error = %e.one_line(), "pack hors de la grille d'ajout");
                return None;
            }
        };
        let widgets: Vec<AddWidget> = seeds
            .seeds
            .into_iter()
            .filter_map(|(file, seed)| {
                let name = qualify(ns, file.as_str());
                let schema = self.registry.get(&name).filter(|w| w.category == Category::Element)?;
                Some(AddWidget {
                    label: kind_name(&name, schema.label.as_deref(), &self.localizer),
                    description: kind_description(&name, schema.description.as_deref(), &self.localizer),
                    name,
                    seed,
                })
            })
            .collect();
        (!widgets.is_empty()).then(|| AddPackView { namespace: ns.as_str().to_owned(), name: seeds.name, widgets })
    }

    /// Le premier slot libre de chaque layout épinglé, et ce qu'il accepte.
    ///
    /// `None` : il accepte tout widget d'élément. Un layout sans slot libre est absent.
    pub(crate) fn free_slots(&self) -> BTreeMap<String, Option<Vec<String>>> {
        let layouts = &self.doc.sources.deck().layouts;
        self.registry
            .iter()
            .filter(|w| w.category == Category::Slide)
            .filter(|w| split_kind(&w.name).is_some_and(|(ns, _)| layouts.keys().any(|l| l.as_str() == ns)))
            .filter_map(|w| Some((w.name.clone(), first_free_slot(&w.params)?)))
            .collect()
    }

    /// Les départs du pack `ns` épinglé en `pin`, tirés du cache ou de son manifeste.
    ///
    /// Recalculés quand le registre connaît un widget d'élément du pack qui n'y est pas encore (sondé depuis).
    fn pack_seeds(&self, ns: &Namespace, pin: &PackPin) -> Result<PackSeeds> {
        let key = (pin.version.clone(), self.localizer.locale().to_owned());
        let files = self.doc.sources.packs.get(&pin.version);
        let files = files.context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        let wanted = |seeds: &PackSeeds| self.element_files(ns, files).all(|f| seeds.seeds.contains_key(f));
        let mut cache = self.seed_cache.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(seeds) = cache.get(&key).filter(|s| wanted(s)) {
            return Ok(seeds.clone());
        }
        let manifest = PackManifest::from_files(files).context(PackManifestSnafu { id: pin.id })?;
        let seeds = self
            .element_files(ns, files)
            .filter_map(|file| {
                let schema = self.registry.get(&qualify(ns, file.as_str()))?;
                let examples = manifest.examples.get(file).map(|v| localized_examples(manifest.id, v, &self.localizer));
                let params = examples.into_iter().flatten().filter(|(k, v)| trial_fits(schema, k, v)).collect();
                let style = manifest.styles.get(file).filter(|s| **s != Style::default()).cloned();
                Some((file.clone(), ElementSeed { params, style }))
            })
            .collect();
        let computed = PackSeeds { name: manifest.name.clone(), seeds };
        cache.insert(key, computed.clone());
        Ok(computed)
    }

    /// Les fichiers de `files` que le registre tient pour des widgets d'élément sous `ns`.
    fn element_files<'a>(&'a self, ns: &'a Namespace, files: &'a PackFiles) -> impl Iterator<Item = &'a WidgetFile> {
        files.widgets().keys().filter(move |f| {
            !f.is_common()
                && self.registry.get(&qualify(ns, f.as_str())).is_some_and(|w| w.category == Category::Element)
        })
    }
}

#[cfg(test)]
#[path = "add_grid_tests.rs"]
mod add_grid_tests;
