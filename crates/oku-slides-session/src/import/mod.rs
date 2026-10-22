//! L'import d'une présentation PowerPoint en deck Oku : une seule implémentation.
//!
//! La CLI (`oku-slides import`) et l'app la partagent.
//!
//! [`import`] lit le `.pptx` (taille vue avant toute lecture, puis lu sur un
//! fil à pile connue, une panique du lecteur dite comme une erreur),
//! projette son contenu ([`ImportTarget`] : retrofit sur des packs de layouts
//! et un thème de la bibliothèque, dont les profils guident la projection ;
//! ou hi-fi sur un pack de layouts (`deck-layouts`) et un thème générés, du
//! deck seul ; la bibliothèque n'est jamais touchée),
//! sonde les packs du deck dans le processus de rendu, puis applique le plan aux
//! sources en une transaction : une seule étape d'historique, que l'annulation
//! défait d'un coup, validée par les règles ordinaires du cœur.
//! Le retrofit refuse un thème cible dont les jetons ne se lisent pas : ses
//! couleurs et sa page guident la projection.
//!
//! Le deck importé se rend par le processus de rendu ([`render`]) ; ses
//! champs qui sortent de leur page, et en retrofit ses boîtes sous la marge
//! basse, deviennent des pertes ([`note_overflows`]).
//! [`ImportState`] garde le rapport et, en hi-fi, l'écart de chaque slide au
//! rendu de référence du témoin choisi à l'import (`[import] witness` :
//! office2pdf ou LibreOffice ; la CLI, aussi par `--reference`), fait une fois hors du verrou de la session ([`ReferenceJob`],
//! [`WitnessRenderer`]) et gardé tant que vit l'onglet.
//! L'agent ajuste un import hi-fi slide par slide, en essais bornés par
//! slide et pour le deck entier ([`TuneLimits`], [`Tuner`]) : [`crate::Session::import_report`] donne la file,
//! [`crate::Session::import_diff`] compare une slide ou zoome sur une région.
//!
//! La lecture et la taille du plan sont bornées (`ReadLimits` ;
//! `max_output_bytes` et `max_replica_bytes` viennent de `[sources]
//! max_sources_mib` et `max_replica_mib`, bornes de réouverture d'un deck :
//! une estimation précoce du contenu produit, pas une garantie). Rien n'est
//! écrit : l'appelant enregistre le deck obtenu.

mod diff;
pub(crate) mod error;
mod geometry;
mod hifi;
mod input;
mod overflow;
mod queue;
mod reference;
mod render;
mod retrofit;
mod sample;
mod score;
mod state;
mod system_fonts;
mod tab;
mod tune;
mod tune_error;

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use oku_slides_core::{Issue, SourceBundle, apply_all, validate};
use oku_slides_import::CheckedPlan;
use oku_slides_platform::Platform;
use oku_slides_platform::config::ImportWitness;
use oku_slides_render::Fonts;

use crate::ProbedWidgets;

pub use diff::{ImportDiff, MARGIN, ZOOM};
pub use error::{ImportError, ImportErrorKind};
use error::{OpSnafu, RegistrySnafu, Result, SourcesSnafu, ThresholdSnafu};
use input::Input;
pub use overflow::note_overflows;
pub use queue::LISTED;
pub use reference::{REFERENCE_WIDTH, ReferenceJob, SPAN, WitnessPages, WitnessRenderer};
pub use render::{RenderedDeck, render};
pub use state::{ImportState, SlideRegion};
pub use tune::{Charge, Seen, SlideState, TuneLimits, Tuner, Tuning};
pub use tune_error::{TuneError, TuneErrorKind};

/// Le mode d'un import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    /// Contenu projeté sur des packs de layouts et un thème existants.
    Retrofit,
    /// Copie fidèle, sur un pack et un thème générés.
    Hifi,
}

/// Où va le contenu importé.
///
/// En JSON, étiqueté par `mode` : `{"mode": "retrofit", "layouts": […], "theme": …}`
/// (`layouts` et `theme` facultatifs) ou `{"mode": "hifi"}` (le témoin n'en vient jamais : l'hôte le choisit).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ImportTarget {
    /// Sur des packs de layouts et un thème de la bibliothèque.
    Retrofit {
        /// Les packs de layouts, dans l'ordre : chacun son id ou son nom ; aussi son dossier avec
        /// [`PackLookup::LibraryOrDir`]. Vide : `essentiel`.
        #[serde(default)]
        layouts: Vec<String>,
        /// Le thème : son id ou son nom ; `None` : le thème par défaut.
        #[serde(default)]
        theme: Option<String>,
    },
    /// Sur un pack et un thème générés pour la présentation.
    Hifi {
        /// Le témoin de la référence (`[import] witness` dans l'app) ; `None` : pas de référence.
        #[serde(skip)]
        witness: Option<ImportWitness>,
    },
}

impl ImportTarget {
    /// Le mode de cette cible.
    pub fn mode(&self) -> ImportMode {
        match self {
            Self::Retrofit { .. } => ImportMode::Retrofit,
            Self::Hifi { .. } => ImportMode::Hifi,
        }
    }
}

/// Où un retrofit cherche ses packs cibles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackLookup {
    /// Dans la bibliothèque seulement, par id ou par nom.
    ///
    /// La cible de l'app, jamais un dossier.
    Library,
    /// Aussi un dossier de pack, lu borné par `[sources] max_sources_mib` : la CLI.
    LibraryOrDir,
}

/// Une demande d'import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRequest {
    /// Le `.pptx` à importer.
    ///
    /// Son nom de fichier nomme le pack et le thème générés (hi-fi).
    pub pptx: PathBuf,
    /// Le mode et sa cible.
    pub target: ImportTarget,
    /// Où le retrofit cherche ses packs ; sans effet en hi-fi.
    pub packs: PackLookup,
    /// Hi-fi : chercher aussi les polices de la présentation parmi celles du système.
    pub system_fonts: bool,
}

/// Un import appliqué, pas encore rendu ni enregistré.
#[derive(Debug)]
pub struct Imported {
    /// Les sources du deck.
    ///
    /// Le plan en une étape d'historique, packs, thèmes, assets et polices embarqués.
    pub sources: SourceBundle,
    /// Les widgets sondés de son pack, et le processus de rendu qui les rend.
    pub widgets: ProbedWidgets,
    /// Les problèmes du deck obtenu ; vide pour un plan valide, comme il doit l'être.
    pub issues: Vec<Issue>,
    /// L'état de l'import : rapport, présentation lue, témoin à venir.
    pub state: ImportState,
}

/// Un import prêt à s'appliquer.
///
/// Les sources (packs et thèmes embarqués), les widgets sondés de leur pack et le plan, dont la
/// sortie tient dans les budgets.
struct Prepared {
    bundle: SourceBundle,
    widgets: ProbedWidgets,
    plan: CheckedPlan,
}

/// Importe `request` : lu, projeté, sondé et appliqué en une étape d'historique.
///
/// En hi-fi, le témoin de `request` est choisi (LibreOffice cherché aussitôt), pas encore lancé.
///
/// `fonts` : les polices de l'app, celles du rendu (le catalogue du hi-fi en
/// part ; l'état y ajoute celles du deck). En hi-fi, `[import] score_threshold`
/// est vérifié avant toute lecture. Rien n'est écrit. Le libellé de l'étape
/// est dans la langue courante de l'interface.
pub fn import(platform: &Platform, fonts: &Fonts, request: &ImportRequest) -> Result<Imported> {
    let threshold = platform.settings.import.score_threshold;
    if matches!(request.target, ImportTarget::Hifi { .. }) {
        snafu::ensure!(threshold.is_finite() && threshold >= 0.0, ThresholdSnafu { value: threshold });
    }
    let input = input::read_input(platform, &request.pptx)?;
    // Les packs intégrés `base` et `essentiel` : la cible du hi-fi et le premier replica de l'import.
    let base = oku_slides_render::builtin_registry(fonts).context(RegistrySnafu)?;
    let name = request.pptx.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let prepared = match &request.target {
        ImportTarget::Retrofit { layouts, theme } => {
            retrofit::prepare(platform, &base, (layouts, theme.as_deref(), request.packs), &input)?
        }
        ImportTarget::Hifi { .. } => {
            let args = hifi::HifiArgs { fonts, base: &base, name, system_fonts: request.system_fonts };
            hifi::prepare(platform, args, &input)?
        }
    };
    let Prepared { mut bundle, widgets, mut plan } = prepared;
    for asset in plan.assets.values() {
        bundle.add_asset(asset.bytes.clone());
    }
    let mut deck = bundle.deck().clone();
    apply_all(&mut deck, plan.take_ops(), &widgets.registry).context(OpSnafu)?;
    let label = platform.live_localizer.current().tr("session-import-label");
    bundle.apply(deck, &label, None).context(SourcesSnafu)?;
    let ids: HashSet<String> = bundle.assets.keys().cloned().collect();
    let issues = validate(bundle.deck(), &widgets.registry, Some(&ids));
    let fonts = fonts.with(bundle.fonts.values().cloned());
    let limits = tune::TuneLimits::of(&platform.settings.import);
    let mut state = ImportState::new(
        request.target.mode(),
        plan,
        input,
        fonts,
        (threshold, limits),
        platform.live_localizer.clone(),
    );
    if let ImportTarget::Hifi { witness } = request.target {
        state.choose_witness(platform, witness);
    }
    Ok(Imported { sources: bundle, widgets, issues, state })
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
