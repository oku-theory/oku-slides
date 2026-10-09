//! Tests de non-régression : un fichier par bogue corrigé.
//! Numérotation locale (`issue_NNNN`), en l'absence d'identifiant de tâche.

#[path = "../common/mod.rs"]
mod common;

mod issue_0001_asset_id_escaping_or_injecting;
mod issue_0002_chart_small_frame_or_huge_data;
mod issue_0003_numbered_list_counts_up;
mod issue_0176_cjk_text_without_font;
mod issue_0325_couverture_sans_texte_ancree_en_bas;
mod issue_0544_chart_text_scaled_with_box;
mod issue_0554_police_graphiques_du_theme;
mod issue_0609_tableau_haut_partage_la_hauteur;
