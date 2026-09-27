//! Prose riche : `Vec<Paragraph>` ↔ un `LoroText` à marques (Peritext).
//!
//! Chaque paragraphe se termine par [`SEP`] (U+2029), le dernier compris. Les
//! champs d'un `Run` (hors `t`) sont des marques sur ses caractères ; ceux d'un
//! `Paragraph` (hors `runs`) sont des marques sur son séparateur.

use std::collections::BTreeMap;

use loro::{ExpandType, LoroText, LoroValue, StyleConfig, StyleConfigMap, TextDelta};
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::read::loro_to_json;
use super::write::{json_to_loro, sync_text};
use crate::error::{LoroSnafu, ReplicaInner};

/// Séparateur de paragraphes (et non `\n`, qu'un run peut contenir).
pub(crate) const SEP: char = '\u{2029}';
/// Champs d'un `Run`, hors `t`.
pub(super) const RUN_MARKS: &[&str] = &["bold", "italic", "color", "size", "link", "highlight"];
/// Champs d'un `Paragraph`, hors `runs`.
pub(super) const PARA_MARKS: &[&str] = &["list"];

type Attrs = BTreeMap<String, LoroValue>;

/// Marques qui ne s'étendent pas à la frappe qui les suit.
const NOT_EXPANDING: &[&str] = &["link", "list"];

/// Une configuration par marque, ni plus ni moins : la frappe au bout d'un
/// gras reste grasse, pas au bout d'un lien.
pub(super) fn styles() -> StyleConfigMap {
    let mut m = StyleConfigMap::new();
    for key in RUN_MARKS.iter().chain(PARA_MARKS) {
        let expand = if NOT_EXPANDING.contains(key) { ExpandType::None } else { ExpandType::After };
        m.insert((*key).into(), StyleConfig { expand });
    }
    m
}

fn attrs_of(obj: &Map<String, Value>, keys: &[&str]) -> Attrs {
    keys.iter().filter_map(|k| obj.get(*k).map(|v| ((*k).to_owned(), json_to_loro(v)))).collect()
}

/// Marques d'un segment du delta ; une marque retirée (`Null`) n'en est pas une.
fn attrs_in(attributes: Option<impl IntoIterator<Item = (String, LoroValue)>>) -> Attrs {
    attributes.map(|m| m.into_iter().filter(|(_, v)| !matches!(v, LoroValue::Null)).collect()).unwrap_or_default()
}

/// Texte cible et marques voulues, caractère par caractère.
fn flatten(value: &Value) -> (String, Vec<Attrs>) {
    let (mut text, mut marks) = (String::new(), Vec::new());
    for p in value.as_array().map(Vec::as_slice).unwrap_or_default() {
        let Some(p) = p.as_object() else { continue };
        for run in p.get("runs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
            let Some(run) = run.as_object() else { continue };
            let t = run.get("t").and_then(Value::as_str).unwrap_or_default();
            let a = attrs_of(run, RUN_MARKS);
            for c in t.chars() {
                text.push(c);
                marks.push(a.clone());
            }
        }
        text.push(SEP);
        marks.push(attrs_of(p, PARA_MARKS));
    }
    (text, marks)
}

/// Marques actuelles, caractère par caractère.
fn current(text: &LoroText) -> Vec<Attrs> {
    let mut out = Vec::new();
    for d in text.to_delta() {
        if let TextDelta::Insert { insert, attributes } = d {
            out.extend(std::iter::repeat_n(attrs_in(attributes), insert.chars().count()));
        }
    }
    out
}

/// Amène `text` à l'état des paragraphes `value` : texte par diff, puis marques.
///
/// Une marque n'est posée (ou retirée) que sur les plages où elle diffère :
/// réconcilier le même contenu n'ajoute aucune op.
pub(super) fn sync(text: &LoroText, value: &Value) -> Result<(), ReplicaInner> {
    let (target, want) = flatten(value);
    sync_text(text, &target)?;
    let have = current(text);
    for key in RUN_MARKS.iter().chain(PARA_MARKS) {
        let w: Vec<_> = want.iter().map(|a| a.get(*key)).collect();
        let h: Vec<_> = have.iter().map(|a| a.get(*key)).collect();
        let mut i = 0;
        while i < w.len() {
            let mut j = i + 1;
            while j < w.len() && w[j] == w[i] {
                j += 1;
            }
            if (i..j).any(|k| h.get(k).copied().flatten() != w[k]) {
                match w[i] {
                    Some(v) => text.mark(i..j, key, v.clone()).context(LoroSnafu)?,
                    None => text.unmark(i..j, key).context(LoroSnafu)?,
                }
            }
            i = j;
        }
    }
    Ok(())
}

fn push_run(runs: &mut Vec<Value>, t: String, a: &Attrs) {
    if t.is_empty() {
        return;
    }
    let mut obj: Map<String, Value> =
        a.iter().filter(|(k, _)| RUN_MARKS.contains(&k.as_str())).map(|(k, v)| (k.clone(), loro_to_json(v))).collect();
    // Même mise en forme que le run précédent : on le prolonge (forme canonique).
    if let Some(Value::Object(last)) = runs.last_mut() {
        let same = last.iter().filter(|(k, _)| k.as_str() != "t").eq(obj.iter());
        if same && let Some(Value::String(prev)) = last.get_mut("t") {
            prev.push_str(&t);
            return;
        }
    }
    obj.insert("t".into(), Value::String(t));
    runs.push(Value::Object(obj));
}

fn push_paragraph(out: &mut Vec<Value>, runs: Vec<Value>, sep: &Attrs) {
    let mut p = Map::new();
    p.insert("runs".into(), Value::Array(runs));
    for (k, v) in sep.iter().filter(|(k, _)| PARA_MARKS.contains(&k.as_str())) {
        p.insert(k.clone(), loro_to_json(v));
    }
    out.push(Value::Object(p));
}

/// Relit les paragraphes. Un texte sans séparateur final forme un dernier paragraphe.
pub(super) fn read(text: &LoroText) -> Vec<Value> {
    let (mut out, mut runs) = (Vec::new(), Vec::new());
    for d in text.to_delta() {
        let TextDelta::Insert { insert, attributes } = d else { continue };
        let a = attrs_in(attributes);
        let mut chunk = String::new();
        for c in insert.chars() {
            if c == SEP {
                push_run(&mut runs, std::mem::take(&mut chunk), &a);
                push_paragraph(&mut out, std::mem::take(&mut runs), &a);
            } else {
                chunk.push(c);
            }
        }
        push_run(&mut runs, chunk, &a);
    }
    if !runs.is_empty() {
        push_paragraph(&mut out, runs, &Attrs::new());
    }
    out
}

#[cfg(test)]
#[path = "rich_tests.rs"]
mod rich_tests;
