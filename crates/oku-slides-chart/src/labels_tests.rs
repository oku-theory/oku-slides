use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::ChartLabels;

#[test]
fn labels_english_numbers_series_from_one() {
    let l = ChartLabels::english();
    assert_that!(l.series(0)).is_equal_to("Series 1".to_owned());
    assert_that!(l.series(11)).is_equal_to("Series 12".to_owned());
    assert_that!(l.x_of("Sales")).is_equal_to("Sales (x)".to_owned());
    assert_that!(l.category()).is_equal_to("Category");
}

#[test]
fn labels_french_come_from_the_localizer() {
    let l = ChartLabels::new(&Localizer::new(Some("fr")));
    assert_that!(l.series(1)).is_equal_to("Série 2".to_owned());
    assert_that!(l.category()).is_equal_to("Catégorie");
    assert_that!(l.value()).is_equal_to("Valeur");
    assert_that!(l.name()).is_equal_to("Nom");
    assert_that!(l.axis()).is_equal_to("Axe");
}
