//! `oku-slides import` : une présentation PowerPoint en deck Oku.
//!
//! Mode `retrofit` (lot 2, le défaut) : le contenu de la présentation est
//! projeté sur un pack et un thème existants (`oku_slides_import::retrofit`).
//! Le pack est obligatoire (id, dossier ou nom) ; le thème est facultatif (id
//! ou nom ; défaut : le thème par défaut).
//!
//! Mode `hifi` (lot 3, [`crate::import_hifi`]) : la présentation est reproduite
//! fidèlement sur un pack et un thème générés pour elle, embarqués dans le
//! deck seulement ; `--pack` et `--theme` y sont refusés.
//!
//! Dans les deux modes, la lecture et la taille du plan sont bornées
//! ([`ReadLimits`] ; `max_output_bytes` et `max_replica_bytes` viennent de
//! `[sources] max_sources_mib` et `max_replica_mib`, bornes de réouverture d'un
//! deck : une estimation précoce du contenu produit, pas une garantie), puis les
//! ops du plan s'appliquent en une transaction et le deck est rendu et enregistré
//! en `.oku-slides.pdf`, comme `save`, avec son rapport s'il est demandé. Le
//! rendu relève les champs qui sortent de leur page, et en retrofit les boîtes
//! sous la marge basse (perte `Overflow`,
//! [`crate::import_overflow`]) ; en hi-fi, sauf `--reference none`, il est
//! comparé au rendu de référence du témoin ([`crate::import_reference`] ;
//! `--reference office2pdf` ou `auto`, le défaut, `soffice` pour LibreOffice) :
//! un écart par slide au rapport, une ligne de plus en sortie.

use std::collections::BTreeMap;
use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr, eyre};
use serde_json::{Map, Value};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, PackManifest, PackRef, SourceBundle, apply_all};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::report::ReferenceReport;
use oku_slides_import::retrofit::{self, ColorTokens, PageTokens};
use oku_slides_import::source::SourceDeck;
use oku_slides_import::{CheckedPlan, ImportReport, ReadLimits, RetrofitTarget, read_pptx};
use oku_slides_platform::Platform;
use oku_slides_platform::storage::StorageErrorKind;
use oku_slides_render::Fonts;
use oku_slides_render_worker::{ReferenceWitness, RenderWorker};
use oku_slides_session::theme_schema::default_tokens;
use oku_slides_session::{ThemeLibrary, adopt_default_theme, theme_tokens};

use crate::import_overflow::note_overflows;
use crate::import_reference::{self, Scoring};
use crate::library::{embed_theme, pack_library, resolve_pack, theme_library};
use crate::out::Out;
use crate::pack_render::{self, DeckWidgets};

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ImportMode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
    /// Copie fidèle, sur un pack et un thème générés.
    Hifi,
}

/// Rendu de référence d'un import hi-fi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ReferenceMode {
    /// Le témoin par défaut (`office2pdf`) ; l'écart de chaque slide va au rapport.
    Auto,
    /// Le témoin `office2pdf`.
    Office2pdf,
    /// Le témoin LibreOffice (`soffice`), cherché parmi `[import] soffice`.
    Soffice,
    /// Pas de rendu de référence.
    None,
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
    /// Hi-fi : chercher les polices de la présentation parmi celles du système.
    pub(crate) system_fonts: bool,
    /// Hi-fi : rendu de référence (`None` : le défaut, `auto`) ; refusé en retrofit.
    pub(crate) reference: Option<ReferenceMode>,
}

/// La présentation lue : ses octets (l'identité du hi-fi en dérive), sa source
/// et les bornes de l'import, dont le budget de sortie que le plan doit tenir.
pub(crate) struct Input {
    pub(crate) bytes: Vec<u8>,
    pub(crate) source: SourceDeck,
    pub(crate) limits: ReadLimits,
}

/// Un import prêt à s'appliquer : les sources (packs et thèmes embarqués),
/// les widgets sondés de leur pack et le plan, dont la sortie tient dans les budgets.
pub(crate) struct Prepared {
    pub(crate) bundle: SourceBundle,
    pub(crate) widgets: DeckWidgets,
    pub(crate) plan: CheckedPlan,
}

/// Les jetons effectifs du thème de `bundle` : les défauts, puis ceux du thème.
///
/// Un thème illisible est une erreur : ses couleurs et sa page guident le retrofit,
/// les défauts seuls donneraient un deck faux sans le dire.
fn deck_tokens(themes: &ThemeLibrary, bundle: &SourceBundle, l: &Localizer) -> Result<Map<String, Value>> {
    let own = theme_tokens(themes, &bundle.themes, bundle.deck()).ok_or_else(|| eyre!(l.tr("cli-import-bad-theme")))?;
    Ok(effective_tokens(default_tokens(), &own))
}

/// Importe `args.input` et écrit le deck obtenu dans `args.output`.
pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: ImportArgs) -> Result<()> {
    let l = out.localizer();
    // Les arguments d'abord : une erreur de forme ne lit pas la présentation.
    let pack = match args.mode {
        ImportMode::Hifi if args.pack.is_some() || args.theme.is_some() => {
            return Err(eyre!(l.tr("cli-import-hifi-no-target")));
        }
        ImportMode::Hifi => {
            let threshold = platform.settings.import.score_threshold;
            if !(threshold.is_finite() && threshold >= 0.0) {
                return Err(eyre!(l.tr_with("cli-import-bad-threshold", &[("value", threshold.to_string().into())])));
            }
            None
        }
        ImportMode::Retrofit if args.reference.is_some() => {
            return Err(eyre!(l.tr("cli-import-reference-hifi-only")));
        }
        ImportMode::Retrofit => Some(args.pack.as_deref().ok_or_else(|| eyre!(l.tr("cli-import-pack-required")))?),
    };
    let input = read_input(platform, l, &args)?;
    let prepared = match pack {
        Some(pack) => prepare_retrofit(platform, l, fonts, &args, pack, &input)?,
        None => crate::import_hifi::prepare(platform, l, fonts, &args, &input)?,
    };
    finish(platform, out, fonts, &args, &input, prepared)
}

/// `n` octets en `u64`.
fn bytes(n: usize) -> u64 {
    // Saturé, jamais atteint : `usize` tient dans `u64` sur toutes les cibles (64 bits au plus),
    // et les réglages en Mio sont déjà saturés en `usize`.
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Lit `args.input`, borné par [`ReadLimits::default`] ; la sortie, approximativement,
/// par la taille des sources et de l'historique qu'un deck peut avoir pour se rouvrir.
fn read_input(platform: &Platform, l: &Localizer, args: &ImportArgs) -> Result<Input> {
    let shown = args.input.display().to_string();
    let sources = &platform.settings.sources;
    let limits = ReadLimits {
        max_output_bytes: bytes(sources.max_sources_bytes()),
        max_replica_bytes: bytes(sources.max_replica_bytes()),
        ..ReadLimits::default()
    };
    // Taille vue avant toute lecture : un fichier trop gros n'entre jamais en mémoire.
    let bytes = platform.storage.read_limited(&args.input, limits.max_input_bytes).map_err(|e| {
        if e.kind() == StorageErrorKind::TooLarge {
            eyre!(l.tr("import-too-large"))
        } else {
            eyre!(e).wrap_err(l.tr_with("cli-reading", &[("path", shown.as_str().into())]))
        }
    })?;
    let source = read_on_own_thread(&bytes, &limits)
        .wrap_err_with(|| l.tr_with("cli-reading", &[("path", shown.as_str().into())]))?
        .map_err(|e| eyre!(e.localize(l)))?;
    Ok(Input { bytes, source, limits })
}

/// Pile du fil de lecture : la lecture est bornée en profondeur (XML, groupes), et cette pile
/// ne dépend ni du fil appelant ni d'un runtime.
const READ_STACK_BYTES: usize = 16 << 20;

/// [`read_pptx`] sur un fil dédié à pile connue ([`READ_STACK_BYTES`]) ; l'erreur externe est
/// celle du lancement du fil, une panique de la lecture est relancée telle quelle.
pub(crate) fn read_on_own_thread(
    bytes: &[u8],
    limits: &ReadLimits,
) -> std::io::Result<Result<SourceDeck, oku_slides_import::ImportError>> {
    std::thread::scope(|scope| {
        let reader = std::thread::Builder::new()
            .name("import-read".to_owned())
            .stack_size(READ_STACK_BYTES)
            .spawn_scoped(scope, || read_pptx(bytes, limits))?;
        Ok(reader.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)))
    })
}

/// Le retrofit de `source` sur le pack `wanted` et le thème `args.theme`.
fn prepare_retrofit(
    platform: &Platform,
    l: &Localizer,
    fonts: &Fonts,
    args: &ImportArgs,
    wanted: &str,
    input: &Input,
) -> Result<Prepared> {
    let library = pack_library(platform);
    let files = resolve_pack(&platform.storage, &library, l, wanted)?;
    let manifest = PackManifest::from_files(&files)
        .map_err(|_| eyre!(l.tr_with("cli-import-bad-pack", &[("pack", wanted.into())])))?;
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

    let tokens = deck_tokens(&themes, &bundle, l)?;
    let target = RetrofitTarget {
        templates: &manifest.templates,
        registry: &widgets.registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let plan =
        retrofit::project(&input.source, &target).check_budget(&input.limits).map_err(|e| eyre!(e.localize(l)))?;
    Ok(Prepared { bundle, widgets, plan })
}

/// Applique le plan de `prepared` en une transaction, rend et écrit le deck
/// (avec ses polices embarquées), relève débordements et, en hi-fi, l'écart à
/// la référence, puis écrit le rapport ; une ligne de résultat, plus celle de
/// l'écart en hi-fi.
fn finish(
    platform: &Platform,
    out: &mut Out<'_>,
    fonts: &Fonts,
    args: &ImportArgs,
    input: &Input,
    prepared: Prepared,
) -> Result<()> {
    let l = out.localizer();
    let Prepared { mut bundle, widgets, mut plan } = prepared;
    for asset in plan.assets.values() {
        bundle.add_asset(asset.bytes.clone());
    }
    let mut deck = bundle.deck().clone();
    apply_all(&mut deck, plan.take_ops(), &widgets.registry).map_err(|e| eyre!(e.localize(l)))?;
    bundle.apply(deck, &l.tr("cli-import-label"), None).map_err(|e| eyre!(e.localize(l)))?;
    crate::report_issues(l, bundle.deck(), &widgets.registry, &bundle.assets);
    let target = args.output.display().to_string();
    let fonts = fonts.with(bundle.fonts.values().cloned());
    let failed = || l.tr_with("cli-import-write-failed", &[("path", target.as_str().into())]);
    let doc =
        pack_render::render(platform, l, fonts.clone(), &widgets, &bundle, bundle.deck()).wrap_err_with(failed)?;
    let written = crate::write_rendered(platform, &doc, &bundle, &args.output).wrap_err_with(failed)?;
    note_overflows(doc.layout(), plan.report_mut());
    if args.mode == ImportMode::Hifi {
        let threshold = platform.settings.import.score_threshold;
        let witness = match args.reference.unwrap_or(ReferenceMode::Auto) {
            ReferenceMode::None => None,
            ReferenceMode::Auto | ReferenceMode::Office2pdf => Some(ReferenceWitness::Office2Pdf),
            ReferenceMode::Soffice => {
                let found = import_reference::soffice(platform);
                if found.is_none() {
                    import_reference::missing(plan.report_mut(), threshold, l);
                }
                found
            }
        };
        if let Some(witness) = witness {
            // Un processus de rendu à lui : le témoin n'ajoute pas sa mémoire à celle du rendu Oku.
            let settings = import_reference::worker_settings(platform, &witness);
            let worker = RenderWorker::new(platform.render_launch.clone(), settings);
            let (pptx, source) = (&input.bytes, &input.source);
            let scoring = Scoring { pptx, source, fonts: &fonts, witness: &witness, worker: &worker, threshold, l };
            import_reference::score(&scoring, &doc, plan.report_mut());
        }
    }
    if let Some(path) = &args.report {
        // Écrit puis renommé : un rapport interrompu ne remplace jamais le précédent.
        platform.storage.write_atomic(path, serde_json::to_string_pretty(&plan.report_json(l))?)?;
    }
    out.line(
        "cli-imported",
        &[
            ("path", target.into()),
            ("slides", bundle.deck().slides.len().into()),
            ("pages", written.pages.into()),
            ("losses", plan.report.loss_count().into()),
            ("requests", plan.widget_requests.len().into()),
        ],
    )?;
    if let Some(reference) = &plan.report.reference {
        reference_line(out, reference, &plan.report)?;
    }
    Ok(())
}

/// La ligne de l'écart visuel : moyenne et slides au-delà du seuil (par leur pire
/// région), ou pourquoi la référence manque.
fn reference_line(out: &mut Out<'_>, reference: &ReferenceReport, report: &ImportReport) -> Result<()> {
    let l = out.localizer();
    match (&reference.unavailable, report.mean_score()) {
        (Some(cause), _) => out.line("cli-import-unscored", &[("reason", cause.localize(l).into())])?,
        (None, Some(mean)) => out.line(
            "cli-import-scored",
            &[
                ("witness", reference.witness.as_str().into()),
                ("mean", mean.into()),
                ("above", report.above_threshold().count().into()),
                ("threshold", reference.threshold.into()),
            ],
        )?,
        // Une présentation sans slide : rien à dire.
        (None, None) => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod import_tests;
