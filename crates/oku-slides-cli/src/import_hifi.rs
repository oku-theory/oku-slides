//! `oku-slides import --mode hifi` : la présentation reproduite fidèlement
//! (`oku_slides_import::hifi`), sur un thème et un pack générés pour elle.
//!
//! Le catalogue des polices réunit celles de l'app (celles du rendu) et, sauf
//! `--no-system-fonts`, celles du système des seules familles que cherche la
//! présentation ([`crate::system_fonts`]). Le pack et le thème générés ont
//! une identité tirée des octets du `.pptx` (deux imports du même fichier
//! donnent le même pack) ; ils ne vivent que dans le deck : la bibliothèque
//! n'est pas touchée.
//!
//! Le pack généré est sondé par le processus de rendu, comme tout pack
//! embarqué ; le registre sondé doit porter exactement les schémas que le
//! projecteur a déclarés et avec lesquels il a construit ses ops. Le
//! projecteur tient le pack dans les bornes de la sonde (`[render]` :
//! `max_widgets`, `max_widget_kib`) ; un écart sur un fichier qui les
//! dépasse ou que le budget (`probe_budget_ms`) a laissé non sondé est dit
//! comme tel (`cli-import-probe-limit`), tout autre écart est un bogue du
//! générateur (`cli-import-probe-mismatch`). Les deux sont refusés avant
//! toute écriture.

use std::collections::{BTreeMap, BTreeSet};

use color_eyre::eyre::{Result, eyre};
use sha2::{Digest, Sha256};
use uuid::{Builder, Uuid};

use oku_slides_core::{Deck, PackRef, WidgetFile, WidgetRegistry, WidgetSchema, now_created};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::fonts::{FontCatalog, FontOrigin};
use oku_slides_import::hifi;
use oku_slides_import::{HifiTarget, PackLimits};
use oku_slides_platform::Platform;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::Fonts;

use crate::import::{ImportArgs, Input, Prepared};
use crate::library::pack_library;

/// Le hi-fi de `input` : les sources du deck, avec le pack, le thème et les
/// polices générés pour lui, et le pack sondé.
pub(crate) fn prepare(
    platform: &Platform,
    l: &Localizer,
    fonts: &Fonts,
    args: &ImportArgs,
    input: &Input,
) -> Result<Prepared> {
    let mut catalog = FontCatalog::new();
    for data in fonts.shared_files() {
        catalog.add(FontOrigin::App, data);
    }
    if args.system_fonts {
        crate::system_fonts::add(&mut catalog, &hifi::sought_fonts(&input.source));
    }
    let (pack_id, theme_id) = identity(&input.bytes);
    let base = crate::registry(fonts)?;
    let target = HifiTarget {
        registry: &base,
        fonts: &catalog,
        pack_id,
        theme_id,
        name: args.input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        created: now_created(),
        limits: pack_limits(&platform.settings.render),
    };
    let mut plan =
        hifi::project(&input.source, &target).check_budget(&input.limits).map_err(|e| eyre!(e.localize(l)))?;
    // Sans pack ni thème, le projecteur n'a pu sérialiser leurs manifestes : un bogue, comme un écart de sonde.
    let mismatch = |widget: &str| eyre!(l.tr_with("cli-import-probe-mismatch", &[("widget", widget.into())]));
    let pack = plan.take_pack().ok_or_else(|| mismatch("pack.json"))?;
    let theme = plan.take_theme().ok_or_else(|| mismatch("theme.json"))?;

    let version = pack.version();
    let sizes: BTreeMap<WidgetFile, usize> = pack.widgets().iter().map(|(n, s)| (n.clone(), s.len())).collect();
    let mut deck = Deck::empty();
    deck.templates = Some(PackRef { id: pack_id, version: version.clone() });
    let packs = BTreeMap::from([(version, pack)]);
    let (mut bundle, widgets) = crate::sources_with_pack(platform, l, &pack_library(platform), &base, deck, packs)?;
    check_probe(&widgets.registry, &plan.schemas).map_err(|w| {
        if beyond_limits(&w, &sizes, &widgets.unprobed, target.limits) {
            eyre!(l.tr_with("cli-import-probe-limit", &[("widget", w.into())]))
        } else {
            mismatch(&w)
        }
    })?;
    bundle.add_theme(theme);
    bundle.fonts.extend(plan.take_fonts());
    Ok(Prepared { bundle, widgets, plan })
}

/// Ids du pack et du thème générés pour un `.pptx` : les deux moitiés du
/// SHA-256 de ses octets, en UUID v8 (version et variante posées).
fn identity(bytes: &[u8]) -> (Uuid, Uuid) {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    let half = |at: usize| {
        let mut b = [0u8; 16];
        b.copy_from_slice(&digest[at..at + 16]);
        Builder::from_custom_bytes(b).into_uuid()
    };
    (half(0), half(16))
}

/// Les bornes de la sonde d'un pack, lues dans `[render]`.
fn pack_limits(render: &RenderSettings) -> PackLimits {
    PackLimits { max_files: render.max_widgets, max_file_bytes: render.max_widget_bytes() }
}

/// Le widget `widget` d'un pack aux fichiers de tailles `sizes` (ordre des
/// noms, `_commun` en tête) est-il hors des bornes de la sonde : au-delà du
/// nombre de fichiers chargés, plus gros que la taille chargée (lui, ou
/// `_commun` dont il dépend), ou laissé non sondé (`unprobed`) ?
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

/// Chaque schéma déclaré est-il celui que la sonde a lu ? Sinon, le nom du
/// premier widget en écart (absent ou différent).
fn check_probe(probed: &WidgetRegistry, declared: &[WidgetSchema]) -> Result<(), String> {
    match declared.iter().find(|s| probed.get(&s.name) != Some(*s)) {
        Some(s) => Err(s.name.clone()),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "import_hifi_tests.rs"]
mod import_hifi_tests;
