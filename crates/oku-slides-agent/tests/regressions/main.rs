//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).

// `regressions` est sa propre cible de test : l'outillage commun est inclus par chemin.
#[path = "../common/mod.rs"]
mod common;

mod issue_0273_test_dirs_never_removed;
mod issue_0611_memory_lost_only_on_refused_resume;
mod issue_0612_late_load_replay_ignored;
