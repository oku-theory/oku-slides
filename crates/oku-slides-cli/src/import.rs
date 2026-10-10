//! `oku-slides import` : une présentation PowerPoint en deck Oku.
//!
//! Mode `retrofit` (lot 2) : le contenu de la présentation est projeté sur
//! un pack et un thème existants (`oku_slides_import::retrofit`). Le pack est
//! obligatoire (id, dossier ou nom) ; le thème est facultatif (id ou nom ;
//! défaut : le thème par défaut). Les ops du plan s'appliquent en une
//! transaction, puis le deck est rendu et enregistré en `.oku-slides.pdf`,
//! comme `save`. Le mode `hifi` est reconnu et refusé : il arrive au lot 3.

use std::collections::BTreeMap;
use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr, eyre};
use serde_json::{Map, Value};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, PackManifest, PackRef, SourceBundle, apply_all};
use oku_slides_i18n::Localize;
use oku_slides_import::retrofit::{self, ColorTokens, PageTokens};
use oku_slides_import::{ReadLimits, RetrofitTarget, read_pptx};
use oku_slides_platform::Platform;
use oku_slides_platform::storage::StorageErrorKind;
use oku_slides_render::Fonts;
use oku_slides_session::theme_schema::default_tokens;
use oku_slides_session::{ThemeLibrary, adopt_default_theme, theme_tokens};

use crate::library::{embed_theme, pack_library, resolve_pack, theme_library};
use crate::out::Out;

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ImportMode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
    /// Copie fidèle (lot 3) : refusée pour l'instant.
    Hifi,
}

/// Les arguments de `import`.
#[derive(Debug)]
pub(crate) struct ImportArgs {
    /// Le `.pptx` à importer.
    pub(crate) input: PathBuf,
    /// Le mode.
    pub(crate) mode: ImportMode,
    /// Le pack cible : id, dossier ou nom.
    pub(crate) pack: Option<String>,
    /// Le thème cible : id ou nom.
    pub(crate) theme: Option<String>,
    /// Le `.oku-slides.pdf` à écrire.
    pub(crate) output: PathBuf,
    /// Le rapport JSON à écrire.
    pub(crate) report: Option<PathBuf>,
}

/// Les jetons effectifs du thème de `bundle` : les défauts, puis ceux du thème.
fn deck_tokens(themes: &ThemeLibrary, bundle: &SourceBundle) -> Map<String, Value> {
    let own = theme_tokens(themes, &bundle.themes, bundle.deck()).unwrap_or_default();
    effective_tokens(default_tokens(), &own)
}

/// Importe `args.input` et écrit le deck obtenu dans `args.output`.
pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: ImportArgs) -> Result<()> {
    let (storage, l) = (&platform.storage, out.localizer());
    if args.mode == ImportMode::Hifi {
        return Err(eyre!(l.tr("cli-import-hifi-later")));
    }
    let wanted = args.pack.as_deref().ok_or_else(|| eyre!(l.tr("cli-import-pack-required")))?;
    let library = pack_library(platform);
    let files = resolve_pack(storage, &library, l, wanted)?;
    let manifest = PackManifest::from_files(&files)
        .map_err(|_| eyre!(l.tr_with("cli-import-bad-pack", &[("pack", wanted.into())])))?;
    let shown = args.input.display().to_string();
    let limits = ReadLimits::default();
    // Taille vue avant toute lecture : un fichier trop gros n'entre jamais en mémoire.
    let bytes = storage.read_limited(&args.input, limits.max_input_bytes).map_err(|e| {
        if e.kind() == StorageErrorKind::TooLarge {
            eyre!(l.tr("import-too-large"))
        } else {
            eyre!(e).wrap_err(l.tr_with("cli-reading", &[("path", shown.as_str().into())]))
        }
    })?;
    let source = read_pptx(&bytes, &limits).map_err(|e| eyre!(e.localize(l)))?;

    let version = files.version();
    let mut deck = Deck::empty();
    deck.templates = Some(PackRef { id: manifest.id, version: version.clone() });
    let base = crate::registry(fonts)?;
    let packs = BTreeMap::from([(version, files)]);
    let (mut bundle, widgets) = crate::sources_with_pack(platform, l, &library, &base, deck, packs)?;
    let themes = theme_library(platform);
    if let Some(wanted) = &args.theme {
        let mut deck = bundle.deck().clone();
        deck.theme = Some(embed_theme(&themes, l, wanted, &mut bundle)?);
        bundle.apply_silent(deck)?;
    }
    adopt_default_theme(&themes, &mut bundle).map_err(|e| eyre!(e.localize(l)))?;

    let tokens = deck_tokens(&themes, &bundle);
    let target = RetrofitTarget {
        templates: &manifest.templates,
        registry: &widgets.registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let plan = retrofit::project(&source, &target);
    for asset in plan.assets.values() {
        bundle.add_asset(asset.bytes.clone());
    }
    let mut deck = bundle.deck().clone();
    apply_all(&mut deck, plan.ops.clone(), &widgets.registry).map_err(|e| eyre!(e.localize(l)))?;
    bundle.apply(deck, &l.tr("cli-import-label"), None).map_err(|e| eyre!(e.localize(l)))?;
    crate::report_issues(l, bundle.deck(), &widgets.registry, &bundle.assets);
    let target = args.output.display().to_string();
    let written = crate::write_document(platform, l, fonts.clone(), &widgets, &bundle, &args.output)
        .wrap_err_with(|| l.tr_with("cli-import-write-failed", &[("path", target.as_str().into())]))?;
    if let Some(path) = &args.report {
        storage.write(path, serde_json::to_string_pretty(&plan.report_json(l))?)?;
    }
    out.line(
        "cli-imported",
        &[
            ("path", args.output.display().to_string().into()),
            ("slides", bundle.deck().slides.len().into()),
            ("pages", written.pages.into()),
            ("losses", plan.report.loss_count().into()),
            ("requests", plan.widget_requests.len().into()),
        ],
    )?;
    Ok(())
}
