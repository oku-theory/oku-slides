//! Tests de non-régression : un fichier par bogue corrigé, numéroté par tâche.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

/// Un dossier de test neuf, supprimé à la fin du test.
///
/// Créé atomiquement sous un nom aléatoire (t-273) : jamais celui d'un
/// lancement précédent, même quand macOS recycle le PID.
fn temp_dir(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("oku-slides-regression-{name}-"))
        .tempdir()
        .unwrap_or_else(|e| panic!("dossier de test impossible : {e}"))
}

mod issue_0163_symlinked_dir_or_oversized_read;
mod issue_0273_stale_test_dirs;
mod issue_1202_agent_selected_invariant;
// ureq, and so `http`, only exist with the `download` feature.
#[cfg(feature = "download")]
mod issue_1211_system_root_certs;
mod issue_1212_update_download_endpoint;
