//! Le hi-fi : la présentation reproduite fidèlement (`oku_slides_import::hifi`).
//!
//! Elle vient sur un pack de layouts généré pour elle (`deck-layouts`, du deck
//! seul, qui requiert `base`), avec `base` et `essentiel` intégrés (les slides
//! de repli sont des `essentiel/blank`), et sur le thème intégré Neutre : ses
//! jetons générés vont dans `deck.tokens` (op `SetTokens` du plan).
//!
//! Le catalogue des polices réunit celles de l'app (celles du rendu) et, si
//! demandé, celles du système des seules familles que cherche la présentation
//! ([`super::system_fonts`]). Le pack généré a une identité tirée des octets
//! du `.pptx` (deux imports du même fichier donnent le même pack) ; il ne vit
//! que dans le deck : la bibliothèque n'est pas touchée.
//!
//! Le pack généré est sondé par le processus de rendu, comme tout pack
//! embarqué ; le registre sondé doit porter exactement les schémas que le
//! projecteur a déclarés et avec lesquels il a construit ses ops. Le
//! projecteur tient le pack dans les bornes de la sonde (`[render]` :
//! `max_widgets`, `max_widget_kib`) ; un écart sur un fichier qui les dépasse
//! ou que le budget (`probe_budget_ms`) a laissé non sondé est dit comme tel
//! (`ProbeLimit`), tout autre écart est un bogue du générateur
//! (`ProbeMismatch`), comme un fichier du pack sans schéma déclaré. Une sonde
//! tout entière échouée (processus de rendu) est dite comme telle
//! (`ProbeFailed`). Tous sont refusés avant toute écriture.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use snafu::{OptionExt, ResultExt};
use uuid::{Builder, Uuid};

use oku_slides_core::{
    Deck, Namespace, PackManifest, PackPin, WidgetFile, WidgetRegistry, WidgetSchema, now_created, qualify,
};
use oku_slides_import::fonts::{FontCatalog, FontOrigin};
use oku_slides_import::hifi;
use oku_slides_import::{HifiTarget, PackLimits};
use oku_slides_platform::Platform;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::Fonts;

use super::error::{
    BadPackSnafu, LibrarySnafu, PackMissingSnafu, PresentationSnafu, ProbeFailedSnafu, ProbeLimitSnafu,
    ProbeMismatchSnafu, Result,
};
use super::{Input, Prepared};
use crate::deck_pack::builtin_pin;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK};
use crate::theme::{NEUTRE_NAMESPACE, builtin_namespace};
use crate::{PackLibrary, sources_with_packs};

/// Ce que le hi-fi prend de la demande d'import.
pub(crate) struct HifiArgs<'a> {
    /// Les polices de l'app, celles du rendu.
    pub(crate) fonts: &'a Fonts,
    /// Le registre intégré : les packs `base` et `essentiel` intégrés.
    pub(crate) base: &'a Arc<WidgetRegistry>,
    /// Le nom du pack généré.
    pub(crate) name: String,
    /// Chercher aussi les polices de la présentation parmi celles du système.
    pub(crate) system_fonts: bool,
}

/// Le hi-fi de `input` : les sources du deck, avec le pack et les polices
/// générés pour lui, et le pack sondé ; le plan pose Neutre et les jetons générés.
pub(crate) fn prepare(platform: &Platform, args: HifiArgs<'_>, input: &Input) -> Result<Prepared> {
    let mut catalog = FontCatalog::new();
    for data in args.fonts.shared_files() {
        catalog.add(FontOrigin::App, data);
    }
    if args.system_fonts {
        super::system_fonts::add(&mut catalog, &hifi::sought_fonts(&input.source));
    }
    let pack_id = identity(&input.bytes);
    let theme = builtin_namespace(NEUTRE_NAMESPACE).context(LibrarySnafu)?;
    // Le `base` intégré, jamais la version de l'utilisateur : le registre de la cible est le sien.
    let (base_ns, base_pin, base_files) = builtin_pin(BASE_PACK).context(LibrarySnafu)?;
    let (essentiel_ns, essentiel_pin, essentiel_files) = builtin_pin(ESSENTIEL_PACK).context(LibrarySnafu)?;
    let target = HifiTarget {
        base: base_pin.clone(),
        essentiel: essentiel_pin.clone(),
        registry: args.base,
        fonts: &catalog,
        pack_id,
        theme,
        name: args.name,
        created: now_created(),
        limits: pack_limits(&platform.settings.render),
    };
    let mut plan =
        hifi::project(&input.source, &target).and_then(|p| p.check_budget(&input.limits)).context(PresentationSnafu)?;
    // Sans pack, le projecteur n'a pu sérialiser son manifeste : un bogue, dit pour lui.
    let pack = plan.take_pack().context(PackMissingSnafu)?;

    let version = pack.version();
    let sizes: BTreeMap<WidgetFile, usize> = pack.widgets().iter().map(|(n, s)| (n.clone(), s.len())).collect();
    // Un manifeste généré illisible est un bogue du générateur, dit comme un pack illisible.
    let requires = PackManifest::from_files(&pack).context(BadPackSnafu { pack: target.name.as_str() })?.requires;
    let mut deck = Deck::empty();
    // `base` et `essentiel` d'abord, épinglés avant la sonde ; les ops `AddPack` du plan les retrouvent tels quels.
    deck.widgets.insert(base_ns, base_pin.clone());
    deck.layouts.insert(essentiel_ns, essentiel_pin.clone());
    // Le pack généré est le pack de layouts propre du deck : épinglé sous `deck-layouts`, il requiert `base`.
    deck.layouts.insert(Namespace::deck_layouts(), PackPin { id: pack_id, version: version.clone(), requires });
    let packs =
        BTreeMap::from([(base_pin.version, base_files), (essentiel_pin.version, essentiel_files), (version, pack)]);
    let library = PackLibrary::user(platform);
    let (mut bundle, widgets) = sources_with_packs(platform, &library, args.base, deck, packs)?;
    // Une sonde tout entière échouée (processus de rendu) n'est pas un écart du générateur.
    if let Some(error) = &widgets.probe_failed {
        return Err(ProbeFailedSnafu { error: error.as_str() }.build().into());
    }
    let files: Vec<WidgetFile> = sizes.keys().cloned().collect();
    if let Err(widget) = check_probe(&widgets.registry, &plan.schemas, &files) {
        let none = BTreeSet::new();
        let unprobed = widgets.unprobed.get(&Namespace::deck_layouts()).unwrap_or(&none);
        let refused = if beyond_limits(&widget, &sizes, unprobed, target.limits) {
            ProbeLimitSnafu { widget }.build()
        } else {
            ProbeMismatchSnafu { widget }.build()
        };
        return Err(refused.into());
    }
    bundle.fonts.extend(plan.take_fonts());
    Ok(Prepared { bundle, widgets, plan })
}

/// Id du pack généré pour un `.pptx`.
///
/// C'est la première moitié du SHA-256 de ses octets, en UUID v8 (version et variante posées).
pub(crate) fn identity(bytes: &[u8]) -> Uuid {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    let mut b = [0u8; 16];
    b.copy_from_slice(&digest[..16]);
    Builder::from_custom_bytes(b).into_uuid()
}

/// Les bornes de la sonde d'un pack, lues dans `[render]`.
fn pack_limits(render: &RenderSettings) -> PackLimits {
    PackLimits { max_files: render.max_widgets, max_file_bytes: render.max_widget_bytes() }
}

/// Le widget `widget` est-il hors des bornes de la sonde ?
///
/// `sizes` : les tailles des fichiers du pack (ordre des noms, `_commun` en
/// tête). Hors bornes : au-delà du nombre de fichiers chargés, plus gros que
/// la taille chargée (lui, ou `_commun` dont il dépend), ou laissé non sondé
/// (`unprobed`).
fn beyond_limits(
    widget: &str,
    sizes: &BTreeMap<WidgetFile, usize>,
    unprobed: &BTreeSet<WidgetFile>,
    limits: PackLimits,
) -> bool {
    let Some(name) = WidgetFile::parse(widget) else { return false };
    let rank = sizes.keys().position(|n| *n == name);
    let large = |n: &WidgetFile| sizes.get(n).is_some_and(|len| *len > limits.max_file_bytes);
    unprobed.contains(&name)
        || rank.is_some_and(|r| r >= limits.max_files)
        || large(&name)
        || large(&WidgetFile::common())
}

/// Le pack se sonde-t-il comme déclaré ?
///
/// Chaque schéma déclaré est-il celui que la sonde a lu, et chaque fichier du pack est-il déclaré ?
///
/// `files` : les fichiers de widget du pack, `_commun` mis à part. Sinon, le
/// nom du premier widget en écart (absent, différent ou non déclaré). Le
/// registre sondé porte aussi la base : ses autres widgets ne sont pas du
/// pack, d'où le contrôle des fichiers plutôt que du registre entier. Les
/// schémas déclarés portent leur nom nu ; la sonde, leur nom qualifié sous `deck-layouts`.
fn check_probe(probed: &WidgetRegistry, declared: &[WidgetSchema], files: &[WidgetFile]) -> Result<(), String> {
    let deck = Namespace::deck_layouts();
    let same = |s: &WidgetSchema| {
        let qualified = WidgetSchema { name: qualify(&deck, &s.name), ..s.clone() };
        probed.get(&qualified.name) == Some(&qualified)
    };
    if let Some(s) = declared.iter().find(|s| !same(s)) {
        return Err(s.name.clone());
    }
    match files.iter().filter(|f| !f.is_common()).find(|f| !declared.iter().any(|s| s.name == f.as_str())) {
        Some(f) => Err(f.as_str().to_owned()),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "hifi_tests.rs"]
mod hifi_tests;
