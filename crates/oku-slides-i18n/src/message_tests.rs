use speculoos::prelude::*;

use super::*;

#[test]
fn message_with_args_localizes_and_displays_in_english() {
    let m = Message::new("chart-series-empty").arg("index", 2usize);
    assert_that!(m.localize(&Localizer::new(Some("fr")))).is_equal_to("série 2 sans données".to_owned());
    assert_that!(m.to_string()).is_equal_to("series 2 has no data".to_owned());
}
