use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use speculoos::prelude::*;

use crate::test_support::{fresh_dir, platform_in};
use crate::{Mode, Model, State, Stt};

fn stt(name: &str) -> (tempfile::TempDir, Stt) {
    let dir = fresh_dir(name);
    let stt = Stt::new(&platform_in(dir.path(), "fr"));
    (dir, stt)
}

#[test]
fn last_owner_dropped_stops_dictation_while_a_worker_lives() {
    let (_dir, stt) = stt("shutdown-worker");
    let session = {
        let mut g = stt.state();
        g.state = State::Recording;
        g.session
    };
    let worker = stt.worker();
    drop(stt);
    let g = worker.state();
    assert_that!((g.state, g.session > session)).is_equal_to((State::Idle, true));
}

#[test]
fn shutdown_joins_a_running_transcriber_and_resets() {
    let (_dir, stt) = stt("shutdown-join");
    let session = {
        let mut g = stt.state();
        g.state = State::Recording;
        g.session
    };
    let (done_tx, done_rx) = mpsc::channel();
    let spawned = stt.spawn_transcriber(move || {
        std::thread::sleep(Duration::from_millis(200));
        done_tx.send(()).ok();
    });
    assert_that!(spawned).is_equal_to(true);
    stt.shutdown();
    // Joined: the thread's message is already there, no wait needed.
    assert_that!(done_rx.try_recv().is_ok()).is_true();
    let g = stt.state();
    assert_that!((g.state, g.session > session)).is_equal_to((State::Idle, true));
    assert_that!(stt.transcribers.lock().map_or(usize::MAX, |t| t.len())).is_equal_to(0);
}

#[test]
fn spawn_transcriber_forgets_finished_threads() {
    let (_dir, stt) = stt("shutdown-prune");
    assert_that!(stt.spawn_transcriber(|| {})).is_equal_to(true);
    let finished = || stt.transcribers.lock().is_ok_and(|t| t.iter().all(std::thread::JoinHandle::is_finished));
    while !finished() {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_that!(stt.spawn_transcriber(|| {})).is_equal_to(true);
    assert_that!(stt.transcribers.lock().map_or(0, |t| t.len())).is_equal_to(1);
}

#[test]
fn download_finishing_after_shutdown_starts_nothing() {
    let (_dir, stt) = stt("shutdown-download");
    {
        let mut g = stt.state();
        g.pause_for_download(Some(Mode::Dictate));
        g.state = State::Downloading;
    }
    stt.shutdown();
    assert_that!(stt.cancel_download.load(Ordering::Relaxed)).is_true();
    assert_that!(stt.finish_download(Model::Parakeet, Ok(()))).is_none();
    assert_that!(stt.state().state).is_equal_to(State::Idle);
}

#[test]
fn listening_paused_by_download_does_not_resume_after_shutdown() {
    let (_dir, stt) = stt("shutdown-listen");
    {
        let mut g = stt.state();
        g.state = State::Listening;
        g.pause_for_download(None);
        g.state = State::Downloading;
    }
    stt.shutdown();
    assert_that!(stt.finish_download(Model::Parakeet, Ok(()))).is_none();
}

#[test]
fn spawn_transcriber_after_shutdown_is_refused() {
    let (_dir, stt) = stt("shutdown-spawn");
    stt.shutdown();
    let (ran_tx, ran_rx) = mpsc::channel();
    let spawned = stt.spawn_transcriber(move || {
        ran_tx.send(()).ok();
    });
    assert_that!(spawned).is_equal_to(false);
    // The closure was dropped unrun: its sender is gone.
    assert_that!(ran_rx.recv_timeout(Duration::from_secs(1)).is_err()).is_true();
    assert_that!(stt.transcribers.lock().map_or(usize::MAX, |t| t.len())).is_equal_to(0);
}

#[test]
fn download_after_shutdown_is_refused() {
    let (_dir, stt) = stt("shutdown-fetch");
    stt.shutdown();
    assert_that!(stt.download(Model::Parakeet, false)).is_false();
    assert_that!(stt.state().state).is_equal_to(State::Idle);
}

#[test]
fn finish_download_once_state_left_downloading_starts_nothing() {
    let (_dir, stt) = stt("shutdown-left");
    {
        let mut g = stt.state();
        g.pause_for_download(Some(Mode::Dictate));
        // Something else moved the state on (a shutdown racing the download's end).
        g.state = State::Idle;
    }
    assert_that!(stt.finish_download(Model::Parakeet, Ok(()))).is_none();
    assert_that!(stt.state().start_after_download).is_none();
}
