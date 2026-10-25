//! Tests de régression de l'import (un fichier par bug).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[path = "../common/mod.rs"]
mod common;
mod issue_0752_hostile_svg;
mod issue_0753_output_budget;
mod issue_0754_many_attributes;
mod issue_0755_chart_categories;
mod issue_0755_external_image;
mod issue_0755_path_bounds;
mod issue_0755_shared_theme;
mod issue_0755_text_bounds;
mod issue_0755_theme_no_fill;
mod issue_0892_clamp_warning;
mod issue_1092_generated_registry_error;
mod issue_692_bomb_rereads;
mod issue_692_invisible_chars;
mod issue_692_layout_dedup;
mod issue_692_merged_cell_edges;
mod issue_692_negative_extent;
mod issue_692_xml_depth;
mod issue_692_xml_nodes_limit;
