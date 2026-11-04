use oku_slides_platform::updates::{InstallMode, UpdateOffer, UpdateStage, UpdateStatus};
use oku_slides_session::agent::UpdateState;
use rstest::rstest;
use speculoos::prelude::*;

use super::{Asked, Gate, gate, refusal_of, state_of};

#[rstest]
#[case::update_release(Asked::Update, false)]
#[case::update_debug(Asked::Update, true)]
#[case::quit_release(Asked::Quit, false)]
#[case::quit_debug(Asked::Quit, true)]
#[case::install_release(Asked::Install, false)]
#[case::install_debug(Asked::Install, true)]
fn app_port_refuses_while_closing(#[case] asked: Asked, #[case] debug_build: bool) {
    assert_that!(gate(asked, true, debug_build)).is_equal_to(Gate::Closing);
}

#[rstest]
#[case::update(Asked::Update, Gate::NoUpdater)]
#[case::quit(Asked::Quit, Gate::Open)]
#[case::install(Asked::Install, Gate::NoUpdater)]
fn app_port_debug_build_gates_only_the_updater(#[case] asked: Asked, #[case] expected: Gate) {
    assert_that!(gate(asked, false, true)).is_equal_to(expected);
}

#[rstest]
#[case::update(Asked::Update)]
#[case::quit(Asked::Quit)]
#[case::install(Asked::Install)]
fn app_port_release_build_running_is_open(#[case] asked: Asked) {
    assert_that!(gate(asked, false, false)).is_equal_to(Gate::Open);
}

fn status(mode: InstallMode, installing: bool) -> UpdateStatus {
    UpdateStatus::new(Some(UpdateOffer { version: "5.2.0".into(), mode }), installing, None)
}

#[rstest]
#[case::install(InstallMode::Install, false, true)]
#[case::installing(InstallMode::Install, true, false)]
#[case::notify_only(InstallMode::NotifyOnly, false, false)]
fn state_of_offer_is_installable_when_the_app_installs(
    #[case] mode: InstallMode,
    #[case] installing: bool,
    #[case] installable: bool,
) {
    let expected = UpdateState::Offer { version: "5.2.0".into(), installable };
    assert_that!(state_of(&status(mode, installing))).is_equal_to(expected);
}

#[test]
fn state_of_no_offer_is_up_to_date() {
    assert_that!(state_of(&UpdateStatus::new(None, false, None))).is_equal_to(UpdateState::UpToDate);
}

#[rstest]
#[case::unavailable(UpdateStage::Unavailable, "agent-error-app-no-offer")]
#[case::refused(UpdateStage::Refused, "agent-error-app-closing")]
#[case::busy(UpdateStage::Busy, "agent-error-app-failed")]
#[case::check(UpdateStage::Check, "agent-error-app-failed")]
#[case::download(UpdateStage::Download, "agent-error-app-failed")]
fn refusal_of_stage_is_its_key(#[case] stage: UpdateStage, #[case] key: &str) {
    assert_that!(refusal_of(stage).key()).is_equal_to(key);
}
