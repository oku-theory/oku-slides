use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::{NoTabs, Prefill, TabPort, TuneRefusal};

#[rstest]
#[case::unavailable(TuneRefusal::unavailable(), "agent-error-tune-unavailable")]
#[case::not_started(TuneRefusal::not_started(), "agent-error-tune-not-started")]
fn tune_refusal_key_is_translated_in_french(#[case] refusal: TuneRefusal, #[case] key: &str) {
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    assert_that!(refusal.key()).is_equal_to(key);
    assert_that!(fr.has(key)).is_true();
    assert_that!(refusal.localize(&fr)).is_not_equal_to(en.tr(key));
}

#[test]
fn no_tabs_tune_refuses_unavailable() {
    assert_that!(NoTabs.tune("t-2").err().map(|e| e.key())).is_equal_to(Some("agent-error-tune-unavailable"));
}

#[test]
fn no_tabs_prefill_returns_unavailable() {
    assert_that!(NoTabs.prefill("t-2", "x".to_owned())).is_equal_to(Prefill::Unavailable);
}

#[test]
fn no_tabs_page_returns_false() {
    assert_that!(NoTabs.page("t-2", "s-1".to_owned())).is_false();
}
