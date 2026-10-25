//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

// `regressions` est sa propre cible de test : l'outillage commun est inclus par chemin.
#[path = "../common/mod.rs"]
mod common;

mod issue_0273_stale_test_dirs;
mod issue_1205_fake_mic_low_rate;
