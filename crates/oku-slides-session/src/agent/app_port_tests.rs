use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::{AppPort, AppRefusal, UnavailableApp, UpdateAction, UpdateState};

#[rstest]
#[case::unavailable(AppRefusal::unavailable(), "agent-error-app-unavailable")]
#[case::closing(AppRefusal::closing(), "agent-error-app-closing")]
#[case::no_offer(AppRefusal::no_offer(), "agent-error-app-no-offer")]
#[case::failed(AppRefusal::failed(), "agent-error-app-failed")]
fn app_refusal_key_is_translated_in_french(#[case] refusal: AppRefusal, #[case] key: &str) {
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    assert_that!(refusal.key()).is_equal_to(key);
    assert_that!(fr.has(key)).is_true();
    assert_that!(refusal.localize(&fr)).is_equal_to(fr.tr(key));
    assert_that!(refusal.localize(&fr)).is_not_equal_to(en.tr(key));
}

#[rstest]
#[case::check(UpdateAction::Check)]
#[case::later(UpdateAction::Later)]
fn unavailable_app_update_is_unavailable(#[case] action: UpdateAction) {
    let state = block_on(UnavailableApp.update(action));
    assert_that!(state.ok()).is_equal_to(Some(UpdateState::Unavailable));
}

#[test]
fn unavailable_app_quit_is_refused_unavailable() {
    let refused = UnavailableApp.quit().err().map(|e| e.key());
    assert_that!(refused).is_equal_to(Some("agent-error-app-unavailable"));
}

#[test]
fn unavailable_app_install_is_refused_unavailable() {
    let refused = UnavailableApp.install().err().map(|e| e.key());
    assert_that!(refused).is_equal_to(Some("agent-error-app-unavailable"));
}

/// Runs a future that never waits, on a one-thread runtime.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread().build().unwrap_or_else(|e| panic!("test runtime: {e}")).block_on(f)
}
