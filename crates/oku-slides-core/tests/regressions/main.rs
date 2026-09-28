//! Tests de non-régression : un fichier par bogue corrigé.
//! Numérotation `issue_NNNN` : identifiant de tâche Oku quand il existe, locale sinon.
//!
//! `issue_0164` (taille de l'enum `ElementBody`) a disparu avec l'enum : les
//! éléments v2 sont génériques (`{id, kind, params}`).

#[path = "../common/mod.rs"]
mod common;

mod issue_0001_asset_id_escaping_or_injecting;
mod issue_0263_undo_reorder_loses_slides;
mod issue_0292_grid_hides_blocks;
