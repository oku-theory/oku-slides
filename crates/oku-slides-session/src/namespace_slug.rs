//! L'espace de noms d'un nouveau pack de la bibliothèque, tiré de son nom.
//!
//! Minuscules ASCII, chiffres et `-` ; les lettres accentuées latines
//! courantes perdent leur accent, tout autre signe devient `-`. Une lettre en
//! tête (sinon `pack-` devant), 64 octets au plus. Pris par un pack ou un thème de la
//! bibliothèque, ou `deck` (réservé) : `-2`, `-3`… s'y ajoutent, si bien
//! que `NamespaceTaken` ne survient jamais à l'enregistrement. Les modèles
//! d'une copie renommée suivent ses widgets (`rename_namespace`).

use snafu::OptionExt;

use oku_slides_core::{Namespace, ParamValue, Params, Slide, qualify, split_kind};

use crate::Result;
use crate::error::UnknownNamespaceSnafu;
use crate::library_packs::PackLibrary;

/// Longueur maximale d'un espace de noms, en octets (celle du cœur).
const MAX: usize = 64;

/// La lettre ASCII de `c` sans son accent, pour les lettres latines courantes.
fn fold(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'æ' => "ae",
        'ç' => "c",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
        'œ' => "oe",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'ý' | 'ÿ' => "y",
        'ß' => "ss",
        _ => return None,
    })
}

/// Le nom `name` en segment d'espace de noms : `[a-z][a-z0-9-]*`, au plus `max` octets.
pub(crate) fn slug(name: &str, max: usize) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        match fold(c) {
            Some(folded) => out.push_str(folded),
            None if c.is_ascii_alphanumeric() => out.push(c),
            // Un séparateur seul entre deux mots, jamais en tête.
            None if !out.is_empty() && !out.ends_with('-') => out.push('-'),
            None => {}
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_lowercase()) {
        out.insert_str(0, "pack-");
    }
    out.truncate(max);
    out.trim_end_matches('-').to_owned()
}

/// L'espace de noms libre d'un nouveau pack ou thème nommé `name` dans `library`, toutes sortes confondues.
///
/// Ni `deck` ni `deck-layouts`, réservés. Le slug de `name`, sinon suivi de `-2`, `-3`… : parmi autant de candidats
/// que d'espaces de noms pris (plus deux), l'un est libre.
pub(crate) fn library_namespace(library: &PackLibrary, name: &str) -> Result<Namespace> {
    let taken = library.namespaces();
    let free = |ns: &Namespace| !ns.is_deck() && !ns.is_deck_layouts() && !taken.contains(ns);
    let base = slug(name, MAX);
    let candidates = (1..=taken.len() + 2).map(|n| match n {
        1 => base.clone(),
        n => {
            let suffix = format!("-{n}");
            format!("{}{suffix}", slug(name, MAX - suffix.len()))
        }
    });
    let found = candidates.filter_map(|c| Namespace::parse(&c)).find(|ns| free(ns));
    Ok(found.context(UnknownNamespaceSnafu { namespace: base })?)
}

/// `kind` passé de l'espace de noms `from` à `to`, s'il est de `from`.
fn renamed(kind: &mut String, from: &Namespace, to: &Namespace) {
    let name = split_kind(kind).filter(|(ns, _)| *ns == from.as_str()).map(|(_, name)| name.to_owned());
    if let Some(name) = name {
        *kind = qualify(to, &name);
    }
}

/// Les éléments de `params` (slots, listes imbriquées comprises) passés de `from` à `to`.
fn rename_params(params: &mut Params, from: &Namespace, to: &Namespace) {
    for value in params.0.values_mut() {
        match value {
            ParamValue::Slot(elements) => {
                for element in elements {
                    renamed(&mut element.kind, from, to);
                    rename_params(&mut element.params, from, to);
                }
            }
            ParamValue::List(items) => {
                for item in items {
                    rename_params(&mut item.values, from, to);
                }
            }
            ParamValue::Text(_) | ParamValue::Rich(_) | ParamValue::Value(_) => {}
        }
    }
}

/// `slide` et ses éléments : chaque widget de l'espace de noms `from` passe à `to`.
///
/// Pour la copie d'un pack sous un nouvel espace de noms : ses modèles suivent ses widgets.
pub(crate) fn rename_namespace(slide: &mut Slide, from: &Namespace, to: &Namespace) {
    renamed(&mut slide.kind, from, to);
    rename_params(&mut slide.params, from, to);
}

#[cfg(test)]
#[path = "namespace_slug_tests.rs"]
mod namespace_slug_tests;
