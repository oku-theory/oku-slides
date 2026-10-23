//! t-1106 : éditer un thème depuis le mode deck vérifiait ses jetons sur la fermeture prise
//! dans la bibliothèque seule.
//!
//! Un parent que seul le deck embarque (thème reçu, ou retiré de la bibliothèque) faisait
//! échouer l'édition (`UnknownTheme`) au lieu de la vérifier sur la version épinglée.

use oku_slides_session::TokenOp;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{add_user_theme, data_dir, fixture_path, ns, session_in};

/// Retire de la bibliothèque des thèmes de `data` le thème d'espace de noms `namespace`.
fn remove_library_theme(data: &std::path::Path, namespace: &str) {
    let themes = data.join("themes");
    let entries = std::fs::read_dir(&themes).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    for dir in entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        let versions = dir.join("versions");
        let Ok(found) = std::fs::read_dir(&versions) else { continue };
        let named = found.flatten().any(|v| {
            let text = std::fs::read_to_string(v.path().join("theme.json")).unwrap_or_default();
            serde_json::from_str::<serde_json::Value>(&text).is_ok_and(|j| j["namespace"] == namespace)
        });
        if named {
            std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("retrait : {e}"));
        }
    }
}

#[test]
fn issue_1106_deck_theme_edit_validates_against_the_embedded_parent() {
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
    let marque = s.find_theme_info("marque").unwrap_or_else(|e| panic!("marque : {}", e.one_line())).id;
    let op = TokenOp::SetToken { path: vec!["color".into(), "ink".into()], value: json!("#111111") };
    let edited = s.edit_theme(marque, None, vec![op]).map(|t| t.changed);
    assert_that!(edited.map_err(|e| e.kind())).is_equal_to(Ok(true));
}
