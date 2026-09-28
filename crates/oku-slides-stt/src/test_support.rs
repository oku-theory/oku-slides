//! Outillage des tests unitaires de `oku-slides-stt`.

/// Un dossier neuf, créé atomiquement sous un nom aléatoire (t-273).
///
/// Jamais celui d'un lancement précédent, même quand macOS recycle le PID
/// (nextest lance un processus par test). Supprimé au `drop`.
pub(crate) fn fresh_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test impossible : {e}"))
}
