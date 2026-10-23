//! t-1098 : deux copies d'un deck changent de thème chacune ; la fusion garde
//! un thème mais l'union des épinglages. La session répare la fermeture :
//! elle redevient exactement celle du thème gardé, ses versions embarquées.

use serde_json::json;
use speculoos::prelude::*;

use crate::common::{add_user_theme, data_dir, fixture_path, session_in};

#[test]
fn merge_of_two_theme_changes_leaves_the_exact_closure_of_the_kept_theme() {
    let ok = |r: oku_slides_session::Result<()>| r.unwrap_or_else(|e| panic!("{}", e.one_line()));
    let (here, there, files) = (data_dir(), data_dir(), data_dir());
    let path = files.join("deck.oku-slides.pdf");
    let mut a = session_in(&here);
    ok(a.open(&fixture_path()));
    ok(a.save(Some(&path)).map(drop));
    // L'autre copie passe à un enfant de Papier, de sa bibliothèque seule.
    add_user_theme(&there, "marque", &["papier"], json!({"color": {"accent": "#ff0000"}}));
    let mut b = session_in(&there);
    ok(b.open(&path));
    ok(b.set_theme(&crate::common::ns("marque")));
    let other = files.join("autre.oku-slides.pdf");
    ok(b.save(Some(&other)).map(drop));
    // Celle-ci passe à Pro, puis reçoit l'autre.
    ok(a.set_theme(&crate::common::ns("pro")));
    ok(a.merge_file(&other).map(drop));
    let deck = a.deck();
    assert_that!(oku_slides_core::check_closure(deck.theme.as_ref(), &deck.themes)).is_ok();
    let expected: Vec<&str> = match deck.theme.as_ref().map(oku_slides_core::Namespace::as_str) {
        Some("pro") => vec!["pro"],
        Some("marque") => vec!["marque", "papier"],
        other => panic!("thème fusionné inattendu : {other:?}"),
    };
    assert_that!(deck.themes.keys().map(oku_slides_core::Namespace::as_str).collect::<Vec<_>>()).is_equal_to(expected);
    let sources = a.sources_for_tests();
    assert_that!(deck.themes.values().all(|p| sources.themes.contains_key(&p.version))).is_true();
}
