//! Erreurs du serveur d'outils : il n'a pas pu démarrer.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// Le serveur d'outils de l'agent n'a pas démarré.
#[derive(Debug, Snafu)]
pub struct McpError(pub(crate) McpInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum McpInner {
    #[snafu(display("cannot listen on 127.0.0.1"))]
    Bind { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("no randomness for the token"))]
    Token { source: getrandom::Error, backtrace: Backtrace },
}

impl Localize for McpError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr("agent-tools-unavailable")
    }
}
