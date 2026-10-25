use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use oku_slides_core::PackKind;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{PackWidgets, RenderErrorKind};
use speculoos::prelude::*;

use super::*;
use crate::protocol::{Canary, Request, Response};
use crate::test_packs::{LOOP_ON_IMPORT, files, ns, tiny_deck};

fn settings(timeout_ms: u64) -> RenderSettings {
    RenderSettings {
        timeout_ms,
        probe_budget_ms: timeout_ms,
        memory_mib: 4096,
        max_frame_mib: 64,
        max_widget_kib: 256,
        max_widgets: 64,
    }
}

fn looping_probe() -> Request {
    Request::ProbeFile { kind: PackKind::Widgets, name: "boucle".into(), source: LOOP_ON_IMPORT.into(), common: None }
}

fn looping_pack() -> WirePack {
    WirePack {
        id: "boucle".into(),
        namespace: "boucle".into(),
        kind: PackKind::Widgets,
        files: BTreeMap::from([("boucle".into(), LOOP_ON_IMPORT.into())]),
    }
}

#[test]
fn link_in_process_answers_open_slot() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(10_000));
    let (resp, _) = link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    assert_that!(matches!(resp, Response::Ok)).is_true();
    assert_that!(link.restarts).is_equal_to(0);
}

#[test]
fn link_in_process_probe_that_loops_times_out_and_restarts() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(300));
    let started = Instant::now();
    let err = link.call(&looping_probe(), &[]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_secs(2));
    // La requête suivante relance un travailleur neuf.
    link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    assert_that!(link.restarts).is_equal_to(1);
}

#[test]
fn link_replays_open_slots_after_restart() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(300));
    link.call(&Request::OpenSlot { slot: 7, fonts: vec![] }, &[]).unwrap();
    link.state.slots.insert(7, SlotState { fonts: vec![], packs: vec![], last_assets: vec![], theme: None });
    let _ = link.call(&looping_probe(), &[]);
    // Marge : seul le rejeu est testé ici, pas la durée du rendu.
    link.settings.timeout_ms = 30_000;
    let deck = Box::new(tiny_deck());
    let (resp, _) = link.call(&Request::Render { slot: 7, deck, assets: vec![] }, &[]).unwrap();
    assert_that!(matches!(resp, Response::Compiled(_))).is_true();
    assert_that!(link.restarts).is_equal_to(1);
}

#[test]
fn link_replay_skips_a_faulty_pack() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(10_000));
    let pack = looping_pack();
    link.faulty_packs.insert(fingerprint_hex(&pack));
    link.state.slots.insert(3, SlotState { fonts: vec![], packs: vec![pack], last_assets: vec![], theme: None });
    // Sans l'exclusion, le rejeu du pack qui boucle dépasserait le délai.
    let (resp, _) = link.call(&Request::CloseSlot { slot: 3 }, &[]).unwrap();
    assert_that!(matches!(resp, Response::Ok)).is_true();
}

#[test]
fn link_first_launch_replays_language_and_fonts() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(10_000));
    link.state.locale = Some("fr".into());
    link.state.fonts.insert("f1".into(), Arc::from(b"not a font".to_vec()));
    link.state.slots.insert(2, SlotState { fonts: vec!["f1".into()], packs: vec![], last_assets: vec![], theme: None });
    link.call(&Request::Release { slot: 2, generations: vec![] }, &[]).unwrap();
    assert_that!(link.sent_fonts.contains("f1")).is_true();
    assert_that!(link.restarts).is_equal_to(0);
}

#[test]
fn link_error_response_is_not_a_link_failure() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(10_000));
    let (resp, _) = link.call(&Request::OpenSlot { slot: 1, fonts: vec!["inconnue".into()] }, &[]).unwrap();
    assert_that!(matches!(resp, Response::Error { .. })).is_true();
    link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    assert_that!(link.restarts).is_equal_to(0);
}

#[test]
fn fingerprint_hex_matches_pack_fingerprint() {
    let pack =
        PackWidgets::new("p", ns("p"), PackKind::Layouts, files(&[("_commun", "#let c = 1"), ("quote", "Q")])).unwrap();
    let wire = WirePack {
        id: "p".into(),
        namespace: "p".into(),
        kind: PackKind::Layouts,
        files: BTreeMap::from([("_commun".into(), "#let c = 1".into()), ("quote".into(), "Q".into())]),
    };
    assert_that!(fingerprint_hex(&wire)).is_equal_to(hex::encode(pack.fingerprint()));
}

#[test]
fn link_replay_with_missing_font_fails_then_reconnects() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(10_000));
    link.state
        .slots
        .insert(4, SlotState { fonts: vec!["absente".into()], packs: vec![], last_assets: vec![], theme: None });
    let err = link.call(&Request::Release { slot: 4, generations: vec![] }, &[]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
    link.state.slots.remove(&4);
    link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    assert_that!(link.restarts).is_equal_to(1);
}

#[test]
fn link_replay_refused_by_worker_drops_the_half_replayed_worker() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(10_000));
    let bad =
        WirePack { id: "pas un id/".into(), namespace: "p".into(), kind: PackKind::Widgets, files: BTreeMap::new() };
    link.state.slots.insert(5, SlotState { fonts: vec![], packs: vec![bad], last_assets: vec![], theme: None });
    assert_that!(link.call(&Request::Release { slot: 5, generations: vec![] }, &[])).is_err();
    link.state.slots.get_mut(&5).unwrap().packs.clear();
    // Un travailleur neuf : l'emplacement 5 y est rejoué, donc rendable.
    let deck = Box::new(tiny_deck());
    let (resp, _) = link.call(&Request::Render { slot: 5, deck, assets: vec![] }, &[]).unwrap();
    assert_that!(matches!(resp, Response::Compiled(_))).is_true();
    assert_that!(link.restarts).is_equal_to(1);
}

#[test]
fn frame_failure_of_dead_worker_is_worker_lost_and_bad_frame_is_frame() {
    use snafu::ResultExt;

    use crate::error::{IoSnafu, TooLargeSnafu, TruncatedSnafu};
    let io = Err::<(), _>(std::io::Error::other("rompu")).context(IoSnafu).unwrap_err();
    let pdf = Request::Pdf { slot: 1, generation: 1 };
    assert_that!(frame_failure(&io, &pdf).kind()).is_equal_to(RenderErrorKind::WorkerLost);
    assert_that!(frame_failure(&TruncatedSnafu.build(), &pdf).kind()).is_equal_to(RenderErrorKind::WorkerLost);
    assert_that!(frame_failure(&TooLargeSnafu { len: 2usize, max: 1usize, payload: true }.build(), &pdf).kind())
        .is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn link_outgoing_frame_over_max_is_refused_as_frame_and_keeps_the_worker() {
    let mut link = Link::new(
        RenderLaunch::InProcess,
        RenderSettings {
            timeout_ms: 10_000,
            probe_budget_ms: 10_000,
            memory_mib: 4096,
            max_frame_mib: 1,
            max_widget_kib: 256,
            max_widgets: 64,
        },
    );
    link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    let big = vec![0u8; 1024 * 1024 + 1];
    let err = link.call(&Request::PutFont { id: "grosse".into() }, &big).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
    // Rien n'a été écrit : le même travailleur répond encore.
    assert_that!(link.connected()).is_true();
    link.call(&Request::OpenSlot { slot: 2, fonts: vec![] }, &[]).unwrap();
    assert_that!(link.restarts).is_equal_to(0);
}

#[test]
fn link_outgoing_header_over_an_eighth_of_the_frame_size_is_refused_as_frame_and_keeps_the_worker() {
    let mut link = Link::new(RenderLaunch::InProcess, RenderSettings { max_frame_mib: 1, ..settings(10_000) });
    link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    // Un en-tête de plus de 128 Kio, sous la taille de trame (1 Mio).
    let req =
        Request::ProbeFile { kind: PackKind::Widgets, name: "w".into(), source: "x".repeat(200 * 1024), common: None };
    let err = link.call(&req, &[]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
    assert_that!(link.connected()).is_true();
    link.call(&Request::OpenSlot { slot: 2, fonts: vec![] }, &[]).unwrap();
    assert_that!(link.restarts).is_equal_to(0);
}

#[test]
fn svg_payload_over_its_limit_says_the_slide_is_too_large_to_preview() {
    use oku_slides_i18n::{Localize, Localizer};

    use crate::error::TooLargeSnafu;
    let l = Localizer::new(Some("en"));
    let svg = Request::Svg { slot: 1, generation: 1, page: 0 };
    let too_large = |payload| TooLargeSnafu { len: 300usize << 20, max: 128usize << 20, payload }.build();
    let e = frame_failure(&too_large(true), &svg);
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::Frame);
    assert_that!(e.localize(&l)).is_equal_to(l.tr_with("render-page-too-large", &[("mib", 128u64.into())]));
    // Un en-tête trop grand reste un message illisible.
    assert_that!(frame_failure(&too_large(false), &svg).localize(&l)).is_equal_to(l.tr("render-frame"));
}

#[test]
fn worker_command_passes_bounds_and_marks_the_child_as_a_worker() {
    let s = RenderSettings {
        timeout_ms: 1_500,
        probe_budget_ms: 1_500,
        memory_mib: 2,
        max_frame_mib: 1,
        max_widget_kib: 256,
        max_widgets: 64,
    };
    let cmd = worker_command(Path::new("/bin/app"), &s, None);
    let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    assert_that!(args).is_equal_to(
        ["--render-worker", "--max-frame-bytes", "1048576", "--timeout-ms", "1500", "--memory-bytes", "2097152"]
            .map(str::to_owned)
            .to_vec(),
    );
    let env: Vec<_> = cmd.get_envs().collect();
    assert_that!(env)
        .contains((std::ffi::OsStr::new(oku_slides_platform::RENDER_WORKER_ENV), Some(std::ffi::OsStr::new("1"))));
}

#[rstest::rstest]
#[case(Response::Hello { version: PROTOCOL_VERSION + 1, canary: None })]
#[case::protocol_3(Response::Hello { version: 3, canary: None })]
#[case::protocol_4(Response::Hello { version: 4, canary: None })]
#[case(Response::Error { error: oku_slides_render::WireError::Frame { message: "unknown op".into() } })]
#[case(Response::Ok)]
fn check_hello_of_another_protocol_is_a_protocol_error(#[case] reply: Response) {
    let e = check_hello(&reply, None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::Protocol);
}

#[rstest::rstest]
#[case::no_canary_asked(None, None, true)]
#[case::denied(Some("/c"), Some(Canary::Denied), true)]
#[case::written(Some("/c"), Some(Canary::Written), false)]
#[case::failed(Some("/c"), Some(Canary::Failed), false)]
#[case::not_answered(Some("/c"), None, false)]
fn check_hello_with_a_canary_requires_it_denied(
    #[case] asked: Option<&str>,
    #[case] seen: Option<Canary>,
    #[case] ok: bool,
) {
    let reply = Response::Hello { version: PROTOCOL_VERSION, canary: seen };
    let result = check_hello(&reply, asked.map(Path::new));
    assert_that!(result.is_ok()).is_equal_to(ok);
    if let Err(e) = result {
        assert_that!(e.kind()).is_equal_to(RenderErrorKind::WorkerLost);
    }
}

#[test]
fn worker_command_without_sandbox_runs_the_program() {
    let cmd = worker_command(Path::new("/bin/w"), &settings(30_000), None);
    assert_that!(cmd.get_program().to_owned()).is_equal_to(std::ffi::OsString::from("/bin/w"));
}

#[cfg(target_os = "macos")]
#[test]
fn worker_command_with_sandbox_runs_sandbox_exec_with_the_work_dir() {
    let sandbox = crate::sandbox::Sandbox::new(Path::new("/bin/bash"), None).unwrap();
    let cmd = worker_command(Path::new("/bin/bash"), &settings(30_000), Some(&sandbox));
    assert_that!(cmd.get_program().to_owned()).is_equal_to(std::ffi::OsString::from("/usr/bin/sandbox-exec"));
    let args: Vec<_> = cmd.get_args().map(std::borrow::ToOwned::to_owned).collect();
    let i = args.iter().position(|a| a == "--work-dir").unwrap();
    assert_that!(std::path::PathBuf::from(&args[i + 1])).is_equal_to(sandbox.work().to_owned());
}
