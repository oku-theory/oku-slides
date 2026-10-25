//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

// `regressions` est sa propre cible de test : l'outillage commun est inclus par chemin.
#[path = "../common/mod.rs"]
mod common;

mod issue_0222_permission_primary_never_allow_always;
mod issue_0273_stale_test_dirs;
mod issue_0637_tool_server_limits;
mod issue_0696_bearer_token_hash;
mod issue_0768_apply_states_rich_shape;
mod issue_0793_live_language;
mod issue_1144_agent_api_target;
mod issue_1144_set_setting_dictation;
mod issue_1171_asset_unknown_agent;
