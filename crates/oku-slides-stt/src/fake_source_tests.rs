use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{fresh_dir, wait_until};

/// Appends `text` to `path`, as the e2e helper does.
fn append(path: &Path, text: &str) {
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// A dictation whose listening reads `dir/said.txt`: never the microphone nor the hub.
fn fake_stt(dir: &Path) -> (Stt, PathBuf) {
    let file = dir.join("said.txt");
    let mut settings = Settings::defaults().unwrap();
    settings.paths.models = Some(dir.join("models"));
    settings.i18n.locale = Some("fr".into());
    "http://127.0.0.1:9".clone_into(&mut settings.models.hub_url);
    settings.stt.apple = false;
    settings.stt.fake_utterances = Some(file.clone());
    (Stt::new(&Platform::with_settings(settings).unwrap()), file)
}

/// `stt` listening, by hand: no reader thread races the test's own reads.
fn listening_by_hand(stt: &Stt) -> u64 {
    let mut g = stt.state();
    g.state = State::Listening;
    g.session
}

#[test]
fn fake_source_starts_at_end_of_file() {
    let dir = fresh_dir("fake-source-end");
    let (stt, file) = fake_stt(dir.path());
    append(&file, "déjà là\n");
    let mut source = FakeSource::at_end(&stt.storage, file.clone());
    append(&file, "ajoute une slide\n");
    assert_that!(source.read_lines(&stt.storage)).is_equal_to(vec!["ajoute une slide".to_owned()]);
    assert_that!(source.read_lines(&stt.storage)).is_empty();
}

#[test]
fn fake_source_partial_line_waits() {
    let dir = fresh_dir("fake-source-partial");
    let (stt, file) = fake_stt(dir.path());
    let mut source = FakeSource::at_end(&stt.storage, file.clone());
    append(&file, "bon");
    assert_that!(source.read_lines(&stt.storage)).is_empty();
    append(&file, "jour\n");
    assert_that!(source.read_lines(&stt.storage)).is_equal_to(vec!["bonjour".to_owned()]);
}

#[test]
fn fake_source_empty_line_dropped() {
    let dir = fresh_dir("fake-source-empty");
    let (stt, file) = fake_stt(dir.path());
    let mut source = FakeSource::at_end(&stt.storage, file.clone());
    append(&file, "\n   \n stop \n");
    assert_that!(source.read_lines(&stt.storage)).is_equal_to(vec!["stop".to_owned()]);
}

#[test]
fn fake_source_suspended_line_dropped() {
    let dir = fresh_dir("fake-source-suspended");
    let (stt, file) = fake_stt(dir.path());
    let rx = stt.utterances().unwrap();
    let session = listening_by_hand(&stt);
    let mut source = FakeSource::at_end(&stt.storage, file.clone());
    let guard = stt.suspend();
    append(&file, "perdu\n");
    assert_that!(tick(&stt.inner, &stt.storage, &mut source, session)).is_true();
    drop(guard);
    append(&file, "gardé\n");
    assert_that!(tick(&stt.inner, &stt.storage, &mut source, session)).is_true();
    assert_that!(rx.try_recv().map(|u| u.text)).is_equal_to(Ok("gardé".to_owned()));
    assert_that!(rx.try_recv().is_err()).is_true();
}

#[test]
fn fake_source_tick_after_stop_ends_the_reader() {
    let dir = fresh_dir("fake-source-stop");
    let (stt, file) = fake_stt(dir.path());
    let session = listening_by_hand(&stt);
    let mut source = FakeSource::at_end(&stt.storage, file);
    stt.stop();
    assert_that!(tick(&stt.inner, &stt.storage, &mut source, session)).is_false();
}

#[test]
fn fake_source_listen_hears_appended_line_without_microphone() {
    let dir = fresh_dir("fake-source-listen");
    let (stt, file) = fake_stt(dir.path());
    let rx = stt.utterances().unwrap();
    stt.listen(Vec::new());
    wait_until(&stt, "listening", |s| s.state == State::Listening);
    append(&file, "ajoute une diapositive titre\n");
    let heard = rx.recv_timeout(Duration::from_secs(30)).unwrap();
    assert_that!(heard.text).is_equal_to("ajoute une diapositive titre".to_owned());
    stt.stop();
    assert_that!(stt.status().state).is_equal_to(State::Idle);
}

#[test]
fn fake_source_line_appended_before_listening_is_published_is_heard() {
    let dir = fresh_dir("fake-source-measure");
    let (stt, file) = fake_stt(dir.path());
    let rx = stt.utterances().unwrap();
    // The end is measured first; a line appended before `Listening` shows is still new.
    let source = FakeSource::at_end(&stt.storage, file.clone());
    append(&file, "tout de suite\n");
    stt.listen_with(source, State::Idle);
    let heard = rx.recv_timeout(Duration::from_secs(30)).unwrap();
    assert_that!(heard.text).is_equal_to("tout de suite".to_owned());
    stt.stop();
}

#[test]
fn fake_source_reader_panic_ends_the_thread_cleanly() {
    let ended = reader_loop(Duration::ZERO, || panic!("lecture"));
    assert_that!(ended).is_equal_to(ReaderEnd::Panicked);
    let mut ticks = 0;
    let ended = reader_loop(Duration::ZERO, || {
        ticks += 1;
        ticks < 3
    });
    assert_that!((ended, ticks)).is_equal_to((ReaderEnd::Ended, 3));
}
