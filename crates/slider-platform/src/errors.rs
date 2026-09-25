//! Affichage des erreurs : une erreur et toutes ses causes.

use std::error::Error;

/// L'erreur puis ses causes, séparées par « : », sur une seule ligne.
pub fn one_line(error: &dyn Error) -> String {
    let mut out = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        out.push_str(": ");
        out.push_str(&cause.to_string());
        source = cause.source();
    }
    out
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod errors_tests;
