//! Lot 4 des packs (issue 1099) : enregistrer le pack adopte le thème édité même quand son parent
//! n'est plus que dans le deck.
//!
//! La fermeture du thème adopté par le deck mis de côté prenait les parents
//! dans la bibliothèque seule : un parent que seul le deck embarque (thème
//! reçu, ou retiré de la bibliothèque) faisait échouer l'enregistrement
//! (`UnknownTheme`) après l'écriture de la version du pack.

use oku_slides_core::Op;
use oku_slides_session::ESSENTIEL_PACK;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{add_user_theme, data_dir, fixture_path, ns, session_in};

/// Retire de la bibliothèque des thèmes de `data` le thème d'espace de noms `namespace`.
fn remove_library_theme(data: &std::path::Path, namespace: &str) {
    let themes = data.join("themes");
    let entries = std::fs::read_dir(&themes).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    let named = |f: &std::path::PathBuf| {
        let json = std::fs::read_to_string(f).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
        json.is_some_and(|v| v["namespace"] == namespace)
    };
    for dir in entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        if walkdir(&dir).iter().any(named) {
            std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("retrait : {e}"));
        }
    }
}

/// Les fichiers sous `dir`, à toute profondeur.
fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries.flatten().map(|e| e.path()).flat_map(|p| if p.is_dir() { walkdir(&p) } else { vec![p] }).collect()
}

#[test]
fn saving_a_pack_adopts_an_edited_theme_whose_parent_is_only_embedded() {
    let data = data_dir();
    add_user_theme(&data, "socle", &[], json!({"color": {"accent": "#204060"}}));
    add_user_theme(&data, "marque", &["socle"], json!({"color": {"surface": "#123456"}}));
    let file = data.join("deck.oku-slides.pdf");
    {
        let mut s = session_in(&data);
        s.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
        s.set_theme(&ns("marque")).unwrap_or_else(|e| panic!("thème : {}", e.one_line()));
        s.save(Some(&file)).unwrap_or_else(|e| panic!("enregistrement : {}", e.one_line()));
    }
    remove_library_theme(&data, "socle");

    let mut s = session_in(&data);
    assert_that!(s.snapshot().themes.iter().any(|t| t.namespace == ns("socle"))).is_false();
    s.open(&file).unwrap_or_else(|e| panic!("réouverture : {}", e.one_line()));
    s.edit_pack(ESSENTIEL_PACK).unwrap_or_else(|e| panic!("atelier : {}", e.one_line()));
    let op = Op::SetToken { path: vec!["color".into(), "ink".into()], value: json!("#111111") };
    s.apply("encre", vec![op], None).unwrap_or_else(|e| panic!("jeton : {}", e.one_line()));
    s.save_pack(false).unwrap_or_else(|e| panic!("enregistrement du pack : {}", e.one_line()));
    s.close_pack().unwrap_or_else(|e| panic!("sortie : {}", e.one_line()));

    let deck = s.deck();
    assert_that!(deck.theme.clone()).is_equal_to(Some(ns("marque")));
    assert_that!(deck.themes.contains_key(&ns("socle"))).is_true();
    let appearance =
        serde_json::to_value(s.snapshot()).unwrap_or_else(|e| panic!("snapshot : {e}"))["appearance"].clone();
    assert_that!(appearance["slide"]["accent"]).is_equal_to(json!("#204060"));
}
