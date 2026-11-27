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
//! - `http` (feature `download`) : `system_tls`, TLS for outgoing HTTP clients.
//! - [`models`] : fichiers de modèles épinglés, téléchargés et vérifiés (feature `download`).
//! - `listen` (feature `listen`) : boucle d'accept bornée des serveurs HTTP
//!   locaux, `serve_connections` et ses `Limits`.
//!
//! Ordre de démarrage d'un binaire : journaux → réglages → infrastructure →
//! état de l'app.

pub mod bases;
pub mod config;
pub mod config_write;
pub mod errors;
#[cfg(feature = "download")]
pub mod http;
#[cfg(feature = "listen")]
mod listen;
mod log_ring;
mod log_secrets;
mod login_path;
pub mod models;
mod program;
pub mod storage;
#[cfg(test)]
mod test_support;
pub mod updates;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_i18n::{LiveLocalizer, Localizer};
use snafu::{Backtrace, ResultExt, Snafu};

use crate::bases::{BaseStore, BaseStoreError};
use crate::config::{ConfigError, Settings};
use crate::storage::{Storage, StorageError};

#[cfg(feature = "listen")]
pub use crate::listen::{Limits, serve_connections};
pub use crate::login_path::ProgramPath;
pub use crate::program::find_program;

/// Où tourne le rendu Typst.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderLaunch {
    /// Sur un fil du processus : aucune borne appliquée (tests, outils).
    InProcess,
    /// Dans un sous-processus `program --render-worker`, borné par `[render]`.
    Process {
        /// App binary relaunched as a render worker.
        program: PathBuf,
    },
}

/// Variable posée par l'hôte dans l'environnement de son processus de rendu.
///
/// Un processus qui la porte ne lance jamais de travailleur : un binaire qui
/// aurait oublié `exit_if_worker` ne se relance pas en chaîne.
pub const RENDER_WORKER_ENV: &str = "OKU_SLIDES_RENDER_WORKER";

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
    /// Settings read at startup, shared without copying.
    pub settings: Arc<Settings>,
    /// File access, the only path to the disk.
    pub storage: Storage,
    /// Langue du lancement : textes de session, journaux, rendu.
    pub localizer: Arc<Localizer>,
    /// Langue courante de l'interface, changée à chaud : textes de l'agent et de ses outils.
    pub live_localizer: LiveLocalizer,
    /// Where Typst rendering runs: on a thread or in a bounded subprocess.
    pub render_launch: RenderLaunch,
    /// The `PATH` where external programs (agent, soffice) are found and launched.
    ///
    /// The process's own, unless [`Platform::with_login_path`] put the login shell's first.
    pub program_path: ProgramPath,
    /// The data base store, opened on first use and shared by every clone.
    bases: Arc<Mutex<Option<BaseStore>>>,
}

/// Démarrage impossible : réglages ou fichiers.
#[derive(Debug, Snafu)]
pub enum PlatformError {
    /// Unreadable settings: invalid `config.toml` or environment.
    #[snafu(display("settings could not be loaded"))]
    Settings {
        /// Error reading the settings.
        source: ConfigError,
        /// Call stack when the error was created.
        backtrace: Backtrace,
    },
    /// The local storage (OpenDAL) did not start.
    #[snafu(display("storage could not start"))]
    Storage {
        /// Storage startup error.
        source: StorageError,
        /// Call stack when the error was created.
        backtrace: Backtrace,
    },
    /// The path of this binary is unreadable.
    #[snafu(display("the executable path could not be read"))]
    CurrentExe {
        /// System error from `current_exe`.
        source: std::io::Error,
        /// Call stack when the error was created.
        backtrace: Backtrace,
    },
}

impl Platform {
    /// Settings (defaults, `config.toml`, environment), local files and
    /// the settings' language (otherwise the system one). To call after
    /// [`telemetry::init`]. Rendering runs in a subprocess of this
    /// binary, except inside a render process (see [`RENDER_WORKER_ENV`]).
    ///
    /// # Errors
    ///
    /// If the binary path is unreadable, the settings invalid or the storage unavailable.
    pub fn load() -> Result<Self, PlatformError> {
        let program = std::env::current_exe().context(CurrentExeSnafu)?;
        let in_worker = std::env::var_os(RENDER_WORKER_ENV).is_some();
        Ok(Self::with_settings(Settings::load().context(SettingsSnafu)?)?
            .with_render_launch(render_launch_for(program, in_worker)))
    }

    /// The platform for these settings (tests, tools).
    ///
    /// # Errors
    ///
    /// If the local storage does not start.
    pub fn with_settings(settings: Settings) -> Result<Self, PlatformError> {
        let localizer = Arc::new(Localizer::new(settings.i18n.locale.as_deref()));
        Ok(Self {
            settings: Arc::new(settings),
            storage: Storage::local().context(StorageSnafu)?,
            live_localizer: LiveLocalizer::new(localizer.clone()),
            localizer,
            render_launch: RenderLaunch::InProcess,
            program_path: ProgramPath::fixed(std::env::var_os("PATH")),
            bases: Arc::default(),
        })
    }

    /// The data base store in the data folder, created on first call; clones share it.
    ///
    /// Lazy: a process that never opens a deck writes nothing.
    ///
    /// # Errors
    ///
    /// If the store's folders cannot be created.
    pub fn bases(&self) -> Result<BaseStore, BaseStoreError> {
        let mut slot = self.bases.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(store) = &*slot {
            return Ok(store.clone());
        }
        let sources = &self.settings.sources;
        let store = BaseStore::new(self.storage.clone(), &self.settings.paths.data(), sources.max_store_bytes())?;
        *slot = Some(store.clone());
        Ok(store)
    }

    /// This platform, its `path` led by the login shell's `PATH` (app launched from the Finder).
    ///
    /// Runs the user's shell once on a thread (3 s at most); without one, the process's `PATH`
    /// stays. For the app only: a terminal already gives the CLI the user's `PATH`.
    #[must_use]
    pub fn with_login_path(mut self) -> Self {
        self.program_path = ProgramPath::from_login_shell(std::env::var_os("PATH"));
        self
    }

    /// Cette plateforme, avec le rendu lancé par `launch`.
    #[must_use]
    pub fn with_render_launch(mut self, launch: RenderLaunch) -> Self {
        self.render_launch = launch;
        self
    }
}
pub mod telemetry;
