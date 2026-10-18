//! Socle d'Oku Slides : ce qui relie l'app à son environnement.
//!
//! - [`telemetry`] : journaux structurés (`tracing`), à installer en premier.
//! - [`errors`] : une erreur et ses causes, sur une ligne.
//! - [`config`] : réglages (défauts embarqués, `config.toml`, `OKU_SLIDES_…`).
//! - [`storage`] : fichiers locaux, par OpenDAL.
//! - [`updates`] : ce que décide la mise à jour de l'app, sans Tauri (hôte seulement).
//! - [`config_write`] : écriture d'une clé dans `config.toml`.
//! - [`find_program`] : un programme externe, par son chemin ou dans `PATH`.
//! - [`Platform`] : réglages, fichiers et langue, réunis au démarrage.
//! - `listen` (feature `listen`) : boucle d'accept bornée des serveurs HTTP
//!   locaux, `serve_connections` et ses `Limits`.
//!
//! Ordre de démarrage d'un binaire : journaux → réglages → infrastructure →
//! état de l'app.

pub mod config;
pub mod config_write;
pub mod errors;
#[cfg(feature = "listen")]
mod listen;
mod program;
pub mod storage;
#[cfg(test)]
mod test_support;
pub mod updates;

use std::path::PathBuf;
use std::sync::Arc;

use oku_slides_i18n::{LiveLocalizer, Localizer};
use snafu::{Backtrace, ResultExt, Snafu};

use crate::config::{ConfigError, Settings};
use crate::storage::{Storage, StorageError};

#[cfg(feature = "listen")]
pub use crate::listen::{Limits, serve_connections};
pub use crate::program::find_program;

/// Où tourne le rendu Typst.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderLaunch {
    /// Sur un fil du processus : aucune borne appliquée (tests, outils).
    InProcess,
    /// Dans un sous-processus `program --render-worker`, borné par `[render]`.
    Process { program: PathBuf },
}

/// Variable posée par l'hôte dans l'environnement de son processus de rendu.
///
/// Un processus qui la porte ne lance jamais de travailleur : un binaire qui
/// aurait oublié `exit_if_worker` ne se relance pas en chaîne.
pub const RENDER_WORKER_ENV: &str = "OKU_SLIDES_RENDER_WORKER";

/// Argument qui fait d'un binaire de l'app l'agent local (voir `oku-slides-local-agent`).
pub const LOCAL_AGENT_FLAG: &str = "--local-agent";

/// Le lancement du rendu d'un binaire `program` ; `in_worker` : ce processus
/// est lui-même un travailleur (voir [`RENDER_WORKER_ENV`]), il rend sur un fil.
pub(crate) fn render_launch_for(program: PathBuf, in_worker: bool) -> RenderLaunch {
    if in_worker {
        tracing::warn!(
            program = %program.display(),
            "processus de rendu sans exit_if_worker : rendu sur un fil, sans relance"
        );
        return RenderLaunch::InProcess;
    }
    RenderLaunch::Process { program }
}

/// Ce dont l'app a besoin de son environnement : réglages, fichiers et
/// langue. Se crée une fois au démarrage ; se clone sans rien recréer.
#[derive(Debug, Clone)]
pub struct Platform {
    pub settings: Arc<Settings>,
    pub storage: Storage,
    /// Langue du lancement : textes de session, journaux, rendu.
    pub localizer: Arc<Localizer>,
    /// Langue courante de l'interface, changée à chaud : textes de l'agent et de ses outils.
    pub live_localizer: LiveLocalizer,
    pub render_launch: RenderLaunch,
    /// Ce binaire, relancé pour l'agent local ; `None` hors de l'app (tests).
    pub program: Option<PathBuf>,
}

/// Démarrage impossible : réglages ou fichiers.
#[derive(Debug, Snafu)]
pub enum PlatformError {
    #[snafu(display("settings could not be loaded"))]
    Settings { source: ConfigError, backtrace: Backtrace },
    #[snafu(display("storage could not start"))]
    Storage { source: StorageError, backtrace: Backtrace },
    #[snafu(display("the executable path could not be read"))]
    CurrentExe { source: std::io::Error, backtrace: Backtrace },
}

impl Platform {
    /// Réglages (défauts, `config.toml`, environnement), fichiers locaux et
    /// langue des réglages (sinon celle du système). À appeler après
    /// [`telemetry::init`]. Le rendu se lance en sous-processus de ce
    /// binaire, sauf dans un processus de rendu (voir [`RENDER_WORKER_ENV`]).
    pub fn load() -> Result<Self, PlatformError> {
        let program = std::env::current_exe().context(CurrentExeSnafu)?;
        let in_worker = std::env::var_os(RENDER_WORKER_ENV).is_some();
        Ok(Self::with_settings(Settings::load().context(SettingsSnafu)?)?
            .with_program(program.clone())
            .with_render_launch(render_launch_for(program, in_worker)))
    }

    /// La plateforme de ces réglages (tests, outils).
    pub fn with_settings(settings: Settings) -> Result<Self, PlatformError> {
        let localizer = Arc::new(Localizer::new(settings.i18n.locale.as_deref()));
        Ok(Self {
            settings: Arc::new(settings),
            storage: Storage::local().context(StorageSnafu)?,
            live_localizer: LiveLocalizer::new(localizer.clone()),
            localizer,
            render_launch: RenderLaunch::InProcess,
            program: None,
        })
    }

    /// Cette plateforme, dont l'agent local se lance avec `program` (tests).
    pub fn with_program(mut self, program: PathBuf) -> Self {
        self.program = Some(program);
        self
    }

    /// Cette plateforme, avec le rendu lancé par `launch`.
    pub fn with_render_launch(mut self, launch: RenderLaunch) -> Self {
        self.render_launch = launch;
        self
    }
}
pub mod telemetry;
