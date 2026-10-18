//! Erreurs de l'agent local : lancement, moteur et protocole ACP.
//!
//! Elles ne vont que dans les journaux (stderr) ; l'hôte montre l'échec du lancement.

#[cfg(debug_assertions)]
use std::path::PathBuf;

use snafu::{Backtrace, Snafu};

/// L'agent local n'a pas pu démarrer ou servir ACP.
#[derive(Debug, Snafu)]
pub struct LocalAgentError(pub(crate) Inner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum Inner {
    #[snafu(display("no local engine available"))]
    NoEngine { backtrace: Backtrace },
    #[cfg(debug_assertions)]
    #[snafu(display("fake engine script {} could not be read", path.display()))]
    Script { path: PathBuf, source: std::io::Error, backtrace: Backtrace },
    #[cfg(debug_assertions)]
    #[snafu(display("fake engine script {} is not valid", path.display()))]
    ScriptFormat { path: PathBuf, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("cannot start the async runtime"))]
    Runtime { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("ACP connection failed"))]
    Acp {
        #[snafu(source(from(agent_client_protocol::Error, Box::new)))]
        source: Box<agent_client_protocol::Error>,
        backtrace: Backtrace,
    },
}
