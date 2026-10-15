//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).

// `regressions` est sa propre cible de test : l'outillage commun est inclus par chemin.
#[path = "../common/mod.rs"]
mod common;

mod issue_0181_serve_obeys_foreign_origins;
mod issue_0193_serve_exposed_requires_a_token;
mod issue_0273_stale_test_dirs;
mod issue_0355_cli_valide_avec_le_pack_du_deck;
mod issue_0419_serve_answers_a_burst_of_keep_alive_connections;
