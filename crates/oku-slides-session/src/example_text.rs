//! Textes d'exemple traduits des packs intégrés : marqueurs `ftl:` de leurs exemples.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_data::{Data, DataCell, DataId};
use serde_json::{Map, Value};
use uuid::Uuid;

use oku_slides_i18n::Localizer;
use oku_slides_render::BuiltinPack;

use crate::deck_pack::builtin_id;

/// Préfixe d'un texte d'exemple traduit.
const FTL: &str = "ftl:";
/// Seules ces clés se traduisent : un pack ne lit aucun autre texte de l'app.
const KEY_PREFIX: &str = "session-example-";

/// Les exemples `values` du pack `manifest_id`, traduits s'il est intégré.
///
/// Un pack qui n'est pas intégré garde chaque texte tel quel.
pub(crate) fn localized_examples(manifest_id: Uuid, values: &Map<String, Value>, l: &Localizer) -> Map<String, Value> {
    if is_builtin(manifest_id) { resolved_examples(values, l) } else { values.clone() }
}

/// Les exemples `values` aux marqueurs écrits en clair, pour une copie qui n'est plus intégrée.
pub(crate) fn resolved_examples(values: &Map<String, Value>, l: &Localizer) -> Map<String, Value> {
    values.iter().map(|(k, v)| (k.clone(), resolved(v, l))).collect()
}

/// The example data `data` of pack `manifest_id`, localized if it is built in.
///
/// Markers in labels, column names and text cells read in the language of `l`; a pack that is
/// not built in keeps every text as written.
pub(crate) fn localized_data(
    manifest_id: Uuid,
    data: &BTreeMap<DataId, Arc<Data>>,
    l: &Localizer,
) -> BTreeMap<DataId, Arc<Data>> {
    if is_builtin(manifest_id) { resolved_data(data, l) } else { data.clone() }
}

/// The example data `data`, their markers written out, for a copy that is no longer built in.
pub(crate) fn resolved_data(data: &BTreeMap<DataId, Arc<Data>>, l: &Localizer) -> BTreeMap<DataId, Arc<Data>> {
    data.iter().map(|(id, d)| (id.clone(), Arc::new(resolved_one(d, l)))).collect()
}

/// `data`, its label, column names and text cells localized.
fn resolved_one(data: &Data, l: &Localizer) -> Data {
    let text = |s: &String| text_of(s, l).unwrap_or_else(|| s.clone());
    let mut out = data.clone();
    out.label = data.label.as_ref().map(text);
    if let Some(table) = out.table_mut() {
        for column in &mut table.columns {
            column.name = text(&column.name);
        }
        table.cells = table
            .cells
            .iter()
            .map(|(key, cell)| match cell {
                DataCell::Text(s) => (key.clone(), DataCell::Text(text(s))),
                other => (key.clone(), other.clone()),
            })
            .collect();
    }
    out
}

/// Whether `manifest_id` is a built-in pack's.
fn is_builtin(manifest_id: Uuid) -> bool {
    BuiltinPack::ALL.into_iter().any(|b| builtin_id(b) == manifest_id)
}

/// `value`, ses marqueurs traduits ; récursif sur les objets et les listes.
///
/// Toujours lue par `serde_json`, dont la profondeur est bornée à 128 : la récursion l'est aussi.
fn resolved(value: &Value, l: &Localizer) -> Value {
    match value {
        Value::String(s) => Value::String(text_of(s, l).unwrap_or_else(|| s.clone())),
        Value::Array(items) => Value::Array(items.iter().map(|v| resolved(v, l)).collect()),
        Value::Object(map) => Value::Object(resolved_examples(map, l)),
        other => other.clone(),
    }
}

/// Le texte du marqueur `s` (`ftl:session-example-…`) ; `None` pour un autre texte ou une clé absente.
fn text_of(s: &str, l: &Localizer) -> Option<String> {
    let key = s.strip_prefix(FTL).filter(|k| k.starts_with(KEY_PREFIX))?;
    let text = l.try_tr(key);
    if text.is_none() {
        tracing::warn!(key, "texte d'exemple sans traduction : marqueur gardé");
    }
    text
}

#[cfg(test)]
#[path = "example_text_tests.rs"]
mod example_text_tests;
