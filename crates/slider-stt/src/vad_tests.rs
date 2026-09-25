use speculoos::prelude::*;

use super::*;

fn vad() -> VadSettings {
    slider_platform::config::Settings::defaults().unwrap().stt.vad
}

fn vad_max_phrase(max_phrase: f32) -> VadSettings {
    let mut v = vad();
    v.max_phrase = max_phrase;
    v
}

#[test]
fn split_silence_returns_no_phrase() {
    let silence = vec![0.0f32; SAMPLE_RATE as usize * 3];
    assert_that!(split(&silence, vad())).is_empty();
}

#[test]
fn segmenter_long_silent_listening_keeps_only_the_lead_in() {
    let mut seg = Segmenter::new(vad());
    let chunk = vec![0.0f32; 800]; // 50 ms, comme la boucle d'écoute
    for _ in 0..(60 * 20) {
        assert_that!(seg.push(&chunk)).is_empty();
    }
    asserting(&format!("{} échantillons gardés", seg.audio.len()))
        .that(&seg.audio.len())
        .is_less_than(2 * SAMPLE_RATE as usize);
    asserting("les positions restent celles de la session").that(&seg.end()).is_equal_to(60 * SAMPLE_RATE as usize);
    assert_that!((seg.silence() - 60.0).abs() < 0.1).is_true();
}

#[test]
fn segmenter_returned_phrase_is_readable_before_next_push() {
    let mut seg = Segmenter::new(vad_max_phrase(1.0));
    // Trois secondes de silence, oubliées en partie, puis une phrase coupée
    // pour sa longueur (1 s).
    seg.push(&vec![0.0; SAMPLE_RATE as usize * 3]);
    let open_at = seg.scanned;
    seg.open = Some(open_at);
    seg.last_voice = open_at;
    seg.config.pause = 100.0;
    let out = seg.push(&vec![0.25; SAMPLE_RATE as usize * 2]);
    let first = out[0];
    assert_that!(first.start).is_equal_to(open_at);
    asserting("l'audio de la phrase est encore là, entier")
        .that(&seg.slice(first).len())
        .is_equal_to(first.end - first.start);
    asserting("le silence d'avant a été oublié").that(&seg.base).is_greater_than(0);
}

#[test]
fn segmenter_too_long_phrase_is_cut() {
    let mut seg = Segmenter::new(vad_max_phrase(1.0));
    seg.open = Some(0);
    seg.last_voice = 0;
    // Pas de voix réelle ici : on vérifie seulement la coupe par la longueur.
    seg.config.pause = 100.0;
    let out = seg.push(&vec![0.0; SAMPLE_RATE as usize * 2]);
    assert_that!(out).is_not_empty();
    assert_that!(out[0].seconds()).is_less_than_or_equal_to(1.1);
}
