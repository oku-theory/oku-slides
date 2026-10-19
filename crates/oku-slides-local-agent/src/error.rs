//! Erreurs de l'agent local : lancement, moteur et protocole ACP.
//!
//! Elles ne vont que dans les journaux (stderr) ; l'hôte montre l'échec du lancement.

#[cfg(any(debug_assertions, all(target_os = "macos", target_arch = "aarch64")))]
use std::path::PathBuf;

use snafu::{Backtrace, Snafu};

/// L'agent local n'a pas pu démarrer ou servir ACP.
#[derive(Debug, Snafu)]
pub struct LocalAgentError(pub(crate) Inner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum Inner {
    #[snafu(display("local engine unavailable: {reason}"))]
    Unavailable { reason: String, backtrace: Backtrace },
    #[cfg(debug_assertions)]
    #[snafu(display("the fake engine needs a fake_script setting"))]
    NoScript { backtrace: Backtrace },
    #[snafu(display("cannot load the platform"))]
    Platform {
        #[snafu(source(from(oku_slides_platform::PlatformError, Box::new)))]
        source: Box<oku_slides_platform::PlatformError>,
        backtrace: Backtrace,
    },
    #[cfg(debug_assertions)]
    #[snafu(display("fake engine script {} could not be read", path.display()))]
    Script { path: PathBuf, source: std::io::Error, backtrace: Backtrace },
    #[cfg(debug_assertions)]
    #[snafu(display("fake engine script {} is not valid", path.display()))]
    ScriptFormat { path: PathBuf, source: serde_json::Error, backtrace: Backtrace },
    /// llama.cpp n'a pas pu charger le modèle ; ses erreurs, de types variés, arrivent en texte.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[snafu(display("cannot load the local model {}: {message}", path.display()))]
    Model { path: PathBuf, message: String, backtrace: Backtrace },
    #[snafu(display("cannot start the async runtime"))]
    Runtime { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("ACP connection failed"))]
    Acp {
        #[snafu(source(from(agent_client_protocol::Error, Box::new)))]
        source: Box<agent_client_protocol::Error>,
        backtrace: Backtrace,
    },
}
