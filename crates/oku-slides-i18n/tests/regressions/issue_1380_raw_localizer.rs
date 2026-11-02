//! t-1380: only the UI `t()` isolates string arguments; the Rust localizer stays raw, because its
//! text also reaches the agent (whose echo `check_text` refuses), the CLI and logs.

use oku_slides_i18n::{FluentValue, Localizer};
use speculoos::prelude::*;

#[test]
fn tr_with_right_to_left_argument_adds_no_isolation_marks() {
    let l = Localizer::new(Some("en"));
    let shown = l.tr_with("cli-reading", &[("path", FluentValue::from("\u{05E9}\u{05DC}\u{05D5}\u{05DD}.okus"))]);
    assert_that!(shown.contains(['\u{2068}', '\u{2069}'])).is_false();
}
