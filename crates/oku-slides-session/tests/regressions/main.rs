//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).

#[path = "../common/mod.rs"]
#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

mod issue_0224_agent_edits_resume;
mod issue_0264_stale_test_data_dirs;
mod issue_0328_pack_recu_garde_la_version_courante;
