//! Socle d'Écho : ce qui relie l'app à son environnement.
//!
//! - [`telemetry`] : journaux structurés (`tracing`), à installer en premier.
//! - [`errors`] : une erreur et ses causes, sur une ligne.
//! - [`config`] : réglages (défauts embarqués, `config.toml`, `ECHO_…`).
//! - [`storage`] : fichiers locaux, par OpenDAL.
//!
//! Ordre de démarrage d'un binaire : journaux → réglages → infrastructure →
//! état de l'app.

pub mod config;
pub mod errors;
pub mod storage;
pub mod telemetry;
