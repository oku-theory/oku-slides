//! The namespace of a new library pack or theme, drawn from its name.
//!
//! Lowercase ASCII, digits and `-`; common accented Latin letters lose their
//! accent, any other sign becomes `-`. A letter first (else the caller's
//! prefix, `pack-` or `theme-`, goes in front), 64 bytes at most. Taken by a
//! pack or theme of the library, or `deck` (reserved): `-2`, `-3`… are added,
//! so `NamespaceTaken` never happens on save. The templates of a renamed
//! copy follow its widgets (`rename_namespace`).

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

/// The name `name` as a namespace segment: `[a-z][a-z0-9-]*`, at most `max` bytes.
///
/// A name that does not start with a letter gets `<prefix>-` in front.
pub(crate) fn slug(name: &str, max: usize, prefix: &str) -> String {
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
        out.insert_str(0, &format!("{prefix}-"));
    }
    out.truncate(max);
    out.trim_end_matches('-').to_owned()
}

/// The free namespace of a new pack or theme named `name` in `library`, all kinds included.
///
/// Neither `deck` nor `deck-layouts`, reserved. The slug of `name` (`fallback`
/// in front when it does not start with a letter), else followed by `-2`,
/// `-3`…: among as many candidates as taken namespaces (plus two), one is free.
pub(crate) fn library_namespace(library: &PackLibrary, name: &str, fallback: &str) -> Result<Namespace> {
    let taken = library.namespaces();
    let free = |ns: &Namespace| !ns.is_deck() && !ns.is_deck_layouts() && !taken.contains(ns);
    let base = slug(name, MAX, fallback);
    let candidates = (1..=taken.len() + 2).map(|n| match n {
        1 => base.clone(),
        n => {
            let suffix = format!("-{n}");
            format!("{}{suffix}", slug(name, MAX - suffix.len(), fallback))
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
