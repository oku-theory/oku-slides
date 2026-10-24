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

/// L'erreur de la session dite à l'utilisateur, dans sa langue : cartes du hub, et repli de l'agent.
///
/// Un fichier illisible ou impossible à écrire est dit sans son chemin : aucune erreur envoyée
/// ne contient de chemin absolu. L'erreur complète reste dans les journaux de l'appelant. Ce qui
/// ne s'adresse qu'à l'agent va dans `say_to_agent` (`tool_error`).
pub(crate) fn say_session(e: &oku_slides_session::SessionError, l: &Localizer) -> String {
    match e.kind() {
        oku_slides_session::SessionErrorKind::File => l.tr("agent-error-file"),
        _ => e.localize(l),
    }
}
