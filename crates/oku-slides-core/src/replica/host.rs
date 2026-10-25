//! Option d'un paramètre `host` : `dataset` en colonnes et lignes à identifiant.
//!
//! Écrite, l'option est une `LoroMap` : `dataset`, s'il a la forme reconnue
//! ([`Shape`]), devient `{columns: LoroMovableList<id>, rows:
//! LoroMovableList<{id, cells}>}`, où `cells` porte une valeur par
//! identifiant de colonne ; toute autre clé est une carte opaque ou une
//! valeur LWW. Lignes, colonnes et cellules se rapprochent par identifiant :
//! des éditions concurrentes de cellules, lignes ou colonnes différentes
//! fusionnent.
//!
//! Relu, le `dataset` est remis en règle : colonne en double et ligne en
//! double aux mêmes cellules écartées, cellule absente ou conteneur lue
//! `null`, autre identifiant en double ou invalide remplacé par un
//! identifiant tiré de la position, taille bornée (lignes, colonnes et
//! cellules en tout).

use std::collections::HashSet;

use loro::{Container, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};
use oku_slides_i18n::Message;
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::read::{MAX_DEPTH, loro_to_json_at};
use super::schema::{CELLS, COLUMNS, Ctx, DATASET, ROWS};
use super::write::{Res, Writer, json_to_loro, list_at, map_at, set_value};
use crate::error::{LoroSnafu, ReplicaInner};
use crate::validate::Issue;

/// Lignes relues au plus dans un `dataset` ; au-delà, tronqué et signalé.
pub(super) const MAX_ROWS: usize = 10_000;

/// Colonnes relues au plus dans un `dataset` ; au-delà, tronqué et signalé.
pub(super) const MAX_COLUMNS: usize = 1_000;

/// Cellules relues au plus dans un `dataset` (lignes × colonnes) ; au-delà,
/// les lignes sont tronquées et signalées.
pub(super) const MAX_CELLS: usize = 1_000_000;

/// Caractères au plus dans un identifiant de ligne ou de colonne.
const MAX_ID_CHARS: usize = 64;

/// Un `dataset` de forme reconnue, prêt à écrire en conteneurs.
struct Shape<'a> {
    rows: Vec<&'a [Value]>,
    ids: Vec<&'a str>,
    columns: Vec<&'a str>,
}

/// Les chaînes d'une liste JSON, si elle n'a que des chaînes.
fn strings(v: Option<&Value>) -> Option<Vec<&str>> {
    v?.as_array()?.iter().map(Value::as_str).collect()
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Array(_) | Value::Object(_))
}

/// La forme reconnue : exactement `source`, `ids` et `columns` ; `source`
/// rectangulaire, de scalaires ; `ids` et `columns` des chaînes alignées,
/// valides et uniques ; dans les bornes de la relecture ([`MAX_COLUMNS`],
/// [`row_budget`]). Toute autre forme s'écrit opaque, sans perte : relue en
/// conteneurs, elle serait réparée ou tronquée, donc différente.
fn shape(ds: &Map<String, Value>) -> Option<Shape<'_>> {
    if ds.len() != 3 {
        return None;
    }
    let ids = strings(ds.get("ids"))?;
    let columns = strings(ds.get(COLUMNS))?;
    if columns.len() > MAX_COLUMNS || ids.len() > row_budget(columns.len()) {
        return None;
    }
    let rows: Vec<&[Value]> =
        ds.get("source")?.as_array()?.iter().map(|r| r.as_array().map(Vec::as_slice)).collect::<Option<_>>()?;
    let aligned = ids.len() == rows.len() && rows.iter().all(|r| r.len() == columns.len());
    (aligned && well_formed(&ids) && well_formed(&columns) && rows.iter().flat_map(|r| r.iter()).all(is_scalar))
        .then_some(Shape { rows, ids, columns })
}

/// Lignes relues au plus pour `columns` colonnes, en-tête compris.
///
/// [`MAX_ROWS`], et [`MAX_CELLS`] cellules en tout ; au moins une (l'en-tête).
/// Un `dataset` plus long se relit tronqué : un producteur (l'import) le
/// borne à l'émission.
#[must_use]
pub fn row_budget(columns: usize) -> usize {
    (MAX_CELLS / columns.max(1)).clamp(1, MAX_ROWS)
}

/// Des identifiants valides et uniques, comme les veut la validation du modèle.
fn well_formed(ids: &[&str]) -> bool {
    let mut seen = HashSet::with_capacity(ids.len());
    ids.iter().all(|id| valid_id(id) && seen.insert(*id))
}

/// La carte `{columns, rows}` du `dataset` de `map` : réutilisée si elle ne
/// porte rien d'autre, remplacée sinon.
fn dataset_at(map: &LoroMap) -> Result<LoroMap, ReplicaInner> {
    match map.get(DATASET) {
        Some(ValueOrContainer::Container(Container::Map(m))) if m.keys().all(|k| &*k == COLUMNS || &*k == ROWS) => {
            Ok(m)
        }
        _ => map.insert_container(DATASET, LoroMap::new()).context(LoroSnafu),
    }
}

impl Writer<'_> {
    /// Amène l'option `map` d'un paramètre `host` à l'état de `obj`.
    pub(super) fn sync_host(&self, map: &LoroMap, obj: &Map<String, Value>) -> Res {
        let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !obj.contains_key(k)).collect();
        for key in stale {
            map.delete(&key).context(LoroSnafu)?;
        }
        for (key, value) in obj {
            match value {
                Value::Object(o)
                    if key == DATASET
                        && let Some(s) = shape(o) =>
                {
                    self.sync_dataset(&dataset_at(map)?, &s)?;
                }
                Value::Object(o) => self.sync_map(&map_at(map, key)?, o, Ctx::Opaque)?,
                v => set_value(map, key, json_to_loro(v))?,
            }
        }
        Ok(())
    }

    /// Amène la carte `{columns, rows}` à l'état de `ds`.
    fn sync_dataset(&self, map: &LoroMap, ds: &Shape<'_>) -> Res {
        self.sync_ids(&list_at(map, COLUMNS)?, &ds.columns)?;
        let rows: Vec<Value> = ds
            .ids
            .iter()
            .zip(&ds.rows)
            .map(|(id, row)| {
                let cells: Map<String, Value> =
                    ds.columns.iter().zip(row.iter()).map(|(c, v)| ((*c).to_owned(), v.clone())).collect();
                serde_json::json!({ "id": id, CELLS: cells })
            })
            .collect();
        self.sync_list(&list_at(map, ROWS)?, &rows, &|m, o| sync_row(m, o))
    }

    /// Réconcilie une liste d'identifiants en valeurs ([`Writer::reorder`]).
    fn sync_ids(&self, list: &LoroMovableList, wanted: &[&str]) -> Res {
        let wanted: Vec<String> = wanted.iter().map(|s| (*s).to_owned()).collect();
        let current: Vec<String> = (0..list.len()).map(|i| value_id(list, i)).collect();
        let insert = |at: usize, id: &str| list.insert(at, id).context(LoroSnafu);
        self.reorder(list, &current, &wanted, &insert, &mut |_| Ok(()))
    }
}

/// L'identifiant en valeur à la position `i` ; vide pour un autre contenu.
fn value_id(list: &LoroMovableList, i: usize) -> String {
    match list.get(i) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
        _ => String::new(),
    }
}

/// Amène la ligne `map` à `{id, cells}` : `id` toujours écrit, cellules
/// rapprochées par identifiant de colonne.
fn sync_row(map: &LoroMap, obj: &Map<String, Value>) -> Res {
    let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| k != "id" && k != CELLS).collect();
    for key in stale {
        map.delete(&key).context(LoroSnafu)?;
    }
    set_value(map, "id", json_to_loro(obj.get("id").unwrap_or(&Value::Null)))?;
    let cells = map_at(map, CELLS)?;
    let empty = Map::new();
    let wanted = obj.get(CELLS).and_then(Value::as_object).unwrap_or(&empty);
    let stale: Vec<String> = cells.keys().map(|k| k.to_string()).filter(|k| !wanted.contains_key(k)).collect();
    for key in stale {
        cells.delete(&key).context(LoroSnafu)?;
    }
    for (column, value) in wanted {
        set_value(&cells, column, json_to_loro(value))?;
    }
    Ok(())
}

/// Un identifiant admis : 1 à 64 lettres ASCII, chiffres, `-` ou `_`.
fn valid_id(id: &str) -> bool {
    (1..=MAX_ID_CHARS).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Remplace les identifiants en double ou invalides : `{prefix}{i}`, suivi
/// de `-{n}` s'il est pris. Déterministe : la première occurrence garde le sien.
pub(super) fn repair(ids: Vec<Option<String>>, prefix: char) -> Vec<String> {
    let mut taken: HashSet<String> = HashSet::with_capacity(ids.len());
    let kept: Vec<Option<String>> =
        ids.into_iter().map(|id| id.filter(|id| valid_id(id) && taken.insert(id.clone()))).collect();
    kept.into_iter()
        .enumerate()
        .map(|(i, id)| {
            id.unwrap_or_else(|| {
                let base = format!("{prefix}{i}");
                // Among `taken.len() + 1` distinct suffixes, at least one is free.
                let fresh = std::iter::once(base.clone())
                    .chain((1..=taken.len() + 1).map(|n| format!("{base}-{n}")))
                    .find(|c| !taken.contains(c))
                    .unwrap_or(base);
                taken.insert(fresh.clone());
                fresh
            })
        })
        .collect()
}

fn issue(message: Message) -> Issue {
    Issue { slide: None, element: None, message }
}

/// La liste en `key` de `map`, si c'en est une.
fn movable(map: &LoroMap, key: &str) -> Option<LoroMovableList> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => Some(l),
        _ => None,
    }
}

/// Une ligne lisible : un `id` chaîne et une carte `cells`.
fn row_parts(entry: Option<ValueOrContainer>) -> Option<(String, LoroMap)> {
    let Some(ValueOrContainer::Container(Container::Map(m))) = entry else { return None };
    let id = match m.get("id") {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
        _ => return None,
    };
    match m.get(CELLS) {
        Some(ValueOrContainer::Container(Container::Map(cells))) => Some((id, cells)),
        _ => None,
    }
}

/// Une cellule : un scalaire ; conteneur, valeur composée ou absente, `null`.
fn cell(cells: &LoroMap, column: Option<&str>, depth: u32) -> Value {
    match column.and_then(|c| cells.get(c)) {
        Some(ValueOrContainer::Value(
            v
            @ (LoroValue::Null | LoroValue::Bool(_) | LoroValue::I64(_) | LoroValue::Double(_) | LoroValue::String(_)),
        )) => loro_to_json_at(&v, depth),
        _ => Value::Null,
    }
}

/// Relit la carte `{columns, rows}` d'un `dataset`, de profondeur `depth`,
/// en `{columns, ids, source}` réparé ; `None` si ce n'en est pas une.
pub(super) fn read_dataset(map: &LoroMap, depth: u32, issues: &mut Vec<Issue>) -> Option<Value> {
    if depth >= MAX_DEPTH || map.len() != 2 {
        return None;
    }
    let (columns, rows) = (movable(map, COLUMNS)?, movable(map, ROWS)?);
    // Une colonne déjà vue est un fantôme (les cellules sont rangées par
    // identifiant) : écartée. Un identifiant invalide reste, réparé plus bas.
    let mut seen_columns: HashSet<String> = HashSet::new();
    let raw_columns: Vec<Option<String>> = (0..columns.len().min(MAX_COLUMNS))
        .map(|j| match columns.get(j) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
            _ => None,
        })
        .filter(|c| c.as_ref().is_none_or(|c| seen_columns.insert(c.clone())))
        .collect();
    let max_rows = row_budget(raw_columns.len());
    if columns.len() > MAX_COLUMNS || rows.len() > max_rows {
        issues.push(issue(Message::new("issue-data-truncated")));
    }
    let mut ids = Vec::new();
    let mut source: Vec<Value> = Vec::new();
    // Lignes gardées, par identifiant brut et cellules sérialisées : linéaire
    // dans le nombre de cellules, même pour des milliers de lignes au même id.
    let mut kept: HashSet<(String, String)> = HashSet::new();
    for i in 0..rows.len().min(max_rows) {
        let Some((id, cells)) = row_parts(rows.get(i)) else {
            issues.push(issue(Message::new("issue-unreadable-row")));
            continue;
        };
        let row = Value::Array(raw_columns.iter().map(|c| cell(&cells, c.as_deref(), depth + 3)).collect());
        // Même identifiant et mêmes cellules qu'une ligne gardée : une ligne
        // recréée deux fois (double retrait puis double annulation), écartée.
        if !kept.insert((id.clone(), row.to_string())) {
            continue;
        }
        source.push(row);
        ids.push(Some(id));
    }
    let mut out = Map::new();
    out.insert(COLUMNS.into(), Value::from(repair(raw_columns, 'c')));
    out.insert("ids".into(), Value::from(repair(ids, 'r')));
    out.insert("source".into(), Value::Array(source));
    Some(Value::Object(out))
}
