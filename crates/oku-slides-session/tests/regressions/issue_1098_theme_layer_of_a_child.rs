//! t-1098 : un enfant de Pro (le test en construit un) rend avec les jetons de Pro sous les siens :
//! le calque envoyé au rendu suit la chaîne linéarisée, pas le seul thème du deck.

use serde_json::json;
use speculoos::prelude::*;

use crate::common::{add_user_theme, data_dir, fixture_path, session_in};

#[test]
fn child_theme_renders_with_its_parent_tokens_under_its_own() {
    let data = data_dir();
    // L'enfant ne pose que la surface ; l'accent vient de son parent Pro.
    add_user_theme(&data, "marque", &["pro"], json!({"color": {"surface": "#123456"}}));
    let mut s = session_in(&data);
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
    s.set_theme(&crate::common::ns("marque")).unwrap_or_else(|e| panic!("thème : {}", e.one_line()));
    let appearance =
        serde_json::to_value(s.snapshot()).unwrap_or_else(|e| panic!("snapshot : {e}"))["appearance"].clone();
    assert_that!(appearance["slide"]["accent"]).is_equal_to(json!("#00bef4"));
    assert_that!(appearance["slide"]["surface"]).is_equal_to(json!("#123456"));
}
