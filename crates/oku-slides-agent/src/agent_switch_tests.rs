use oku_slides_mcp::{Origin, VoiceApplied, VoicePort};
use oku_slides_session::VoiceEvent;
use rstest::rstest;
use speculoos::prelude::*;

use crate::agent_switch::Switch;
use crate::voice_port::applied_of;
use crate::voice_port::voice_port_tests::{T, voice_with_stt};

fn select(id: &str) -> VoiceEvent {
    VoiceEvent::SelectAgent { id: id.into() }
}

fn agent(tab: &str) -> Origin {
    Origin::Agent { tab: tab.into(), confirmed: true }
}

/// Une requête d'agent en attente pendant que `t-1` tourne ; rend la garde et l'époque.
fn pending_bis(t: &T) -> (crate::agent_switch::PromptGuard, u64) {
    let guard = t.voice.prompt_started("t-1");
    t.voice.apply(select("bis"), agent("t-1")).unwrap();
    (guard, t.voice.switch_epoch())
}

#[test]
fn select_agent_from_agent_while_prompting_waits() {
    let t = voice_with_stt();
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(t.voice.apply(select("bis"), agent("t-1")).map_err(|e| e.kind()))
        .is_ok_containing(VoiceApplied::WhenIdle);
}

#[test]
fn select_agent_from_agent_with_nothing_running_applies_now() {
    let t = voice_with_stt();
    assert_that!(t.voice.apply(select("bis"), agent("t-1")).map_err(|e| e.kind())).is_ok_containing(VoiceApplied::Now);
}

#[test]
fn select_agent_while_prompting_is_shown_as_pending() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    assert_that!(t.voice.view().agents.pending).is_equal_to(Some("bis".to_owned()));
}

#[test]
fn view_busy_while_any_tab_prompts() {
    let t = voice_with_stt();
    let _guard = t.voice.prompt_started("t-2");
    assert_that!(t.voice.view().agents.busy).is_true();
}

#[test]
fn prompt_start_bumps_the_revision() {
    let t = voice_with_stt();
    let rx = t.voice.revision();
    let before = *rx.borrow();
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(*rx.borrow()).is_greater_than(before);
}

#[test]
fn second_prompt_does_not_bump_the_revision() {
    let t = voice_with_stt();
    let _first = t.voice.prompt_started("t-1");
    let rx = t.voice.revision();
    let before = *rx.borrow();
    let _second = t.voice.prompt_started("t-2");
    assert_that!(*rx.borrow()).is_equal_to(before);
}

#[test]
fn pending_agent_stays_pending_while_another_tab_prompts() {
    let t = voice_with_stt();
    let _other = t.voice.prompt_started("t-2");
    let (guard, _) = pending_bis(&t);
    drop(guard);
    assert_that!(t.voice.view().agents.pending).is_equal_to(Some("bis".to_owned()));
}

#[test]
fn pending_agent_applies_when_the_last_prompt_ends() {
    let t = voice_with_stt();
    let (guard, _) = pending_bis(&t);
    drop(guard);
    t.until("bis actif", |t| t.voice.view().agents.active == "bis");
}

#[test]
fn applied_pending_agent_is_saved() {
    let t = voice_with_stt();
    let (guard, _) = pending_bis(&t);
    drop(guard);
    t.until("bis enregistré", |t| t.saved_agent().as_deref() == Some("bis"));
}

#[test]
fn switch_with_an_old_epoch_after_a_user_click_changes_nothing() {
    let t = voice_with_stt();
    let (guard, epoch) = pending_bis(&t);
    t.voice.apply(select("faux"), Origin::User).unwrap();
    // Plus rien ne tourne : seule l'époque peut arrêter le changement.
    drop(guard);
    t.voice.switch_now("t-1", "bis", epoch);
    assert_that!(t.voice.view().agents.active).is_equal_to("faux".to_owned());
}

#[test]
fn switch_with_an_old_epoch_saves_nothing() {
    let t = voice_with_stt();
    let (guard, epoch) = pending_bis(&t);
    t.voice.apply(select("faux"), Origin::User).unwrap();
    // Plus rien ne tourne : seule l'époque peut arrêter le changement.
    drop(guard);
    t.voice.switch_now("t-1", "bis", epoch);
    assert_that!(t.saved_agent()).is_not_equal_to(Some("bis".to_owned()));
}

#[test]
fn switch_with_an_old_epoch_does_not_bring_back_the_pending_choice() {
    let t = voice_with_stt();
    let (guard, epoch) = pending_bis(&t);
    t.voice.apply(select("faux"), Origin::User).unwrap();
    // La garde reste vivante : sans la règle de l'époque, le changement repasserait en attente.
    let _outcome = t.voice.switch_now("t-1", "bis", epoch);
    let pending = t.voice.view().agents.pending;
    drop(guard);
    assert_that!(pending).is_none();
}

#[rstest]
#[case(Switch::Go, VoiceApplied::Now)]
#[case(Switch::Wait, VoiceApplied::WhenIdle)]
#[case(Switch::Stale, VoiceApplied::Unchanged)]
fn applied_of_a_switch_says_its_outcome(#[case] outcome: Switch, #[case] applied: VoiceApplied) {
    assert_that!(applied_of(outcome)).is_equal_to(applied);
}

#[test]
fn switch_while_a_prompt_runs_keeps_the_choice_pending() {
    let t = voice_with_stt();
    let (_guard, epoch) = pending_bis(&t);
    t.voice.switch_now("t-1", "bis", epoch);
    let agents = t.voice.view().agents;
    assert_that!((agents.active.as_str(), agents.pending)).is_equal_to(("faux", Some("bis".to_owned())));
}

#[test]
fn user_select_drops_a_pending_one() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    t.voice.apply(select("faux"), Origin::User).unwrap();
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn user_select_of_the_active_agent_does_not_restart_it() {
    let t = voice_with_stt();
    let launches = t.voice.link_epoch();
    t.voice.apply(select("faux"), Origin::User).unwrap();
    assert_that!(t.voice.link_epoch()).is_equal_to(launches);
}

#[test]
fn agent_request_for_the_active_one_clears_the_pending() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    t.voice.apply(select("faux"), agent("t-2")).unwrap();
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn closing_the_requesting_tab_drops_its_pending_agent() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    t.voice.tab_closed("t-1");
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn prompt_end_after_tab_closed_releases_its_guard() {
    let t = voice_with_stt();
    let guard = t.voice.prompt_started("t-1");
    t.voice.tab_closed("t-1");
    drop(guard);
    assert_that!(t.voice.view().agents.busy).is_false();
}

#[test]
fn shutdown_drops_the_pending_agent() {
    let t = voice_with_stt();
    let (guard, _) = pending_bis(&t);
    t.voice.shutdown();
    drop(guard);
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn switch_after_shutdown_changes_nothing() {
    let t = voice_with_stt();
    let (guard, epoch) = pending_bis(&t);
    t.voice.shutdown();
    // Plus rien ne tourne : seul l'arrêt peut empêcher le changement.
    drop(guard);
    t.voice.switch_now("t-1", "bis", epoch);
    assert_that!(t.voice.view().agents.active).is_equal_to("faux".to_owned());
}
