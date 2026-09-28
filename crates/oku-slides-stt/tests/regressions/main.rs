//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).

// `regressions` est sa propre cible de test : l'outillage commun est inclus par chemin.
#[path = "../common/mod.rs"]
mod common;

mod issue_0273_stale_test_dirs;
