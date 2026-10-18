//! t-903 : le thème du deck seul d'un import hi-fi se reprend après un autre.
//!
//! Avant t-903, la liste des thèmes ne montrait que la bibliothèque : après
//! Papier, le thème généré par l'import ne revenait que par l'annulation, et
//! `set_theme` le refusait (`UnknownTheme`). Désormais le snapshot l'offre
//! (`deck_themes`), même après enregistrement et réouverture, et le
//! reprendre ramène sa version sans rien écrire dans la bibliothèque.

use oku_slides_session::{PAPIER_THEME, Request};
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::hifi_import::{imported_hifi, library_files};

#[test]
fn import_theme_is_offered_and_repicked_after_papier_save_and_reopen() {
    let data = data_dir();
    let mut s = imported_hifi(&data);
    let imported = s.deck().theme.clone().unwrap_or_else(|| panic!("thème de l'import"));
    let before = library_files(&data, "themes");

    s.set_theme(PAPIER_THEME).unwrap_or_else(|e| panic!("Papier : {}", e.one_line()));
    let path = data.join("importe.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false })
        .unwrap_or_else(|e| panic!("enregistrement du deck : {}", e.one_line()));

    let mut reopened = crate::common::session_in(&data);
    reopened.open(&path).unwrap_or_else(|e| panic!("réouverture : {}", e.one_line()));
    let listed: Vec<_> =
        reopened.snapshot().deck_themes.iter().map(|t| (t.id, t.version.clone(), t.deck_only)).collect();
    assert_that!(listed).is_equal_to(vec![(imported.id, imported.version.clone(), true)]);

    reopened.set_theme(imported.id).unwrap_or_else(|e| panic!("reprise : {}", e.one_line()));
    assert_that!(reopened.deck().theme.clone()).is_equal_to(Some(imported));
    assert_that!(library_files(&data, "themes")).is_equal_to(before);
}
