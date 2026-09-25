use speculoos::prelude::*;

use super::repetitive;

#[test]
fn repetitive_syllable_loop_returns_true() {
    assert_that!(repetitive("Arudun, slid, sud, affaidaudududududududududududududududududududududududududududududud"))
        .is_true();
    assert_that!(repetitive("Houdun slidzu la feuille de route d'un mjmjmjmjjjjjjjjjjjj")).is_true();
}

#[test]
fn repetitive_normal_sentences_return_false() {
    assert_that!(repetitive("Ajoute une slide sur la feuille de route du deuxième trimestre, avec trois colonnes."))
        .is_false();
    assert_that!(repetitive("Ask what you can do for your country, ask what your country can do for you.")).is_false();
    assert_that!(repetitive("Il a dit : « non, non, non » et c'est tout. Rendez-vous le 11/11/2026.")).is_false();
}
