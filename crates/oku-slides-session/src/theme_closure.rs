//! La fermeture d'un thème : lui et tous les thèmes qu'il étend, épinglés par un deck.
//!
//! Un deck épingle son thème et toute la fermeture de ses `extends`
//! (`Deck.themes`, une [`ThemePin`] par espace de noms), chaque version
//! embarquée dans ses sources. La session matérialise cette fermeture depuis
//! la bibliothèque ([`theme_closure`]) ; après une fusion qui la casse (deux
//! `set_theme` concurrents), elle la refait à partir des versions embarquées
//! ([`repaired_closure`]). Un thème introuvable est `UnknownTheme`, une
//! boucle `ThemeCycle`.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use snafu::OptionExt;

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
fn library_theme(themes: &ThemeLibrary, ns: &Namespace) -> Option<(ThemePin, PackFiles)> {
    themes.by_namespace(ns).map(|s| (pin_of(&s.manifest, s.version), s.files))
}

/// La fermeture de `ns`, chaque thème trouvé par `find`.
///
/// Un thème que `find` ne trouve pas est `UnknownTheme` ; des `extends` qui
/// bouclent sont `ThemeCycle`. Chaque thème n'est cherché qu'une fois.
pub(crate) fn closure_with(
    ns: &Namespace,
    mut find: impl FnMut(&Namespace) -> Option<(ThemePin, PackFiles)>,
) -> Result<Closure> {
    let mut pins = BTreeMap::new();
    let mut files = Vec::new();
    let mut todo = vec![ns.clone()];
    while let Some(next) = todo.pop() {
        if pins.contains_key(&next) {
            continue;
        }
        let (pin, f) = find(&next).context(UnknownThemeSnafu { theme: next.as_str() })?;
        todo.extend(pin.extends.iter().rev().cloned());
        pins.insert(next, pin);
        files.push(f);
    }
    linearize(ns, &pins).map_err(theme_chain_error)?;
    Ok((pins, files))
}

/// Les jetons de chaque thème de la fermeture `closure` de `ns`, dans l'ordre linéarisé.
///
/// `with_root` : `ns` compris, en dernier ; sinon ses seuls parents.
pub(crate) fn chain_tokens(closure: &Closure, ns: &Namespace, with_root: bool) -> Result<Vec<Map<String, Value>>> {
    let (pins, files) = closure;
    let mut order = linearize(ns, pins).map_err(theme_chain_error)?;
    if !with_root {
        order.retain(|n| n != ns);
    }
    let mut chain = Vec::new();
    for n in &order {
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
