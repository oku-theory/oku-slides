//! Lecture tolérante des valeurs d'exemple d'un manifeste de pack (`examples`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, PoisonError};

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::pack::WidgetFile;

/// Les valeurs d'exemple d'un widget dans `widgets.json`, telles qu'écrites.
#[derive(Deserialize)]
#[serde(untagged)]
enum StoredValues {
    /// Forme actuelle : les valeurs.
    Values(Map<String, Value>),
    /// Forme d'avant t-541 : une liste de cas nommés (`{"name", "values"}`).
    Cases(Vec<StoredCase>),
}

/// Un cas nommé d'avant t-541 : seules ses valeurs comptent.
#[derive(Deserialize)]
struct StoredCase {
    #[serde(default)]
    values: Map<String, Value>,
}

/// Lit `examples` sans rendre illisible le pack qui le porte (t-524) : d'un
/// pack d'avant t-541 (une liste de cas), le premier cas donne les valeurs du
/// widget (celles que montraient l'aperçu et les réglages par défaut), les
/// autres sont oubliés. Une entrée mal formée (nom qui n'est pas celui d'un
/// fichier de widget, valeurs qui ne sont pas un objet) est écartée avec un
/// diagnostic au journal ; les autres restent. Les valeurs vides sont omises.
///
/// Les bornes des valeurs (`PLAIN_MAX`…) dépendent du schéma du widget : elles
/// s'appliquent à leur usage (`trial_fits`), pas ici.
pub(crate) fn lenient_examples<'de, D>(de: D) -> std::result::Result<BTreeMap<WidgetFile, Map<String, Value>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Value::Object(stored) = Value::deserialize(de)? else {
        diagnose("", "`examples` n'est pas un objet, ignoré");
        return Ok(BTreeMap::new());
    };
    Ok(stored
        .into_iter()
        .filter_map(|(name, v)| {
            let Some(widget) = WidgetFile::parse(&name) else {
                diagnose(&name, "valeurs d'exemple d'un nom de widget invalide, ignorées");
                return None;
            };
            let values = match serde_json::from_value(v) {
                Ok(StoredValues::Values(m)) => m,
                Ok(StoredValues::Cases(cases)) => cases.into_iter().next().map(|c| c.values).unwrap_or_default(),
                Err(e) => {
                    diagnose(&name, &format!("valeurs d'exemple mal formées, ignorées : {e}"));
                    return None;
                }
            };
            (!values.is_empty()).then_some((widget, values))
        })
        .collect())
}

/// Problèmes distincts déjà signalés : au-delà, plus rien n'est retenu.
const DIAGNOSED_MAX: usize = 256;

/// Signale un problème de `examples` : au journal (`warn`) la première fois
/// dans le processus, puis en `debug` (le manifeste est relu à chaque accès
/// à la bibliothèque).
fn diagnose(widget: &str, problem: &str) {
    static SEEN: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
    let key = format!("{widget}\0{problem}");
    let first = {
        let mut seen = SEEN.lock().unwrap_or_else(PoisonError::into_inner);
        seen.len() < DIAGNOSED_MAX && seen.insert(key)
    };
    if first {
        tracing::warn!(widget, "widgets.json : {problem}");
    } else {
        tracing::debug!(widget, "widgets.json : {problem}");
    }
}
