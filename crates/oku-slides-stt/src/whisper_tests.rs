use std::path::Path;

use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;

use super::{
    MAX_PROMPT_TOKENS, Preprocessor, build_prompt, decoding_options, forbid_repetitions, language_code, repetitive,
    spoken, threads,
};
use crate::SttErrorKind;

/// Réglages réduits : 8 bandes mel, trames de 16 échantillons, tranches de 256.
fn settings(extra: &str) -> String {
    format!(
        r#"{{"feature_size":8,"hop_length":16,"n_fft":64,"n_samples":256,"nb_max_frames":16,"sampling_rate":16000{extra}}}"#
    )
}

fn small() -> Preprocessor {
    Preprocessor::parse(settings("").as_bytes()).unwrap()
}

fn tokens(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

#[test]
fn repetitive_syllable_loop_returns_true() {
    assert_that!(repetitive("Arudun, slid, sud, affaidaudududududududududududududududududududududududududududududud"))
        .is_true();
    assert_that!(repetitive("Houdun slidzu la feuille de route d'un mjmjmjmjjjjjjjjjjjj")).is_true();
}

#[test]
fn repetitive_normal_sentences_return_false() {
    assert_that!(repetitive("Ajoute une slide sur la feuille de route du deuxième trimestre, avec trois colonnes."))
        .is_false();
    assert_that!(repetitive("Ask what you can do for your country, ask what your country can do for you.")).is_false();
    assert_that!(repetitive("Il a dit : « non, non, non » et c'est tout. Rendez-vous le 11/11/2026.")).is_false();
}

#[test]
fn repetitive_long_text_of_repeated_phrases_returns_true() {
    assert_that!(repetitive(&"la feuille de route. ".repeat(12))).is_true();
}

#[test]
fn preprocessor_without_filters_computes_them() {
    let pre = small();
    assert_that!(pre.mel_filters.shape()).is_equal_to([8, 33].as_slice());
    assert_that!(pre.sampling_rate).is_equal_to(16_000);
}

#[test]
fn preprocessor_with_filters_keeps_them() {
    let pre = Preprocessor::parse(settings(r#","mel_filters":[[1,0,0],[0,1,0]]"#).as_bytes()).unwrap();
    assert_that!(pre.mel_filters.shape()).is_equal_to([2, 3].as_slice());
    assert_that!(pre.mel_filters[[1, 1]]).is_equal_to(1.0);
}

#[test]
fn preprocessor_ragged_filters_return_model_error() {
    let err = Preprocessor::parse(settings(r#","mel_filters":[[1,0,0],[0,1]]"#).as_bytes()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}

#[test]
fn preprocessor_invalid_json_returns_model_error() {
    let err = Preprocessor::parse(b"{\"feature_size\": 8}").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}

#[test]
fn preprocessor_zero_hop_length_returns_model_error() {
    let err =
        Preprocessor::parse(settings("").replace("\"hop_length\":16", "\"hop_length\":0").as_bytes()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
    assert_that!(err.to_string()).contains("hop_length");
}

#[test]
fn mel_empty_audio_returns_no_chunk() {
    let (features, chunks) = small().mel(&[]).unwrap();
    assert_that!(chunks).is_equal_to(0);
    assert_that!(features.len()).is_equal_to(0);
}

#[test]
fn mel_splits_audio_into_fixed_size_chunks() {
    let audio: Vec<f32> = (0..600).map(|i| (i as f32 / 5.0).sin()).collect();
    let (features, chunks) = small().mel(&audio).unwrap();
    // 600 échantillons en tranches de 256 : 3 tranches de 8 bandes × 16 trames.
    assert_that!(chunks).is_equal_to(3);
    assert_that!(features.shape()).is_equal_to([3, 8, 16].as_slice());
    assert_that!(features.is_standard_layout()).is_true();
    assert_that!(features.iter().all(|v| v.is_finite())).is_true();
    assert_that!(features.iter().any(|v| *v != 0.0)).is_true();
}

#[test]
fn mel_settings_promising_too_many_frames_do_not_panic() {
    // 256 / 16 = 16 trames par tranche, mais seulement 4 de place.
    let json = settings("").replace("\"nb_max_frames\":16", "\"nb_max_frames\":4");
    let pre = Preprocessor::parse(json.as_bytes()).unwrap();
    let (features, _) = pre.mel(&[0.1; 256]).unwrap();
    assert_that!(features.shape()).is_equal_to([1, 8, 4].as_slice());
}

#[test]
fn language_code_strips_special_token_marks() {
    assert_that!(language_code("<|fr|>")).is_equal_to("fr");
    assert_that!(language_code("en")).is_equal_to("en");
}

#[test]
fn build_prompt_without_context_is_the_instruction() {
    assert_that!(build_prompt(None, "fr")).is_equal_to(tokens(&[
        "<|startoftranscript|>",
        "<|fr|>",
        "<|transcribe|>",
        "<|notimestamps|>",
    ]));
}

#[test]
fn build_prompt_with_context_puts_it_first() {
    let prompt = build_prompt(Some(tokens(&["▁Oku Slides", "▁slide"])), "en");
    assert_that!(prompt[..3].to_vec()).is_equal_to(tokens(&["<|startofprev|>", "▁Oku Slides", "▁slide"]));
    assert_that!(prompt[4].as_str()).is_equal_to("<|en|>");
}

#[test]
fn build_prompt_long_context_keeps_the_last_tokens() {
    let context: Vec<String> = (0..MAX_PROMPT_TOKENS + 50).map(|i| i.to_string()).collect();
    let prompt = build_prompt(Some(context), "fr");
    assert_that!(prompt.len()).is_equal_to(1 + MAX_PROMPT_TOKENS + 4);
    assert_that!(prompt[1].as_str()).is_equal_to("50");
}

#[test]
fn decoding_options_budget_grows_with_audio_and_is_capped() {
    let one_second = decoding_options(4, 16_000, 16_000, false);
    assert_that!(one_second.max_length).is_equal_to(4 + 28);
    assert_that!(one_second.beam_size).is_equal_to(5);
    assert_that!(one_second.return_no_speech_prob).is_true();
    let long = decoding_options(4, 16_000 * 60, 16_000, true);
    assert_that!(long.max_length).is_equal_to(448);
    assert_that!(long.beam_size).is_equal_to(1);
}

#[test]
fn forbid_repetitions_switches_to_greedy_without_repeats() {
    let mut options = decoding_options(4, 16_000, 16_000, false);
    forbid_repetitions(&mut options);
    assert_that!(options.beam_size).is_equal_to(1);
    assert_that!(options.no_repeat_ngram_size).is_equal_to(3);
    assert_that!(options.repetition_penalty).is_greater_than(1.0);
}

#[test]
fn spoken_drops_special_tokens_only() {
    let seq = tokens(&["<|fr|>", "▁Bonjour", "<|", "|>", "<|endoftext|>"]);
    assert_that!(spoken(seq)).is_equal_to(tokens(&["▁Bonjour", "<|", "|>"]));
}

#[test]
fn threads_is_between_one_and_eight() {
    assert_that!(threads()).is_greater_than_or_equal_to(1);
    assert_that!(threads()).is_less_than_or_equal_to(8);
}

/// Avec un vrai modèle : `OKU_SLIDES_TEST_WHISPER_MODEL=<dossier> cargo nextest run --run-ignored all`.
#[test]
#[ignore = "demande un modèle CTranslate2 (OKU_SLIDES_TEST_WHISPER_MODEL)"]
fn transcriber_real_model_transcribes_without_error() {
    let Some(dir) = std::env::var_os("OKU_SLIDES_TEST_WHISPER_MODEL") else { return };
    let storage = Storage::local().unwrap();
    let t = super::Transcriber::load(Path::new(&dir), &storage).unwrap();
    let silence = vec![0.0f32; 16_000];
    let quick = t.transcribe(&silence, None, Some("Oku Slides"), true).unwrap();
    assert_that!(quick.language.is_some()).is_true();
    let fixed = t.transcribe(&silence, Some("fr"), None, false).unwrap();
    assert_that!(fixed.language.as_deref()).is_equal_to(Some("fr"));
    assert_that!(t.transcribe(&[], None, None, true).unwrap().text.is_empty()).is_true();
    // Une vraie phrase, si fournie : WAV mono 16 kHz 16 bits et texte attendu.
    if let (Some(wav), Some(expected)) =
        (std::env::var_os("OKU_SLIDES_TEST_WHISPER_WAV"), std::env::var("OKU_SLIDES_TEST_WHISPER_TEXT").ok())
    {
        let mut reader = hound::WavReader::open(wav).unwrap();
        let samples: Vec<f32> = reader.samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
        let heard = t.transcribe(&samples, Some("fr"), None, false).unwrap().text.to_lowercase();
        for word in expected.to_lowercase().split_whitespace() {
            asserting(&format!("« {word} » dans « {heard} »")).that(&heard.contains(word)).is_true();
        }
    }
}

fn model_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("oku-slides-whisper-{name}-{}", std::process::id()))
}

#[test]
fn read_model_lists_visible_files_through_storage() {
    let storage = Storage::local().unwrap();
    let dir = model_dir("read");
    storage.write(&dir.join("model.bin"), b"poids".to_vec()).unwrap();
    storage.write(&dir.join("config.json"), b"{}".to_vec()).unwrap();
    storage.write(&dir.join(".complete"), Vec::new()).unwrap();
    let mut names: Vec<String> = super::read_model(&storage, &dir).unwrap().into_iter().map(|(n, _)| n).collect();
    names.sort();
    assert_that!(names).is_equal_to(vec!["config.json".to_owned(), "model.bin".to_owned()]);
}

#[test]
fn read_model_without_files_returns_model_error() {
    let storage = Storage::local().unwrap();
    let err = super::read_model(&storage, &model_dir("absent")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}

#[test]
fn load_without_tokenizer_returns_model_error() {
    let storage = Storage::local().unwrap();
    let dir = model_dir("no-tokenizer");
    storage.write(&dir.join("model.bin"), b"poids".to_vec()).unwrap();
    let err = super::Transcriber::load(&dir, &storage).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
    assert_that!(err.to_string()).contains("tokenizer.json");
}
