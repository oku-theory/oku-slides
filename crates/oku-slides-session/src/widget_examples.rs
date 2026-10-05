//! Cas d'exemple des widgets : des jeux de valeurs nommés, dans le pack.
//!
//! Gardés dans `pack.json` (`PackManifest::examples`) : versionnés, hachés
//! et exportés avec le pack. Un widget sans cas enregistrés en a un,
//! implicite ([`EXAMPLE_DEFAULT`], valeurs neutres), écrit seulement à son
//! premier changement ; un widget revenu à ce seul cas implicite n'en
//! garde pas de trace.
//!
//! Chaque changement (valeur, nom, ajout, suppression) passe, en mode
//! modèles seulement, par le chemin des écritures du pack
//! (`commit_manifest` → version de la séance, une étape d'annulation du
//! deck mis de côté). Les valeurs se contrôlent comme celles des réglages
//! (`widget_params.rs`) ; une valeur que le schéma n'admet plus est ignorée
//! à l'aperçu et retirée à la prochaine écriture de son cas. Une liste gardée
//! que le schéma ne lit plus repart, au geste suivant sur elle
//! (`edit_example_list`), des items par défaut que montre l'aperçu.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{ListItem, ListOp, ListSpec, ParamType, ParamValue, WidgetFile, default_items, param_value};

use crate::agent::context::clipped;
use crate::error::{
    ExampleExistsSnafu, InvalidExampleNameSnafu, LastExampleSnafu, NotEditingTemplatesSnafu, PackJsonSnafu,
    TooManyExamplesSnafu, TrialEncodeSnafu, TrialUndeclaredSnafu, TrialValueSnafu, UnknownExampleSnafu,
    UnknownWidgetSnafu, WidgetInvalidSnafu,
};
use crate::pack::{Example, PackManifest};
use crate::widget_params::{checked_value, same_as};
use crate::widget_preview::preview_params;
use crate::widgets_edit::{WidgetTarget, parse_name, trial_fits};
use crate::{Mode, Result, Session};

/// Cas d'exemple d'un widget, au plus.
pub(crate) const EXAMPLES_MAX: usize = 12;
/// Longueur d'un nom de cas, en caractères.
pub(crate) const EXAMPLE_NAME_MAX: usize = 40;
/// Chiffres hexadécimaux de l'empreinte du pack dans la révision : 52 bits,
/// un entier exact en JavaScript.
const REVISION_HEX: usize = 13;

/// Les cas de `name` dans `manifest` : les enregistrés, sinon le cas implicite.
pub(crate) fn examples_in(manifest: &PackManifest, name: &WidgetFile) -> Vec<Example> {
    match manifest.examples.get(name) {
        Some(list) if !list.is_empty() => list.clone(),
        _ => vec![Example::implicit()],
    }
}

/// La position du cas `example` dans `list` (le premier pour `None`), par son nom sans espaces autour.
fn position(list: &[Example], example: Option<&str>) -> Result<usize> {
    let wanted = example.map(str::trim);
    let found = match wanted {
        None => (!list.is_empty()).then_some(0),
        Some(w) => list.iter().position(|x| x.name == w),
    };
    Ok(found.context(UnknownExampleSnafu { name: clipped(wanted.unwrap_or_default(), 80) })?)
}

/// Le cas `example` de `list` (le premier pour `None`).
fn find<'a>(list: &'a [Example], example: Option<&str>) -> Result<&'a Example> {
    let i = position(list, example)?;
    Ok(list.get(i).context(UnknownExampleSnafu { name: String::new() })?)
}

/// Un nouveau nom de cas : sans espaces autour, de 1 à 40 caractères, libre dans `list`.
///
/// `except` : la position du cas renommé, dont le nom actuel reste permis.
fn new_name(list: &[Example], name: &str, except: Option<usize>) -> Result<String> {
    let name = name.trim();
    let count = name.chars().count();
    ensure!((1..=EXAMPLE_NAME_MAX).contains(&count), InvalidExampleNameSnafu { max: EXAMPLE_NAME_MAX });
    let taken = list.iter().enumerate().any(|(i, x)| Some(i) != except && x.name == name);
    ensure!(!taken, ExampleExistsSnafu { name });
    Ok(name.to_owned())
}

impl Session {
    /// Révision des cas d'exemple pour l'interface : un changement relance les aperçus.
    ///
    /// Tirée de l'empreinte de la version éditée, qui couvre `pack.json` et
    /// donc les cas : elle change avec eux (et avec toute autre écriture du
    /// pack). `0` hors du mode modèles.
    pub(crate) fn examples_revision(&self) -> u64 {
        match &self.mode {
            Mode::Templates { edit, .. } => {
                edit.base.hex().get(..REVISION_HEX).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or_default()
            }
            Mode::Deck | Mode::History { .. } => 0,
        }
    }

    /// Noms des cas de chaque widget du pack édité (sans Commun), dans l'ordre ;
    /// vide hors du mode modèles ou si `pack.json` est illisible.
    pub(crate) fn example_names(&self) -> BTreeMap<&str, Vec<String>> {
        let Mode::Templates { edit, .. } = &self.mode else { return BTreeMap::new() };
        let Some(files) = self.doc.sources.packs.get(&edit.base) else { return BTreeMap::new() };
        let Ok(manifest) = PackManifest::from_files(files) else { return BTreeMap::new() };
        files
            .widgets()
            .keys()
            .filter(|n| !n.is_common())
            .map(|n| (n.as_str(), examples_in(&manifest, n).into_iter().map(|x| x.name).collect()))
            .collect()
    }

    /// Les cas du widget `name` dans le pack en cours (le cas implicite s'il n'en a pas).
    pub(crate) fn examples_of(&self, name: &WidgetFile) -> Result<Vec<Example>> {
        let t = self.widget_target()?;
        let manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        Ok(examples_in(&manifest, name))
    }

    /// Les valeurs du cas `example` de `name` (le premier pour `None`).
    pub(crate) fn example_values(&self, name: &WidgetFile, example: Option<&str>) -> Result<Map<String, Value>> {
        let list = self.examples_of(name)?;
        Ok(find(&list, example)?.values.clone())
    }

    /// Ajoute le cas `example` à `widget`, en dernier ; ses valeurs sont celles de `from`, sinon aucune.
    pub fn add_example(&mut self, widget: &str, example: &str, from: Option<&str>) -> Result<()> {
        self.write_examples(widget, |list| {
            ensure!(list.len() < EXAMPLES_MAX, TooManyExamplesSnafu { max: EXAMPLES_MAX });
            let name = new_name(list, example, None)?;
            let values = match from {
                Some(_) => find(list, from)?.values.clone(),
                None => Map::new(),
            };
            list.push(Example { name, values });
            Ok(())
        })
    }

    /// Renomme le cas `from` de `widget` en `to`, à sa place.
    pub fn rename_example(&mut self, widget: &str, from: &str, to: &str) -> Result<()> {
        self.write_examples(widget, |list| {
            let i = position(list, Some(from))?;
            let name = new_name(list, to, Some(i))?;
            if let Some(x) = list.get_mut(i) {
                x.name = name;
            }
            Ok(())
        })
    }

    /// Supprime le cas `example` de `widget` ; refusé pour son dernier cas.
    pub fn remove_example(&mut self, widget: &str, example: &str) -> Result<()> {
        self.write_examples(widget, |list| {
            let i = position(list, Some(example))?;
            ensure!(list.len() > 1, LastExampleSnafu);
            list.remove(i);
            Ok(())
        })
    }

    /// Règle des valeurs du cas `example` de `widget` (le premier pour `None`), tout ou rien.
    ///
    /// `null` ou la valeur neutre effacent une valeur ; un texte vidé ou une liste vidée
    /// (`[]`) restent vides (le cas « Vide »), voir `emptied` ; les
    /// valeurs gardées que le schéma n'admet plus sont retirées. Rend les
    /// valeurs du cas après l'écriture.
    pub fn set_example_values(
        &mut self,
        widget: &str,
        example: Option<&str>,
        values: Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let neutral = preview_params(&schema, &Map::new(), &self.localizer);
        let mut planned = Vec::with_capacity(values.len());
        for (param, value) in values {
            let p = schema.param(&param).context(TrialUndeclaredSnafu { param: clipped(&param, 80) })?;
            let kept = match emptied(&p.ty, &value) {
                Some(empty) => Some(empty),
                None => checked_value(p, value)?.filter(|v| !same_as(neutral.get(&param), v)),
            };
            planned.push((param, kept));
        }
        let mut written = Map::new();
        self.write_examples(widget, |list| {
            let i = position(list, example)?;
            if let Some(x) = list.get_mut(i) {
                x.values.retain(|k, v| trial_fits(&schema, k, v));
                for (param, kept) in planned {
                    match kept {
                        Some(v) => x.values.insert(param, v),
                        None => x.values.remove(&param),
                    };
                }
                written = x.values.clone();
            }
            Ok(())
        })?;
        Ok(written)
    }

    /// Change la liste `param` du cas `example` de `widget` (le premier pour `None`) ;
    /// sans valeur gardée, ou gardée mais que le schéma ne lit plus (colonne renommée,
    /// `max` abaissé), la liste part des items par défaut du schéma — ce que montrent
    /// l'aperçu et le tableau ; une valeur gardée qui n'est pas une liste est refusée.
    /// Vidée, la liste reste `[]`. Rend les valeurs du cas après l'écriture.
    pub fn edit_example_list(
        &mut self,
        widget: &str,
        example: Option<&str>,
        param: &str,
        change: ListOp,
    ) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let p = schema.param(param).context(TrialUndeclaredSnafu { param: clipped(param, 80) })?;
        let spec = ListSpec::of(&p.name, &p.ty).context(TrialUndeclaredSnafu { param: clipped(param, 80) })?;
        let defaults = default_items(p).context(TrialValueSnafu { param })?;
        let mut written = Map::new();
        self.write_examples(widget, |list| {
            let i = position(list, example)?;
            if let Some(x) = list.get_mut(i) {
                // Lue avant le tri des valeurs gardées : une liste que le schéma ne lit plus repart
                // des défauts (ce que montrent l'aperçu et le tableau) ; une non-liste est une erreur.
                let mut items: Vec<ListItem> = match x.values.get(param) {
                    Some(v @ Value::Array(_)) => match param_value(&p.ty, param, v.clone()) {
                        Ok(Some(ParamValue::List(items))) => items,
                        Ok(_) | Err(_) => defaults,
                    },
                    Some(v) => match param_value(&p.ty, param, v.clone()).context(TrialValueSnafu { param })? {
                        Some(ParamValue::List(items)) => items,
                        _ => Vec::new(),
                    },
                    None => defaults,
                };
                x.values.retain(|k, v| trial_fits(&schema, k, v));
                change.apply(&mut items, &spec).context(TrialValueSnafu { param })?;
                let stored = serde_json::to_value(&items).context(TrialEncodeSnafu { param })?;
                x.values.insert(param.to_owned(), stored);
                written = x.values.clone();
            }
            Ok(())
        })?;
        Ok(written)
    }

    /// Écriture de l'agent (outil `set_widget_trial`) : le premier cas, écrit et persisté.
    pub fn set_example_values_by_agent(
        &mut self,
        widget: &str,
        values: Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let written = self.set_example_values(widget, None, values)?;
        self.persist(true);
        Ok(written)
    }

    /// Change les cas de `widget` par `change`, puis écrit le manifeste (mode modèles seulement).
    ///
    /// Revenus au seul cas implicite, les cas ne sont plus enregistrés.
    fn write_examples(&mut self, widget: &str, change: impl FnOnce(&mut Vec<Example>) -> Result<()>) -> Result<()> {
        let file = parse_name(widget)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let t: WidgetTarget = self.widget_target()?;
        ensure!(t.files.widget(&file).is_some(), UnknownWidgetSnafu { name: file.as_str() });
        let mut manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        let mut list = examples_in(&manifest, &file);
        change(&mut list)?;
        if list == [Example::implicit()] {
            manifest.examples.remove(&file);
        } else {
            manifest.examples.insert(file, list);
        }
        let widgets = t.files.widgets().clone();
        self.commit_manifest(&t, manifest, widgets)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "widget_examples_tests.rs"]
mod widget_examples_tests;

#[cfg(test)]
#[path = "widget_examples_list_tests.rs"]
mod widget_examples_list_tests;

/// Un texte ou une liste vidés dans un cas : gardés vides (le cas « Vide »), et non remis
/// à la valeur de l'aperçu comme le ferait `checked_value`. Seul `null` remet la valeur
/// d'aperçu. Une liste vidée sous son `min` passe par `checked_value`, qui la refuse.
fn emptied(ty: &ParamType, value: &Value) -> Option<Value> {
    let blank = |v: &Value| v.as_str().is_some_and(|s| s.trim().is_empty());
    match ty {
        ParamType::List { min: 0, .. } if value.as_array().is_some_and(Vec::is_empty) => Some(Value::Array(Vec::new())),
        ParamType::Plain if blank(value) => Some(Value::String(String::new())),
        ParamType::Rich if blank(value) || value.as_array().is_some_and(|ps| ps.iter().all(paragraph_is_empty)) => {
            Some(Value::Array(Vec::new()))
        }
        _ => None,
    }
}

/// Un paragraphe bien formé (`{"runs": [...]}`) sans texte. Tout autre élément n'est pas
/// « vide » : il passe par `checked_value`, qui le refuse.
fn paragraph_is_empty(p: &Value) -> bool {
    p.get("runs").and_then(Value::as_array).is_some_and(|runs| {
        runs.iter().all(|r| r.get("t").and_then(Value::as_str).is_some_and(|t| t.trim().is_empty()))
    })
}
