//! Valeurs d'exemple des widgets : un jeu de valeurs par widget, dans le pack.
//!
//! Gardées dans `pack.json` (`PackManifest::examples`) : versionnées, hachées
//! et exportées avec le pack. Un widget sans valeurs gardées montre les valeurs
//! neutres ; il n'a d'entrée qu'à son premier changement, et la perd quand ses
//! valeurs redeviennent neutres.
//!
//! Chaque changement passe, en mode modèles seulement, par le chemin des
//! écritures du pack (`commit_manifest` → version de la séance, une étape
//! d'annulation du deck mis de côté). Les valeurs se contrôlent comme celles
//! des réglages (`widget_params.rs`) ; une valeur que le schéma n'admet plus
//! est ignorée à l'aperçu et retirée à la prochaine écriture du widget. Une
//! liste gardée que le schéma ne lit plus repart, au geste suivant sur elle
//! (`edit_example_list`), des items par défaut que montre l'aperçu.

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{ListItem, ListOp, ListSpec, ParamType, ParamValue, WidgetFile, default_items, param_value};

use crate::agent::clip;
use crate::error::{
    NotEditingTemplatesSnafu, PackJsonSnafu, TrialEncodeSnafu, TrialUndeclaredSnafu, TrialValueSnafu,
    UnknownWidgetSnafu, WidgetInvalidSnafu,
};
use crate::pack::PackManifest;
use crate::widget_params::{checked_value, same_as};
use crate::widget_preview::preview_params;
use crate::widgets_edit::{WidgetTarget, parse_name, trial_fits};
use crate::{Mode, Result, Session};

/// Chiffres hexadécimaux de l'empreinte du pack dans la révision : 52 bits,
/// un entier exact en JavaScript.
const REVISION_HEX: usize = 13;

impl Session {
    /// Révision des valeurs d'exemple pour l'interface : un changement relance les aperçus.
    ///
    /// Tirée de l'empreinte de la version éditée, qui couvre `pack.json` et
    /// donc les valeurs : elle change avec elles (et avec toute autre écriture
    /// du pack). `0` hors du mode modèles.
    pub(crate) fn examples_revision(&self) -> u64 {
        match &self.mode {
            Mode::Templates { edit, .. } => {
                edit.base.hex().get(..REVISION_HEX).and_then(|h| u64::from_str_radix(h, 16).ok()).unwrap_or_default()
            }
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => 0,
        }
    }

    /// Les valeurs d'exemple de `name` dans le pack en cours (aucune s'il n'en a pas).
    pub(crate) fn example_values(&self, name: &WidgetFile) -> Result<Map<String, Value>> {
        let t = self.widget_target()?;
        let manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        Ok(manifest.examples.get(name).cloned().unwrap_or_default())
    }

    /// Règle des valeurs d'exemple de `widget`, tout ou rien.
    ///
    /// `null` ou la valeur neutre effacent une valeur ; un texte vidé ou une liste vidée
    /// (`[]`) restent vides, voir `emptied` ; les valeurs gardées que le schéma n'admet
    /// plus sont retirées. Rend les valeurs du widget après l'écriture.
    pub fn set_example_values(&mut self, widget: &str, values: Map<String, Value>) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let neutral = preview_params(&schema, &Map::new(), &self.localizer);
        let mut planned = Vec::with_capacity(values.len());
        for (param, value) in values {
            let p = schema.param(&param).context(TrialUndeclaredSnafu { param: clip(&param, 80) })?;
            let kept = match emptied(&p.ty, &value) {
                Some(empty) => Some(empty),
                None => checked_value(p, value)?.filter(|v| !same_as(neutral.get(&param), v)),
            };
            planned.push((param, kept));
        }
        self.write_examples(widget, |values| {
            values.retain(|k, v| trial_fits(&schema, k, v));
            for (param, kept) in planned {
                match kept {
                    Some(v) => values.insert(param, v),
                    None => values.remove(&param),
                };
            }
            Ok(())
        })
    }

    /// Change la liste `param` des valeurs d'exemple de `widget` ; sans valeur gardée,
    /// ou gardée mais que le schéma ne lit plus (colonne renommée, `max` abaissé), la
    /// liste part des items par défaut du schéma — ce que montrent l'aperçu et le
    /// tableau ; une valeur gardée qui n'est pas une liste est refusée. Vidée, la liste
    /// reste `[]`. Rend les valeurs du widget après l'écriture.
    pub fn edit_example_list(&mut self, widget: &str, param: &str, change: ListOp) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let p = schema.param(param).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let spec = ListSpec::of(&p.name, &p.ty).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let defaults = default_items(p).context(TrialValueSnafu { param })?;
        self.write_examples(widget, |values| {
            // Lue avant le tri des valeurs gardées : une liste que le schéma ne lit plus repart
            // des défauts (ce que montrent l'aperçu et le tableau) ; une non-liste est une erreur.
            let mut items: Vec<ListItem> = match values.get(param) {
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
            values.retain(|k, v| trial_fits(&schema, k, v));
            change.apply(&mut items, &spec).context(TrialValueSnafu { param })?;
            let stored = serde_json::to_value(&items).context(TrialEncodeSnafu { param })?;
            values.insert(param.to_owned(), stored);
            Ok(())
        })
    }

    /// Écriture de l'agent (outil `set_widget_trial`) : les valeurs du widget, écrites et persistées.
    pub fn set_example_values_by_agent(
        &mut self,
        widget: &str,
        values: Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let written = self.set_example_values(widget, values)?;
        self.persist(true);
        Ok(written)
    }

    /// Change les valeurs d'exemple de `widget` par `change`, puis écrit le manifeste
    /// (mode modèles seulement) ; rend les valeurs écrites.
    ///
    /// Redevenues vides, les valeurs ne sont plus enregistrées.
    fn write_examples(
        &mut self,
        widget: &str,
        change: impl FnOnce(&mut Map<String, Value>) -> Result<()>,
    ) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let t: WidgetTarget = self.widget_target()?;
        ensure!(t.files.widget(&file).is_some(), UnknownWidgetSnafu { name: file.as_str() });
        let mut manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        let mut values = manifest.examples.remove(&file).unwrap_or_default();
        change(&mut values)?;
        if !values.is_empty() {
            manifest.examples.insert(file, values.clone());
        }
        let widgets = t.files.widgets().clone();
        self.commit_manifest(&t, manifest, widgets)?;
        Ok(values)
    }
}

#[cfg(test)]
#[path = "widget_examples_tests.rs"]
mod widget_examples_tests;

#[cfg(test)]
#[path = "widget_examples_list_tests.rs"]
mod widget_examples_list_tests;

/// Un texte ou une liste vidés : gardés vides (pour essayer le widget sans eux), et non
/// remis à la valeur de l'aperçu comme le ferait `checked_value`. Seul `null` remet la
/// valeur d'aperçu. Une liste vidée sous son `min` passe par `checked_value`, qui la refuse.
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
