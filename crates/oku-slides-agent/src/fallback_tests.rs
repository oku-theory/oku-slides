use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::config::{AgentPreset, AgentSettings, Builtin, LocalEngine};
use oku_slides_platform::local_agent::{AGENT_MODEL_SIZE, Target};
use rstest::rstest;
use speculoos::prelude::*;

use super::{Facts, Move, Standby, TurnRoute, decide};
use crate::choice::AgentChoice;
use crate::choice::choice_fallback::{RETRY_MAX, next_wait};
use crate::error::{Reason, Unusable};
use crate::link::FAILURE_TTL;
use crate::voice_port::AppVoice;

/// A test voice whose `main` agent runs `command`, with a built-in `local` preset.
struct T {
    voice: Arc<AppVoice>,
    data: PathBuf,
    _rt: tokio::runtime::Runtime,
    _dir: tempfile::TempDir,
}

impl T {
    /// `agent.selected` as written in `config.toml`, if it was.
    fn saved_agent(&self) -> Option<String> {
        let toml = std::fs::read_to_string(self.data.join("config.toml")).unwrap_or_default();
        toml.lines().find_map(|l| l.strip_prefix("selected = \"")).map(|v| v.trim_end_matches('"').to_owned())
    }

    fn route(&self, tab: &str) -> TurnRoute {
        self.voice.turn_route(tab)
    }

    fn link_preset(&self) -> Option<String> {
        self.voice.choice.link_preset()
    }
}

/// How the test platform's local agent is set up.
#[derive(Clone, Copy)]
enum Local {
    /// The fake engine: ready (debug builds only).
    #[cfg(debug_assertions)]
    Ready,
    /// Gguf without its model: a download on Apple silicon, unsupported elsewhere.
    NoModel,
    /// No app binary: the local agent cannot run.
    NoProgram,
}

/// A voice whose selected agent `selected` is `main` (running `command`) or the built-in `local`.
fn build_selected(command: &str, local: Local, selected: &str) -> T {
    let dir = tempfile::Builder::new().prefix("oku-fallback-").tempdir().unwrap();
    let data = dir.path().to_path_buf();
    let mut settings = oku_slides_session::Settings::defaults().unwrap();
    settings.paths.data = Some(data.clone());
    settings.paths.decks = Some(data.join("decks"));
    settings.models.hub_url = "http://127.0.0.1:9".into();
    let presets = [
        ("main".to_owned(), AgentPreset::new("Main", command, vec![])),
        ("local".to_owned(), AgentPreset::builtin("Local", Builtin::Local)),
    ]
    .into_iter()
    .collect();
    settings.agent = AgentSettings::new(selected, presets).unwrap();
    settings.i18n.locale = Some("fr".into());
    settings.stt.apple = false;
    settings.local_agent.engine = match local {
        #[cfg(debug_assertions)]
        Local::Ready => LocalEngine::Fake,
        Local::NoModel | Local::NoProgram => LocalEngine::Gguf,
    };
    let platform = oku_slides_session::Platform::with_settings(settings).unwrap();
    let platform = match local {
        Local::NoProgram => platform,
        _ => platform.with_program(PathBuf::from("/app")),
    };
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let preset = platform.settings.agent.active().clone();
    let link = crate::Link::for_preset(selected, AgentChoice::launch_of(&preset, &platform), None);
    let choice = Arc::new(AgentChoice::new(&platform, link));
    T { voice: AppVoice::new(None, choice, rt.handle().clone()), data, _rt: rt, _dir: dir }
}

fn build(command: &str, local: Local) -> T {
    build_selected(command, local, "main")
}

const MISSING: &str = "/nonexistent/oku-slides-main-agent";

fn missing() -> Reason {
    Unusable::NotFound { command: MISSING.to_owned() }.into()
}

/// What the local agent without its model says on this machine.
fn without_model() -> Unusable {
    match Target::current() {
        Target::AppleSilicon => Unusable::LocalNeedsDownload { size: AGENT_MODEL_SIZE },
        Target::IntelMac => Unusable::LocalIntelMac,
        Target::Other => Unusable::LocalUnsupportedOs,
    }
}

/// An executable file at `path`: the selected agent's program shows up.
fn create_program(path: &Path) {
    std::fs::write(path, b"#!/bin/sh\n").unwrap();
}

fn facts(main: bool, local: Standby, standing: bool, alone: bool) -> Facts {
    Facts { main: main.then(missing), local, standing: standing.then(missing), alone }
}

#[rstest]
#[case::main_usable(facts(false, Standby::Ready, false, true), TurnRoute::Main, Move::Stay)]
#[case::back_to_main(facts(false, Standby::Absent, true, true), TurnRoute::Main, Move::ToMain)]
#[case::back_waits_for_other_tabs(facts(false, Standby::Absent, true, false), TurnRoute::Local { reason: missing() }, Move::Stay)]
#[case::falls_back(facts(true, Standby::Ready, false, true), TurnRoute::Local { reason: missing() }, Move::ToLocal)]
#[case::keeps_falling_back(facts(true, Standby::Ready, true, false), TurnRoute::Local { reason: missing() }, Move::Stay)]
#[case::no_switch_while_another_tab_prompts(facts(true, Standby::Ready, false, false), TurnRoute::Main, Move::Stay)]
#[case::no_fallback(facts(true, Standby::Absent, false, true), TurnRoute::Main, Move::Stay)]
#[case::needs_download(
    facts(true, Standby::Blocked(Unusable::LocalNeedsDownload { size: 7 }), false, true),
    TurnRoute::NeedsDownload { reason: missing(), size: 7 },
    Move::Stay
)]
#[case::unusable(
    facts(true, Standby::Blocked(Unusable::LocalIntelMac), false, true),
    TurnRoute::Unusable { main: missing(), local: Unusable::LocalIntelMac },
    Move::Stay
)]
fn decide_facts_give_route_and_move(#[case] facts: Facts, #[case] route: TurnRoute, #[case] moved: Move) {
    assert_that!(decide(facts)).is_equal_to((route, moved));
}

#[test]
fn turn_route_main_usable_is_main() {
    let t = build("true", Local::NoModel);
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
    assert_that!(t.link_preset()).is_equal_to(Some("main".to_owned()));
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_main_missing_local_ready_is_local() {
    let t = build(MISSING, Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Local { reason: missing() });
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_falling_back_runs_the_builtin_preset() {
    let t = build(MISSING, Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    assert_that!(t.link_preset()).is_equal_to(Some("local".to_owned()));
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_falling_back_keeps_the_selected_agent() {
    let t = build(MISSING, Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    assert_that!(t.voice.choice.active()).is_equal_to("main".to_owned());
    assert_that!(t.saved_agent()).is_none();
}

#[cfg(debug_assertions)]
#[test]
fn main_availability_while_falling_back_tells_the_selected_agent_reason() {
    let t = build(MISSING, Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    let expected = missing().localize(&Localizer::new(Some("fr")));
    assert_that!(t.voice.choice.availability()).is_equal_to(Some(expected));
}

#[test]
fn turn_route_local_without_model_needs_download_or_is_unusable() {
    let t = build(MISSING, Local::NoModel);
    let _guard = t.voice.prompt_started("t-1");
    let expected = match without_model() {
        Unusable::LocalNeedsDownload { size } => TurnRoute::NeedsDownload { reason: missing(), size },
        local => TurnRoute::Unusable { main: missing(), local },
    };
    assert_that!(t.route("t-1")).is_equal_to(expected);
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_local_failing_is_unusable_with_both_reasons() {
    let t = build(MISSING, Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    // The link runs the local agent now: its launch failed.
    t.voice.choice.link_failed(Unusable::NoHttp, Instant::now());
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Unusable { main: missing(), local: Unusable::NoHttp });
}

#[test]
fn turn_route_without_app_binary_never_falls_back() {
    let t = build(MISSING, Local::NoProgram);
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
    assert_that!(t.link_preset()).is_equal_to(Some("main".to_owned()));
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_main_usable_again_goes_back_to_main() {
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("agent");
    let t = build(&program.to_string_lossy(), Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    create_program(&program);
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
    assert_that!(t.link_preset()).is_equal_to(Some("main".to_owned()));
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_another_tab_prompting_keeps_the_main_agent() {
    let t = build(MISSING, Local::Ready);
    let _other = t.voice.prompt_started("t-2");
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
    assert_that!(t.link_preset()).is_equal_to(Some("main".to_owned()));
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_another_tab_prompting_keeps_the_local_agent() {
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("agent");
    let t = build(&program.to_string_lossy(), Local::Ready);
    let guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    drop(guard);
    create_program(&program);
    let _other = t.voice.prompt_started("t-2");
    let _guard = t.voice.prompt_started("t-1");
    let reason = Unusable::NotFound { command: program.to_string_lossy().into_owned() }.into();
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Local { reason });
    assert_that!(t.link_preset()).is_equal_to(Some("local".to_owned()));
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_selected_builtin_never_falls_back() {
    let t = build_selected(MISSING, Local::Ready, "local");
    let _guard = t.voice.prompt_started("t-1");
    t.voice.choice.link_failed(Unusable::NoHttp, Instant::now());
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
    assert_that!(t.link_preset()).is_equal_to(Some("local".to_owned()));
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_launch_failure_falls_back_until_it_is_stale() {
    let t = build("true", Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    t.voice.choice.link_failed(Unusable::NoHttp, Instant::now());
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Local { reason: Unusable::NoHttp.into() });
    assert_that!(t.voice.choice.availability()).is_some();
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_stale_launch_failure_goes_back_to_main() {
    let t = build("true", Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    let Some(old) = Instant::now().checked_sub(FAILURE_TTL.saturating_sub(Duration::from_millis(200))) else { return };
    t.voice.choice.link_failed(Unusable::NoHttp, old);
    t.route("t-1");
    std::thread::sleep(Duration::from_millis(300));
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
}

#[cfg(debug_assertions)]
#[test]
fn availability_both_unusable_tells_both_reasons() {
    let t = build(MISSING, Local::Ready);
    let guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    drop(guard);
    t.voice.choice.link_failed(Unusable::NoHttp, Instant::now());
    let expected = super::both_reasons(&missing(), &Unusable::NoHttp, &Localizer::new(Some("fr")));
    assert_that!(t.voice.availability()).is_equal_to(Some(expected));
}

#[test]
fn availability_without_app_binary_tells_the_main_reason() {
    let t = build(MISSING, Local::NoProgram);
    let expected = missing().localize(&Localizer::new(Some("fr")));
    assert_that!(t.voice.availability()).is_equal_to(Some(expected));
}

#[test]
fn availability_local_without_model_refuses_only_without_download() {
    let t = build(MISSING, Local::NoModel);
    let refused = !matches!(without_model(), Unusable::LocalNeedsDownload { .. });
    assert_that!(t.voice.availability().is_some()).is_equal_to(refused);
}

#[cfg(debug_assertions)]
#[test]
fn availability_local_ready_is_none() {
    let t = build(MISSING, Local::Ready);
    assert_that!(t.voice.availability()).is_none();
}

#[rstest]
#[case("en")]
#[case("de")]
fn both_reasons_localized_names_both(#[case] locale: &str) {
    let l = Localizer::new(Some(locale));
    let text = super::both_reasons(&missing(), &Unusable::NoProgram, &l);
    assert_that!(text.contains(&Unusable::NoProgram.localize(&l))).is_true();
    assert_that!(text.contains(&missing().localize(&l))).is_true();
}

/// A launch failure that goes stale [`FAILURE_TTL`] after the switch, in 200 ms.
fn nearly_stale() -> Option<Instant> {
    Instant::now().checked_sub(FAILURE_TTL.saturating_sub(Duration::from_millis(200)))
}

/// `t`'s selected agent failed at launch at `at`.
fn main_failed(t: &T, at: Instant) {
    t.voice.choice.link_failed(Unusable::NoHttp, at);
}

#[cfg(debug_assertions)]
#[test]
fn route_apply_another_agent_selected_since_the_read_runs_on_it() {
    let t = build(MISSING, Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    let read = t.voice.route_read();
    t.voice.choice.switch_to("local");
    let epoch = t.voice.choice.launch_epoch();
    assert_that!(t.voice.route_apply("t-1", read)).is_equal_to(TurnRoute::Main);
    assert_that!(t.voice.choice.fallback_reason()).is_none();
    assert_that!(t.voice.choice.launch_epoch()).is_equal_to(epoch);
}

#[rstest]
#[case::first_fallback(None, FAILURE_TTL)]
#[case::first_retry_failed(Some(FAILURE_TTL), FAILURE_TTL * 2)]
#[case::capped(Some(Duration::from_secs(400)), RETRY_MAX)]
#[case::stays_capped(Some(RETRY_MAX), RETRY_MAX)]
fn next_wait_retry_doubles_up_to_the_cap(#[case] retried: Option<Duration>, #[case] wait: Duration) {
    assert_that!(next_wait(retried)).is_equal_to(wait);
}

#[cfg(debug_assertions)]
#[test]
fn turn_route_failed_retry_waits_twice_as_long() {
    let t = build("true", Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    let Some(at) = nearly_stale() else { return };
    main_failed(&t, at);
    t.route("t-1");
    std::thread::sleep(Duration::from_millis(300));
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
    // The retry fails as the first launch did.
    let Some(at) = nearly_stale() else { return };
    main_failed(&t, at);
    t.route("t-1");
    std::thread::sleep(Duration::from_millis(300));
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Local { reason: Unusable::NoHttp.into() });
}

#[cfg(debug_assertions)]
#[rstest]
#[case::served(true)]
#[case::chosen_again(false)]
fn turn_route_retry_served_or_agent_chosen_resets_the_wait(#[case] served: bool) {
    let t = build("true", Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    let Some(at) = nearly_stale() else { return };
    main_failed(&t, at);
    t.route("t-1");
    std::thread::sleep(Duration::from_millis(300));
    t.route("t-1");
    if served {
        t.voice.main_served();
    } else {
        t.voice.choice.switch_to("main");
    }
    let Some(at) = nearly_stale() else { return };
    main_failed(&t, at);
    t.route("t-1");
    std::thread::sleep(Duration::from_millis(300));
    assert_that!(t.route("t-1")).is_equal_to(TurnRoute::Main);
}

#[cfg(debug_assertions)]
#[test]
fn switch_to_the_selected_agent_while_falling_back_retries_it() {
    let t = build("true", Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    main_failed(&t, Instant::now());
    t.route("t-1");
    t.voice.choice.switch_to("main");
    assert_that!(t.link_preset()).is_equal_to(Some("main".to_owned()));
    assert_that!(t.voice.choice.fallback_reason()).is_none();
}

#[cfg(debug_assertions)]
#[test]
fn switch_to_the_running_fallback_preset_keeps_the_link() {
    let t = build("true", Local::Ready);
    let _guard = t.voice.prompt_started("t-1");
    main_failed(&t, Instant::now());
    t.route("t-1");
    let epoch = t.voice.choice.launch_epoch();
    t.voice.choice.switch_to("local");
    assert_that!(t.voice.choice.launch_epoch()).is_equal_to(epoch);
    assert_that!(t.voice.choice.active()).is_equal_to("local".to_owned());
    assert_that!(t.voice.choice.fallback_reason()).is_none();
}

#[cfg(debug_assertions)]
#[test]
fn diagnosis_while_falling_back_tells_the_selected_agent_reason_and_stderr() {
    let t = build("true", Local::Ready);
    let tail = t.voice.choice.link_stderr();
    let launch = tail.begin();
    tail.push(launch, "main: boom");
    let Some(at) = nearly_stale() else { return };
    main_failed(&t, at);
    let _guard = t.voice.prompt_started("t-1");
    t.route("t-1");
    let launch = tail.begin();
    tail.push(launch, "local: hello");
    // The failure kept is stale now: the selected agent's reason still tells why it stands in.
    std::thread::sleep(Duration::from_millis(300));
    let diagnosis = t.voice.choice.diagnosis();
    assert_that!(diagnosis.stderr).is_equal_to(vec!["main: boom".to_owned()]);
    let expected = Reason::from(Unusable::NoHttp).localize(&Localizer::new(Some("fr")));
    assert_that!(diagnosis.unusable).is_equal_to(Some(expected));
}
