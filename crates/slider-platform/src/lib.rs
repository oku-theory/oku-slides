//! Socle d'Écho : ce qui relie l'app à son environnement.
//!
//! - [`telemetry`] : journaux structurés (`tracing`), à installer en premier.
//!
//! Ordre de démarrage d'un binaire : journaux → réglages → infrastructure →
//! état de l'app.

pub mod telemetry;
