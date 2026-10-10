//! Tests de non-régression : un fichier par bogue corrigé (`issue_<tâche>`).

#[path = "../common/mod.rs"]
#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

mod issue_0195_options_aux_identifiants_en_double;
mod issue_0224_agent_edits_resume;
mod issue_0264_stale_test_data_dirs;
mod issue_0328_pack_recu_garde_la_version_courante;
mod issue_0329_widget_casse_n_empeche_pas_les_autres;
mod issue_0465_fait_blanc_compte_dans_la_borne;
mod issue_0524_valeurs_mal_formees_d_un_pack_importe;
mod issue_0554_action_jeton_sans_cle_path;
mod issue_0613_annuler_un_renommage_ramene_la_conversation;
mod issue_0630_link_angle_bracket;
mod issue_0674_etat_ui_dans_l_ordre;
mod issue_0694_reprise_v7_mise_de_cote;
mod issue_0746_reprise_v8_mise_de_cote;
