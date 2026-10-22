//! `oku-slides import` : une présentation PowerPoint en deck Oku.
//!
//! Par l'import de la session (`oku_slides_session::import`), le même que l'app.
//!
//! Mode `retrofit` (le défaut) : le contenu est projeté sur des packs de layouts et un
//! thème existants ; `--pack`, répétable, nomme chaque pack de layouts (espace de noms,
//! id, dossier ou nom ; aucun : `essentiel`), `--theme` facultatif (id ou nom ; défaut :
//! le thème par défaut). Mode `hifi` : la
//! présentation est reproduite fidèlement sur un pack et un thème générés pour
//! elle, embarqués dans le deck seulement ; `--pack` et `--theme` y sont refusés.
//!
//! L'import appliqué en une transaction, le deck est rendu par le processus
//! de rendu et enregistré en `.oku-slides.pdf`, comme `save`, avec son rapport
//! s'il est demandé. Le rendu relève les champs qui sortent de leur page, et
//! en retrofit les boîtes sous la marge basse (perte `Overflow`) ; en hi-fi,
//! sauf `--reference none`, il est aussitôt comparé au rendu de référence du
//! témoin (`--reference auto`, le défaut : celui de `[import] witness` ;
//! `office2pdf` ou `soffice` pour LibreOffice l'emportent sur le réglage) : un
//! écart par slide au rapport, une ligne de plus en sortie.

use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr, eyre};

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::ImportReport;
use oku_slides_import::report::ReferenceReport;
use oku_slides_platform::Platform;
use oku_slides_platform::config::ImportWitness;
use oku_slides_render::Fonts;
use oku_slides_session::import::{
    self as session_import, ImportMode as SessionMode, ImportRequest, ImportTarget, Imported, PackLookup,
    note_overflows,
};

use crate::out::Out;

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ImportMode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
    /// Copie fidèle, sur un pack et un thème générés.
    Hifi,
}

impl From<ImportMode> for SessionMode {
    fn from(mode: ImportMode) -> Self {
        match mode {
            ImportMode::Retrofit => Self::Retrofit,
            ImportMode::Hifi => Self::Hifi,
        }
    }
}

/// Rendu de référence d'un import hi-fi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ReferenceMode {
    /// Le témoin de `[import] witness` (`office2pdf` par défaut).
    ///
    /// L'écart de chaque slide va au rapport.
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
    /// Les packs de layouts cibles du retrofit, dans l'ordre : id, dossier, nom ou espace de noms.
    ///
    /// Vide : `essentiel`.
    pub(crate) packs: Vec<String>,
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

/// Importe `args.input` et écrit le deck obtenu dans `args.output`.
pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: ImportArgs) -> Result<()> {
    let l = out.localizer();
    // Les arguments d'abord : une erreur de forme ne lit pas la présentation.
    // Le pack cible peut être un dossier, ici seulement : l'app ne prend que la bibliothèque.
    let request = ImportRequest {
        pptx: args.input.clone(),
        target: target(platform, l, &args)?,
        system_fonts: args.system_fonts,
        packs: PackLookup::LibraryOrDir,
    };
    let imported = session_import::import(platform, fonts, &request).map_err(|e| eyre!(e.localize(l)))?;
    finish(platform, out, &args, imported)
}

/// Le témoin de `--reference` ; sans l'option ou `auto`, celui de `[import] witness` (t-905).
///
/// L'option explicite l'emporte sur le réglage ; `none` : pas de référence.
fn witness(platform: &Platform, reference: Option<ReferenceMode>) -> Option<ImportWitness> {
    match reference.unwrap_or(ReferenceMode::Auto) {
        ReferenceMode::Auto => Some(platform.settings.import.witness),
        ReferenceMode::Office2pdf => Some(ImportWitness::Office2pdf),
        ReferenceMode::Soffice => Some(ImportWitness::Soffice),
        ReferenceMode::None => None,
    }
}

/// La cible de l'import selon les options ; une option d'un autre mode est refusée.
fn target(platform: &Platform, l: &Localizer, args: &ImportArgs) -> Result<ImportTarget> {
    match SessionMode::from(args.mode) {
        SessionMode::Hifi if !args.packs.is_empty() || args.theme.is_some() => {
            Err(eyre!(l.tr("cli-import-hifi-no-target")))
        }
        SessionMode::Hifi => Ok(ImportTarget::Hifi { witness: witness(platform, args.reference) }),
        SessionMode::Retrofit if args.reference.is_some() => Err(eyre!(l.tr("cli-import-reference-hifi-only"))),
        SessionMode::Retrofit => Ok(ImportTarget::Retrofit { layouts: args.packs.clone(), theme: args.theme.clone() }),
    }
}

/// Rend et écrit le deck importé, puis son rapport.
///
/// Le deck part avec ses polices embarquées ; le rapport relève débordements et, en hi-fi, l'écart
/// à la référence. Une ligne de résultat, plus celle de l'écart en hi-fi.
fn finish(platform: &Platform, out: &mut Out<'_>, args: &ImportArgs, imported: Imported) -> Result<()> {
    let l = out.localizer();
    let Imported { sources: bundle, widgets, issues, mut state } = imported;
    crate::log_issues(l, &issues);
    let target = args.output.display().to_string();
    let failed = || l.tr_with("cli-import-write-failed", &[("path", target.as_str().into())]);
    let doc = session_import::render(platform, state.fonts(), &widgets, &bundle).wrap_err_with(failed)?;
    let compiled = doc.compiled();
    let pdf = compiled.pdf().wrap_err_with(failed)?;
    let written =
        crate::write_rendered(platform, &pdf, compiled.page_count(), &bundle, &args.output).wrap_err_with(failed)?;
    note_overflows(compiled.layout(), state.report_mut());
    if SessionMode::from(args.mode) == SessionMode::Hifi {
        // Le témoin, choisi à l'import (`witness`) ; le même chemin que l'app : le travail de référence, puis la note de chaque slide.
        if let Some(job) = state.reference_job() {
            job.run(platform);
        }
        state.score_deck(compiled);
    }
    if let Some(path) = &args.report {
        // Écrit puis renommé : un rapport interrompu ne remplace jamais le précédent.
        platform.storage.write_atomic(path, serde_json::to_string_pretty(&state.report_json(l))?)?;
    }
    let report = state.report();
    out.line(
        "cli-imported",
        &[
            ("path", target.into()),
            ("slides", bundle.deck().slides.len().into()),
            ("pages", written.pages.into()),
            ("losses", report.loss_count().into()),
            ("requests", state.widget_requests().len().into()),
        ],
    )?;
    if let Some(reference) = &report.reference {
        reference_line(out, reference, report)?;
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
