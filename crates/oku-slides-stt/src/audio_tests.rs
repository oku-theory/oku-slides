use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

/// Un WAV PCM 16 bits : `channels` voies, `samples` échantillons par voie.
pub(crate) fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data: Vec<u8> =
        samples.iter().flat_map(|s| std::iter::repeat_n(s.to_le_bytes(), channels.into())).flatten().collect();
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + u32::try_from(data.len()).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&data);
    out
}

#[test]
fn parse_wav_stereo_16bit_returns_rate_and_mono_samples() {
    let (rate, mono) = parse_wav(&wav(8000, 2, &[16384, -16384])).unwrap();
    assert_that!(rate).is_equal_to(8000);
    assert_that!(mono).has_length(2);
    assert_that!(mono[0]).is_close_to(0.5, 1e-6);
    assert_that!(mono[1]).is_close_to(-0.5, 1e-6);
}

#[test]
fn parse_wav_not_riff_returns_audio_file_error() {
    let err = parse_wav(b"ceci n'est pas un wav").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::AudioFile);
}

#[test]
fn parse_wav_8bit_returns_audio_file_error() {
    let mut bytes = wav(8000, 1, &[0, 1]);
    bytes[34] = 8; // bits par échantillon
    assert_that!(parse_wav(&bytes).map(|_| ()).unwrap_err().kind()).is_equal_to(SttErrorKind::AudioFile);
}

#[test]
fn parse_wav_without_data_chunk_returns_audio_file_error() {
    let bytes = wav(8000, 1, &[]);
    let truncated = &bytes[..36];
    assert_that!(parse_wav(truncated).map(|_| ()).unwrap_err().kind()).is_equal_to(SttErrorKind::AudioFile);
}

#[test]
fn parse_wav_data_before_fmt_returns_audio_file_error() {
    let bytes = wav(8000, 1, &[0, 1]);
    let (header, rest) = bytes.split_at(12);
    let (fmt, data) = rest.split_at(24);
    let reordered = [header, data, fmt].concat();
    assert_that!(parse_wav(&reordered).map(|_| ()).unwrap_err().kind()).is_equal_to(SttErrorKind::AudioFile);
}

#[rstest::rstest]
#[case::eight_channels(8, true)]
#[case::nine_channels(9, false)]
fn parse_wav_channel_count_is_capped_at_8(#[case] channels: u16, #[case] ok: bool) {
    assert_that!(parse_wav(&wav(8000, channels, &[0, 1])).is_ok()).is_equal_to(ok);
}

#[test]
fn parse_wav_zero_rate_returns_audio_file_error() {
    let bytes = wav(0, 1, &[0, 1]);
    assert_that!(parse_wav(&bytes).map(|_| ()).unwrap_err().kind()).is_equal_to(SttErrorKind::AudioFile);
}

#[test]
fn recorder_fake_mic_replays_the_file_then_silence() {
    let dir = crate::test_support::fresh_dir("oku-slides-fake-mic-");
    let storage = Storage::local().unwrap();
    let file = dir.path().join("voix.wav");
    storage.write(&file, wav(16_000, 1, &vec![8192; 1600])).unwrap();
    let mut recorder = Recorder::start(Some(&file), &storage).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let (samples, rate) = recorder.take();
    recorder.finish();
    assert_that!(rate).is_equal_to(16_000);
    assert_that!(samples.len()).is_greater_than(0);
    assert_that!(recorder.level()).is_greater_than_or_equal_to(0.0);
}

#[test]
fn recorder_fake_mic_missing_file_returns_audio_file_error() {
    let storage = Storage::local().unwrap();
    let err = Recorder::start(Some(std::path::Path::new("/nonexistent/voix.wav")), &storage).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::AudioFile);
}

#[test]
fn sample_rate_f32_equals_sample_rate() {
    assert_that!(f64::from(SAMPLE_RATE_F32)).is_equal_to(f64::from(SAMPLE_RATE));
}
