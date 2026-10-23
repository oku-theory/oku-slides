//! La fermeture d'un thème : lui et tous les thèmes qu'il étend, épinglés par un deck.
//!
//! Un deck épingle son thème et toute la fermeture de ses `extends`
//! (`Deck.themes`, une [`ThemePin`] par espace de noms), chaque version
//! embarquée dans ses sources. La session matérialise cette fermeture depuis
//! la bibliothèque ([`theme_closure`]) ; après une fusion qui la casse (deux
//! `set_theme` concurrents), elle la refait à partir des versions embarquées
//! ([`repaired_closure`]). Un thème introuvable est `UnknownTheme`, une
//! boucle `ThemeCycle`. Le brouillon d'un enfant, sans espace de noms, se
//! rend sur la fermeture de ses seuls parents ([`closure_of_parents`]).

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackVersion, SourceBundle, ThemeManifest, ThemePin, linearize};

use crate::Result;
use crate::deck_theme::manifest_of;
use crate::error::{ThemeVersionMissingSnafu, UnknownThemeSnafu, theme_chain_error};
use crate::theme::ThemeLibrary;

/// Les épinglages d'une fermeture et les fichiers de chacune de ses versions.
pub(crate) type Closure = (BTreeMap<Namespace, ThemePin>, Vec<PackFiles>);

/// L'épinglage de la version `version` du thème `manifest`.
pub(crate) fn pin_of(manifest: &ThemeManifest, version: PackVersion) -> ThemePin {
    ThemePin { id: manifest.id, version, extends: manifest.extends.clone() }
}

/// La fermeture de `ns` dans la bibliothèque `themes`, chaque thème dans sa version courante.
pub(crate) fn theme_closure(themes: &ThemeLibrary, ns: &Namespace) -> Result<Closure> {
    closure_with(ns, |n| library_theme(themes, n))
}

/// La version courante du thème `ns` de `themes`, épinglée, et ses fichiers.
pub(crate) fn library_theme(themes: &ThemeLibrary, ns: &Namespace) -> Option<(ThemePin, PackFiles)> {
    themes.by_namespace(ns).map(|s| (pin_of(&s.manifest, s.version), s.files))
}

/// La fermeture de `ns`, chaque thème trouvé par `find`.
///
/// Un thème que `find` ne trouve pas est `UnknownTheme` ; des `extends` qui
/// bouclent sont `ThemeCycle`. Chaque thème n'est cherché qu'une fois.
pub(crate) fn closure_with(
    ns: &Namespace,
    find: impl FnMut(&Namespace) -> Option<(ThemePin, PackFiles)>,
) -> Result<Closure> {
    let closure = gathered(vec![ns.clone()], find)?;
    linearize(ns, &closure.0).map_err(theme_chain_error)?;
    Ok(closure)
}

/// La fermeture des thèmes `parents`, sous une racine synthétique qu'aucun thème ne tient.
///
/// Celle du brouillon d'un enfant, qui n'a pas encore d'espace de noms : la
/// racine `deck` (réservée) étend `parents` le temps de linéariser
/// ([`parents_order`]), avec la même détection des boucles et le même ordre
/// que [`closure_with`]. Elle n'est pas dans la fermeture rendue.
pub(crate) fn closure_of_parents(
    parents: &[Namespace],
    find: impl FnMut(&Namespace) -> Option<(ThemePin, PackFiles)>,
) -> Result<Closure> {
    let closure = gathered(parents.iter().rev().cloned().collect(), find)?;
    parents_order(parents, &closure.0)?;
    Ok(closure)
}

/// L'ordre linéarisé sous `parents` : celui d'une racine synthétique `deck` qui les étend, sans elle.
pub(crate) fn parents_order(parents: &[Namespace], pins: &BTreeMap<Namespace, ThemePin>) -> Result<Vec<Namespace>> {
    let Some(first) = parents.first() else { return Ok(Vec::new()) };
    // La racine ne se lit jamais : seuls ses `extends` comptent pour linéariser.
    let version = pins.get(first).context(UnknownThemeSnafu { theme: first.as_str() })?.version.clone();
    let root = Namespace::deck();
    let mut rooted = pins.clone();
    rooted.insert(root.clone(), ThemePin { id: Uuid::nil(), version, extends: parents.to_vec() });
    let mut order = linearize(&root, &rooted).map_err(theme_chain_error)?;
    order.retain(|n| *n != root);
    Ok(order)
}

/// Les thèmes de `todo`, trouvés par `find`, et tous ceux qu'ils étendent ; chacun cherché une fois.
fn gathered(
    mut todo: Vec<Namespace>,
    mut find: impl FnMut(&Namespace) -> Option<(ThemePin, PackFiles)>,
) -> Result<Closure> {
    let mut pins = BTreeMap::new();
    let mut files = Vec::new();
    while let Some(next) = todo.pop() {
        if pins.contains_key(&next) {
            continue;
        }
        let (pin, f) = find(&next).context(UnknownThemeSnafu { theme: next.as_str() })?;
        todo.extend(pin.extends.iter().rev().cloned());
        pins.insert(next, pin);
        files.push(f);
    }
    Ok((pins, files))
}

/// Les jetons de chaque thème de la fermeture `closure` de `ns`, dans l'ordre linéarisé.
///
/// `with_root` : `ns` compris, en dernier ; sinon ses seuls parents.
pub(crate) fn chain_tokens(closure: &Closure, ns: &Namespace, with_root: bool) -> Result<Vec<Map<String, Value>>> {
    let mut order = linearize(ns, &closure.0).map_err(theme_chain_error)?;
    if !with_root {
        order.retain(|n| n != ns);
    }
    tokens_in_order(closure, &order)
}

/// Les jetons des thèmes `order` de la fermeture `closure`, dans cet ordre.
pub(crate) fn tokens_in_order(closure: &Closure, order: &[Namespace]) -> Result<Vec<Map<String, Value>>> {
    let (pins, files) = closure;
    let mut chain = Vec::new();
    for n in order {
        let pin = pins.get(n).context(UnknownThemeSnafu { theme: n.as_str() })?;
        let f = files
            .iter()
            .find(|f| f.version() == pin.version)
            .context(ThemeVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        chain.push(manifest_of(pin, f)?.tokens);
    }
    Ok(chain)
}

/// Le thème `ns` tel que `sources` l'embarque : sa version épinglée, sinon la
/// plus récente (`created`) de ses versions embarquées.
fn embedded_theme(sources: &SourceBundle, ns: &Namespace) -> Option<(ThemePin, PackFiles)> {
    let read = |files: &PackFiles| ThemeManifest::from_files(files).ok().filter(|m| m.namespace == *ns);
    if let Some(pin) = sources.deck().themes.get(ns)
        && let Some(files) = sources.themes.get(&pin.version)
        && let Some(m) = read(files).filter(|m| m.id == pin.id)
    {
        return Some((pin_of(&m, pin.version.clone()), files.clone()));
    }
    let newest = sources
        .themes
        .iter()
        .filter_map(|(v, f)| Some((v, f, read(f)?)))
        .max_by(|a, b| a.2.created.cmp(&b.2.created).then(a.0.cmp(b.0)));
    newest.map(|(v, f, m)| (pin_of(&m, v.clone()), f.clone()))
}

/// Le thème `ns` tel que l'une des `sources` l'embarque (dans l'ordre), sinon la
/// version courante de la bibliothèque `themes`.
pub(crate) fn embedded_or_library(
    themes: &ThemeLibrary,
    sources: &[&SourceBundle],
    ns: &Namespace,
) -> Option<(ThemePin, PackFiles)> {
    sources.iter().find_map(|s| embedded_theme(s, ns)).or_else(|| library_theme(themes, ns))
}

/// La fermeture de `ns` refaite pour un deck dont les épinglages ne sont plus
/// exactement elle (fusion concurrente).
///
/// Chaque thème vient de ses versions embarquées dans `sources` (celle qu'il
/// épingle d'abord), sinon de la bibliothèque `themes`.
pub(crate) fn repaired_closure(themes: &ThemeLibrary, sources: &SourceBundle, ns: &Namespace) -> Result<Closure> {
    closure_with(ns, |n| embedded_theme(sources, n).or_else(|| library_theme(themes, n)))
}

#[cfg(test)]
#[path = "theme_closure_tests.rs"]
mod theme_closure_tests;
