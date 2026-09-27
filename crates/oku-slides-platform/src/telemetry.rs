//! Journaux structurés. `info` : ce qui compte pour l'utilisateur (document
//! ouvert, enregistré, modèle téléchargé) ; `debug` : le fonctionnement
//! (rendu, dictée) ; `trace` : ce qui se répète (boucles, sondages).

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Niveau par défaut, sans `RUST_LOG` : `info` pour nous, `warn` pour Loro
/// (`loro`, `loro_internal`) dont les journaux `info` (import, commit)
/// n'intéressent pas l'utilisateur et noient les nôtres.
pub(crate) const DEFAULT_FILTER: &str = "info,loro=warn,loro_internal=warn";

/// Installe l'abonné des journaux, sur la sortie d'erreur.
///
/// En développement, `RUST_LOG` choisit le niveau (défaut : [`DEFAULT_FILTER`]) ;
/// dans un build de production, le niveau est celui de [`DEFAULT_FILTER`],
/// sans réglage. Sans effet si un abonné est déjà installé (tests, deuxième
/// appel).
pub fn init() {
    let filter = if cfg!(debug_assertions) {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER))
    } else {
        EnvFilter::new(DEFAULT_FILTER)
    };
    // `try_init` : un abonné déjà installé n'est pas une erreur.
    let _ = tracing_subscriber::registry().with(fmt::layer().with_writer(std::io::stderr)).with(filter).try_init();
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod telemetry_tests;
