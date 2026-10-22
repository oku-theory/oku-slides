//! Valeurs d'exemple des widgets : un jeu de valeurs par widget, dans le pack.
//!
//! Gardées dans `widgets.json` (`PackManifest::examples`) : versionnées, hachées
//! et exportées avec le pack. Un widget sans valeurs gardées montre les valeurs
//! neutres ; il n'a d'entrée qu'à son premier changement, et la perd quand ses
//! valeurs redeviennent neutres.
//!
//! Ils sont la seule source des exemples, layouts compris : en mode pack, la
//! slide `layout-<nom>` du deck de travail n'en est que l'aperçu, refaite
//! depuis le manifeste à chaque écriture (`pack_mode::realign_layouts`) ; une
//! op sur elle s'écrit dans l'exemple (`layout_examples`).
//!
//! Chaque changement passe, en mode pack seulement, par le chemin des
//! écritures du pack (`commit_manifest` → version de la séance, une étape
//! d'annulation du deck mis de côté). Les valeurs se contrôlent comme celles
//! des réglages (`widget_params.rs`) ; une valeur que le schéma n'admet plus
//! est ignorée à l'aperçu et retirée à la prochaine écriture du widget. Une
//! liste gardée que le schéma ne lit plus repart, au geste suivant sur elle
//! (`edit_example_list`), des items par défaut que montre l'aperçu.

use std::hash::{DefaultHasher, Hash, Hasher};

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{
    ListItem, ListOp, ListPath, ListSpec, Namespace, ParamType, ParamValue, WidgetFile, default_items, param_value,
};

use crate::agent::clip;
use crate::error::{
    NotEditingPackSnafu, PackJsonSnafu, TrialEncodeSnafu, TrialUndeclaredSnafu, TrialValueSnafu, UnknownWidgetSnafu,
    WidgetInvalidSnafu,
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
    /// Tirée de l'empreinte de la version éditée, qui couvre `widgets.json` et
    /// donc les valeurs : elle change avec elles (et avec toute autre écriture
    /// du pack) ; pour un pack de layouts, aussi des slides du deck de travail,
    /// ses exemples vivants. `0` hors du mode pack.
    pub(crate) fn examples_revision(&self) -> u64 {
        let Mode::Pack { edit, .. } = &self.mode else { return 0 };
        let base = edit.base.hex().get(..REVISION_HEX).and_then(|h| u64::from_str_radix(h, 16).ok());
        let slides = &self.doc.sources.deck().slides;
        if slides.is_empty() {
            return base.unwrap_or_default();
        }
        let mut hasher = DefaultHasher::new();
        base.hash(&mut hasher);
        // Comparée dans la même session seulement : la forme de débogage suffit.
        format!("{slides:?}").hash(&mut hasher);
        hasher.finish() >> (64 - 4 * REVISION_HEX)
    }

    /// Les valeurs d'exemple de `name` dans le pack `pack` (voir `widget_target`) ; aucune s'il n'en a pas.
    pub(crate) fn example_values(&self, pack: Option<&Namespace>, name: &WidgetFile) -> Result<Map<String, Value>> {
        let t = self.widget_target(pack)?;
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
        ensure!(self.in_pack_mode(), NotEditingPackSnafu);
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
        ensure!(self.in_pack_mode(), NotEditingPackSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        // `param` : le paramètre, ou `param/item/colonne` pour la liste imbriquée d'un item.
        let path = ListPath::parse(param).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let p = schema.param(path.param).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let spec = ListSpec::of(&p.name, &p.ty).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let defaults = default_items(p).context(TrialValueSnafu { param })?;
        self.write_examples(widget, |values| {
            // Lue avant le tri des valeurs gardées : une liste que le schéma ne lit plus repart
            // des défauts (ce que montrent l'aperçu et le tableau) ; une non-liste est une erreur.
            let mut items: Vec<ListItem> = match values.get(path.param) {
                Some(v @ Value::Array(_)) => match param_value(&p.ty, path.param, v.clone()) {
                    Ok(Some(ParamValue::List(items))) => items,
                    Ok(_) | Err(_) => defaults,
                },
                Some(v) => match param_value(&p.ty, path.param, v.clone()).context(TrialValueSnafu { param })? {
                    Some(ParamValue::List(items)) => items,
                    _ => Vec::new(),
                },
                None => defaults,
            };
            values.retain(|k, v| trial_fits(&schema, k, v));
            path.apply(&mut items, &spec, change).context(TrialValueSnafu { param })?;
            let stored = serde_json::to_value(&items).context(TrialEncodeSnafu { param })?;
            values.insert(path.param.to_owned(), stored);
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
    /// (mode pack seulement) ; rend les valeurs écrites.
    ///
    /// Redevenues vides, les valeurs ne sont plus enregistrées. La slide d'un layout
    /// suit son exemple, dans la même étape (`commit_manifest`).
    fn write_examples(
        &mut self,
        widget: &str,
        change: impl FnOnce(&mut Map<String, Value>) -> Result<()>,
    ) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_pack_mode(), NotEditingPackSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let t: WidgetTarget = self.widget_target(None)?;
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
pub(crate) fn emptied(ty: &ParamType, value: &Value) -> Option<Value> {
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
