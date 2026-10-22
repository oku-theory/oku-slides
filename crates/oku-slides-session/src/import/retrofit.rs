//! Le retrofit.
//!
//! Le contenu de la présentation projeté sur des packs de layouts et un thème
//! existants (`oku_slides_import::retrofit`), guidé par les profils de chaque
//! pack cible, dans l'ordre donné. Un pack est désigné par son id ou son nom
//! ([`PackLookup::Library`], l'app), ou aussi son dossier
//! ([`PackLookup::LibraryOrDir`], la CLI) ; sans pack désigné, `essentiel`.
//! Le thème par son id ou son nom, sinon le thème par défaut.
//!
//! Le deck épingle le `base` intégré, puis chaque pack cible dans sa version
//! désignée et ses besoins ; ni les packs cibles ni la bibliothèque ne changent.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{
    Deck, LayoutProfile, Namespace, PackFiles, PackKind, PackManifest, PackPin, SourceBundle, ThemeRef, WidgetRegistry,
};
use oku_slides_import::RetrofitTarget;
use oku_slides_import::retrofit::{self, ColorTokens, PageTokens};
use oku_slides_platform::Platform;

use super::error::{
    BadPackSnafu, BadThemeSnafu, LibrarySnafu, NotLayoutPackSnafu, PresentationSnafu, Result, SourcesSnafu,
    UnknownThemeSnafu,
};
use super::{Input, PackLookup, Prepared};
use crate::deck_closure::closure_with_files;
use crate::deck_pack::builtin_pin;
use crate::deck_theme::manifest_of;
use crate::error::ThemeVersionMissingSnafu;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK};
use crate::theme_schema::default_tokens;
use crate::{
    DEFAULT_THEME, PackLibrary, ThemeLibrary, adopt_default_theme, embed_theme, library_pack, resolve_pack,
    sources_with_packs,
};

/// Les jetons effectifs du thème `r` de `bundle`.
///
/// Les défauts, puis ceux de sa version embarquée.
///
/// Le thème est lu tel quel, sans repli sur le thème par défaut : illisible,
/// absent des sources ou d'un autre id, c'est une erreur. Ses couleurs et sa
/// page guident le retrofit ; les défauts seuls donneraient un deck faux sans le dire.
fn deck_tokens(bundle: &SourceBundle, r: &ThemeRef) -> Result<Map<String, Value>> {
    let files = bundle
        .themes
        .get(&r.version)
        .context(ThemeVersionMissingSnafu { id: r.id, version: r.version.clone() })
        .map_err(crate::SessionError::from)
        .context(BadThemeSnafu)?;
    let own = manifest_of(r, files).context(BadThemeSnafu)?.tokens;
    Ok(effective_tokens(default_tokens(), &own))
}

/// Les fichiers du pack de layouts désigné par `wanted`, cherché selon `lookup`.
///
/// Un pack d'une autre sorte est refusé (`NotLayoutPack`).
fn layout_pack(platform: &Platform, library: &PackLibrary, wanted: &str, lookup: PackLookup) -> Result<PackFiles> {
    let files = match lookup {
        PackLookup::Library => library_pack(library, wanted)?,
        PackLookup::LibraryOrDir => {
            resolve_pack(&platform.storage, library, wanted, platform.settings.sources.max_sources_bytes())?
        }
    };
    ensure!(files.kind() == PackKind::Layouts, NotLayoutPackSnafu { pack: wanted });
    Ok(files)
}

/// Le retrofit de `input` sur les packs de layouts `wanted` (vide : `essentiel`) et le thème `theme`.
///
/// Chaque pack de `wanted` se cherche selon `lookup` ; ses profils s'ajoutent, dans l'ordre.
pub(crate) fn prepare(
    platform: &Platform,
    base: &Arc<WidgetRegistry>,
    (wanted, theme, lookup): (&[String], Option<&str>, PackLookup),
    input: &Input,
) -> Result<Prepared> {
    let library = PackLibrary::user(platform);
    let essentiel = [ESSENTIEL_PACK.to_string()];
    let wanted = if wanted.is_empty() { essentiel.as_slice() } else { wanted };
    let mut deck = Deck::empty();
    // Le retrofit écrit des éléments `base/*` : le `base` intégré, épinglé d'abord.
    let (ns, pin, files) = builtin_pin(BASE_PACK).context(LibrarySnafu)?;
    let mut packs = BTreeMap::from([(pin.version.clone(), files)]);
    deck.widgets.insert(ns, pin);
    let mut profiles: Vec<(Namespace, LayoutProfile)> = Vec::new();
    for name in wanted {
        let target = layout_pack(platform, &library, name, lookup)?;
        let manifest = PackManifest::from_files(&target).context(BadPackSnafu { pack: name.as_str() })?;
        // Un pack désigné deux fois ne compte qu'une fois.
        if deck.layouts.contains_key(&manifest.namespace) {
            continue;
        }
        let pin = PackPin { id: manifest.id, version: target.version(), requires: manifest.requires.clone() };
        // Le pack cible et ses besoins, pris dans la bibliothèque.
        let pinned = (manifest.namespace.clone(), pin, target);
        for (ns, pin, files) in closure_with_files(&library, &deck, pinned, &packs).context(LibrarySnafu)? {
            deck.pins_mut(files.kind()).insert(ns, pin.clone());
            packs.insert(pin.version, files);
        }
        profiles.extend(manifest.profiles.iter().map(|p| (manifest.namespace.clone(), p.clone())));
    }
    let (mut bundle, widgets) = sources_with_packs(platform, &library, base, deck, packs)?;
    let themes = ThemeLibrary::user(platform);
    if let Some(wanted) = theme {
        let mut deck = bundle.deck().clone();
        deck.theme = Some(embed_theme(&themes, wanted, &mut bundle)?);
        bundle.apply_silent(deck).context(SourcesSnafu)?;
    }
    adopt_default_theme(&themes, &mut bundle).context(LibrarySnafu)?;

    // Le thème par défaut adopté juste avant : son absence serait un bogue, dit comme un thème inconnu.
    let theme = bundle.deck().theme.clone().context(UnknownThemeSnafu { theme: DEFAULT_THEME.to_string() })?;
    let tokens = deck_tokens(&bundle, &theme)?;
    let target = RetrofitTarget {
        profiles: &profiles,
        registry: &widgets.registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let plan = retrofit::project(&input.source, &target)
        .and_then(|plan| plan.check_budget(&input.limits))
        .context(PresentationSnafu)?;
    Ok(Prepared { bundle, widgets, plan })
}

#[cfg(test)]
#[path = "retrofit_tests.rs"]
mod retrofit_tests;
