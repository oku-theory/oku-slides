//! Le retrofit.
//!
//! Le contenu de la présentation projeté sur un pack et un thème existants
//! (`oku_slides_import::retrofit`). Le pack est désigné par son id ou son nom
//! ([`PackLookup::Library`], l'app), ou aussi son dossier ([`PackLookup::LibraryOrDir`], la CLI) ;
//! le thème par son id ou son nom, sinon le thème par défaut.
//!
//! Le deck prend la version cible du pack, telle quelle ; son pack enfant
//! ([`ChildPack`]) n'est créé qu'à la première écriture d'un widget, et ni le
//! pack cible ni la bibliothèque ne changent.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, PackManifest, PackPin, SourceBundle, ThemeRef, WidgetRegistry, kinds};
use oku_slides_import::RetrofitTarget;
use oku_slides_import::retrofit::{self, ColorTokens, PageTokens};
use oku_slides_platform::Platform;

use super::child_pack::ChildPack;
use super::error::{
    BadPackSnafu, BadThemeSnafu, LibrarySnafu, PresentationSnafu, Result, SourcesSnafu, UnknownThemeSnafu,
};
use super::{Input, PackLookup, Prepared};
use crate::deck_closure::closure_with_files;
use crate::deck_pack::builtin_pin;
use crate::deck_theme::manifest_of;
use crate::error::ThemeVersionMissingSnafu;
use crate::pack::BASE_PACK;
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

/// Le retrofit de `input` sur le pack `wanted` et le thème `theme`.
///
/// `input` est le `.pptx` nommé `name` ; `wanted` se cherche selon `lookup`.
pub(crate) fn prepare(
    platform: &Platform,
    base: &Arc<WidgetRegistry>,
    (wanted, theme, lookup): (&str, Option<&str>, PackLookup),
    name: &str,
    input: &Input,
) -> Result<Prepared> {
    let library = PackLibrary::user(platform);
    let target = match lookup {
        PackLookup::Library => library_pack(&library, wanted)?,
        PackLookup::LibraryOrDir => {
            resolve_pack(&platform.storage, &library, wanted, platform.settings.sources.max_sources_bytes())?
        }
    };
    let manifest = PackManifest::from_files(&target).context(BadPackSnafu { pack: wanted })?;
    let pin = PackPin { id: manifest.id, version: target.version(), requires: manifest.requires.clone() };
    let child = ChildPack::new(&platform.live_localizer.current(), &input.bytes, pin.clone(), &manifest.name, name);
    let mut deck = Deck::empty();
    let mut packs = BTreeMap::from([(pin.version.clone(), target.clone())]);
    // Le pack cible et ses besoins, pris dans la bibliothèque.
    let wanted = (manifest.namespace.clone(), pin, target);
    for (ns, pin, files) in closure_with_files(&library, &deck, wanted, &packs).context(LibrarySnafu)? {
        packs.insert(pin.version.clone(), files);
        deck.widgets.insert(ns, pin);
    }
    // Le retrofit écrit des éléments `base/*` : le `base` intégré si la fermeture de la cible ne l'a pas.
    if !deck.widgets.contains_key(&kinds::base::namespace()) {
        let (ns, pin, files) = builtin_pin(BASE_PACK).context(LibrarySnafu)?;
        packs.insert(pin.version.clone(), files);
        deck.widgets.insert(ns, pin);
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
        templates: &manifest.templates,
        registry: &widgets.registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let plan = retrofit::project(&input.source, &target).check_budget(&input.limits).context(PresentationSnafu)?;
    Ok(Prepared { bundle, widgets, plan, child: Some(child) })
}

#[cfg(test)]
#[path = "retrofit_tests.rs"]
mod retrofit_tests;
