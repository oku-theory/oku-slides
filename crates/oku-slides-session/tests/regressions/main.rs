//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[path = "../common/mod.rs"]
#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;
#[path = "../common/hifi_import.rs"]
mod hifi_import;
#[path = "../common/workspace_support.rs"]
mod workspace_support;

mod issue_0195_options_aux_identifiants_en_double;
mod issue_0224_agent_edits_resume;
mod issue_0264_stale_test_data_dirs;
mod issue_0328_pack_recu_garde_la_version_courante;
mod issue_0329_widget_casse_n_empeche_pas_les_autres;
mod issue_0465_fait_blanc_compte_dans_la_borne;
mod issue_0524_valeurs_mal_formees_d_un_pack_importe;
mod issue_0554_action_jeton_sans_cle_path;
mod issue_0630_link_angle_bracket;
mod issue_0674_etat_ui_dans_l_ordre;
mod issue_0694_reprise_v7_mise_de_cote;
mod issue_0746_reprise_v8_mise_de_cote;
mod issue_0902_deck_only_workshop;
mod issue_1015_pack_fault_isolated;
mod issue_1016_deck_widget_goes_to_deck_pack;
mod issue_1039_new_deck_pins_base;
mod issue_1039_replicas_know_base_hosts;
mod issue_1040_heavy_base_content_disables_no_pack;
mod issue_1040_heavy_base_content_spares_a_cheap_user_pack;
mod issue_1063_new_deck_pins_essentiel_layouts;
mod issue_1063_old_tab_set_aside_notice;
mod issue_1068_layout_example_emptied_by_element_op;
mod issue_1068_legacy_essentiel_copy;
mod issue_1091_requires_ignores_legacy_builtin_copy;
mod issue_1093_pack_save_conflict_writes_no_version;
mod issue_1098_theme_closure_repaired_after_merge;
mod issue_1098_theme_layer_of_a_child;
mod issue_1099_parent_seul_embarque;
mod issue_1106_deck_theme_edit_uses_embedded_closure;
mod issue_1111_layout_examples_follow_ui_language;
mod issue_1112_pack_updates_inside_runtime;
mod issue_1178_coller_image_autre_onglet;
mod issue_1182_copier_coller_entre_fenetres;
