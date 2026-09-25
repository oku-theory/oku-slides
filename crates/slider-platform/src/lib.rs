//! Socle d'Écho : ce qui relie l'app à son environnement.
//!
//! - [`telemetry`] : journaux structurés (`tracing`), à installer en premier.
//! - [`errors`] : une erreur et ses causes, sur une ligne.
//! - [`config`] : réglages (défauts embarqués, `config.toml`, `ECHO_…`).
//! - [`storage`] : fichiers locaux, par OpenDAL.
//! - [`Platform`] : réglages, fichiers et langue, réunis au démarrage.
//!
//! Ordre de démarrage d'un binaire : journaux → réglages → infrastructure →
//! état de l'app.

pub mod config;
pub mod errors;
pub mod storage;

use std::sync::Arc;

use slider_i18n::Localizer;
use snafu::{Backtrace, ResultExt, Snafu};

use crate::config::{ConfigError, Settings};
use crate::storage::{Storage, StorageError};

/// Ce dont l'app a besoin de son environnement : réglages, fichiers et
/// langue. Se crée une fois au démarrage ; se clone sans rien recréer.
#[derive(Debug, Clone)]
pub struct Platform {
    pub settings: Arc<Settings>,
    pub storage: Storage,
    pub localizer: Arc<Localizer>,
}

/// Démarrage impossible : réglages ou fichiers.
#[derive(Debug, Snafu)]
pub enum PlatformError {
    #[snafu(display("settings could not be loaded"))]
    Settings { source: ConfigError, backtrace: Backtrace },
    #[snafu(display("storage could not start"))]
    Storage { source: StorageError, backtrace: Backtrace },
}

impl Platform {
    /// Réglages (défauts, `config.toml`, environnement), fichiers locaux et
    /// langue des réglages (sinon celle du système). À appeler après
    /// [`telemetry::init`].
    pub fn load() -> Result<Self, PlatformError> {
        Self::with_settings(Settings::load().context(SettingsSnafu)?)
    }

    /// La plateforme de ces réglages (tests, outils).
    pub fn with_settings(settings: Settings) -> Result<Self, PlatformError> {
        let localizer = Localizer::new(settings.i18n.locale.as_deref());
        Ok(Self {
            settings: Arc::new(settings),
            storage: Storage::local().context(StorageSnafu)?,
            localizer: Arc::new(localizer),
        })
    }
}
pub mod telemetry;
