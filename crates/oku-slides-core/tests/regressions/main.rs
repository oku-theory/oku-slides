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
mod issue_0527_rich_list_links;
mod issue_0528_list_size_bounds;
mod issue_0591_set_kind_revalide_host;
mod issue_0652_undo_remove_vs_peer_move;
mod issue_0677_bundle_dedups_before_normalizing;
mod issue_0693_unique_ids_keeps_its_names;
mod issue_0694_move_across_slots_keeps_concurrent_edit;
mod issue_0694_undo_remove_vs_peer_move_in_tree;
mod issue_0694_undo_tree_moves_keep_elements;
mod issue_0746_move_between_slides_keeps_concurrent_edit;
mod issue_0746_undo_deck_tree_moves_keep_slides_and_elements;
mod issue_0747_undo_remove_keeps_the_peer_move_and_edit;
mod issue_0776_undo_add_keeps_moved_subtree;
mod issue_0809_deep_tree_chain;
mod issue_0865_inscribed_slot_keys;
mod issue_0866_hidden_depth_refused_on_save;
