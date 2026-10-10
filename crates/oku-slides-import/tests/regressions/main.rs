//! Tests de régression de l'import (un fichier par bug).

#[path = "../common/mod.rs"]
mod common;
mod issue_692_bomb_rereads;
mod issue_692_invisible_chars;
mod issue_692_layout_dedup;
mod issue_692_merged_cell_edges;
mod issue_692_xml_depth;
mod issue_692_xml_nodes_limit;
