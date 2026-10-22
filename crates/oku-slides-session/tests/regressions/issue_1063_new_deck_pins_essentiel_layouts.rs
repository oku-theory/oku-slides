//! t-1063 : un nouveau deck n'épinglait aucun pack de layouts : ses slides n'avaient pas de widget.

use oku_slides_core::kinds;
use speculoos::prelude::*;

use crate::common::{data_dir, session_in};

#[test]
fn issue_1063_new_deck_pins_essentiel_layouts() {
    let data = data_dir();
    let s = session_in(&data);
    let deck = s.deck();
    assert_that!(deck.layouts.contains_key(&kinds::essentiel::namespace())).is_true();
    assert_that!(deck.slides.iter().all(|sl| kinds::essentiel::ALL.contains(&sl.kind.as_str()))).is_true();
}
