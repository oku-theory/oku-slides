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
mod errors_tests {
    use super::one_line;

    #[derive(Debug)]
    struct Outer(std::io::Error);
    impl std::fmt::Display for Outer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("could not save")
        }
    }
    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn one_line_with_cause_joins_the_chain() {
        let e = Outer(std::io::Error::other("disk full"));
        assert_eq!(one_line(&e), "could not save: disk full");
    }

    #[test]
    fn one_line_without_cause_returns_the_message() {
        assert_eq!(one_line(&std::io::Error::other("alone")), "alone");
    }
}
