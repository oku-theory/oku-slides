//! Un `ChartOption` en ligne faisait peser chaque `ElementBody` près de
//! 300 octets, texte ou image compris : clippy (`large_enum_variant`)
//! bloquait `just check`. Le graphique vit désormais dans une `Box`.

use oku_slides_core::ElementBody;
use speculoos::prelude::*;

#[test]
fn element_body_with_boxed_chart_stays_small() {
    // Les variantes hors graphique tiennent sous 80 octets ; 128 laisse de la marge.
    assert_that!(std::mem::size_of::<ElementBody>()).is_less_than_or_equal_to(128);
}
