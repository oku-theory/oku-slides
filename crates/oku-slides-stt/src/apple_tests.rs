use std::time::Duration;

use speculoos::prelude::*;

use super::*;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/stt/dictee-fr.wav");

fn fixture() -> Vec<f32> {
    let mut reader = hound::WavReader::open(FIXTURE).unwrap();
    reader.samples::<i16>().map(|s| f32::from(s.unwrap_or(0)) / 32768.0).collect()
}

/// Les événements jusqu'à `Done` (ou une erreur) ; `None` si rien ne finit.
fn until_end(events: &mpsc::Receiver<Event>) -> Option<Event> {
    loop {
        match events.recv_timeout(Duration::from_secs(10)).ok()? {
            Event::Done => return Some(Event::Done),
            Event::Failed(e) => return Some(Event::Failed(e)),
            _ => {}
        }
    }
}

#[test]
#[ignore = "macOS 26 et langue française installée : just test-apple"]
fn cancel_after_done_is_harmless() {
    let (mut stream, events) = AppleEngine.open("fr", &[]);
    stream.push(&fixture());
    stream.finish();
    assert_that!(matches!(until_end(&events), Some(Event::Done))).is_true();
    stream.cancel();
    drop(stream);
}

#[test]
#[ignore = "macOS 26 et langue française installée : just test-apple"]
fn cancel_after_finish_ends_the_stream() {
    let (mut stream, events) = AppleEngine.open("fr", &[]);
    stream.push(&fixture());
    stream.finish();
    stream.cancel();
    assert_that!(until_end(&events).is_some()).is_true();
    drop(stream);
}

#[test]
#[ignore = "macOS 26 et langue française installée : just test-apple"]
fn drop_without_finish_ends_the_stream() {
    let (mut stream, events) = AppleEngine.open("fr", &[]);
    stream.push(&fixture()[..16_000]);
    drop(stream);
    assert_that!(until_end(&events).is_some()).is_true();
}

#[test]
#[ignore = "macOS 26 et langue française installée : just test-apple"]
fn installed_french_is_ready() {
    assert_that!(AppleEngine.availability("fr")).is_equal_to(Availability::Ready);
}

#[test]
#[ignore = "macOS 26 et langue française installée : just test-apple"]
fn unknown_language_is_unsupported() {
    assert_that!(AppleEngine.availability("xx")).is_equal_to(Availability::Unsupported);
}
