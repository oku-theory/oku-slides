//! Capture du micro (cpal) dans un fil dédié : le flux audio reste sur ce
//! fil (il n'est pas `Send` partout), les échantillons sont partagés.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use oku_slides_platform::storage::Storage;
use snafu::{OptionExt, ResultExt, ensure};

use crate::error::{
    AudioFileSnafu, MicrophoneConfigSnafu, MicrophonePlaySnafu, MicrophoneStreamSnafu, MicrophoneTimeoutSnafu,
    NoMicrophoneSnafu, SttError, UnsupportedFormatSnafu, WavSnafu,
};
use crate::num::{count_f32, count_f64, narrow_f32, trunc_usize};

/// Rate of the audio given to the engines: mono, 16 kHz.
pub const SAMPLE_RATE: u32 = 16_000;

/// [`SAMPLE_RATE`] as `f32`, exact; a test checks they are equal.
pub(crate) const SAMPLE_RATE_F32: f32 = 16_000.0;

/// Échantillons enregistrés : mono, au débit de l'appareil.
#[derive(Debug, Default)]
pub struct Captured {
    pub samples: Vec<f32>,
    pub rate: u32,
    /// Niveau récent (RMS), 0..1.
    pub level: f32,
}

#[derive(Debug)]
pub struct Recorder {
    pub captured: Arc<Mutex<Captured>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// Les échantillons partagés, même si un fil a paniqué en les tenant : ce
/// ne sont que des données audio.
fn lock(captured: &Mutex<Captured>) -> MutexGuard<'_, Captured> {
    captured.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Recorder {
    /// Démarre l'enregistrement sur le micro par défaut.
    /// Démarre l'enregistrement sur le micro par défaut, ou rejoue
    /// `fake_mic` (fichier WAV) à la place (tests, démos).
    pub fn start(fake_mic: Option<&std::path::Path>, storage: &Storage) -> Result<Self, SttError> {
        let fake_mic = fake_mic.map(std::path::Path::to_path_buf);
        let storage = storage.clone();
        let captured = Arc::new(Mutex::new(Captured::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), SttError>>();
        let (shared, halt) = (captured.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            if let Some(wav) = fake_mic {
                match read_wav(&storage, &wav) {
                    Ok((rate, samples)) => {
                        ready_tx.send(Ok(())).ok();
                        lock(&shared).rate = rate;
                        // 20 ms per packet; at least one sample, since `chunks(0)` would panic.
                        let chunk = ((rate / 50) as usize).max(1);
                        for part in samples.chunks(chunk) {
                            if halt.load(Ordering::Relaxed) {
                                return;
                            }
                            push_samples(&shared, &mut part.iter().copied());
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        while !halt.load(Ordering::Relaxed) {
                            push_samples(&shared, &mut std::iter::repeat_n(0.0, chunk));
                            std::thread::sleep(Duration::from_millis(20));
                        }
                    }
                    Err(e) => {
                        ready_tx.send(Err(e)).ok();
                    }
                }
                return;
            }
            let stream = match open_stream(shared) {
                Ok(s) => s,
                Err(e) => {
                    ready_tx.send(Err(e)).ok();
                    return;
                }
            };
            if let Err(e) = stream.play().context(MicrophonePlaySnafu) {
                ready_tx.send(Err(e.into())).ok();
                return;
            }
            ready_tx.send(Ok(())).ok();
            while !halt.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(20));
            }
            drop(stream);
        });
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self { captured, stop, thread: Some(thread) }),
            Ok(Err(e)) => Err(e),
            Err(_) => MicrophoneTimeoutSnafu.fail()?,
        }
    }

    /// Arrête la capture (les derniers échantillons restent à prendre).
    pub fn finish(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().ok();
        }
    }

    /// Échantillons arrivés depuis le dernier appel, et leur débit.
    pub fn take(&self) -> (Vec<f32>, u32) {
        let mut c = lock(&self.captured);
        (std::mem::take(&mut c.samples), c.rate)
    }

    pub fn level(&self) -> f32 {
        lock(&self.captured).level
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Rééchantillonnage linéaire vers 16 kHz, morceau par morceau (sans
/// discontinuité entre deux morceaux).
#[derive(Debug)]
pub struct Resampler {
    ratio: f64,
    pos: f64,
    buf: Vec<f32>,
}

impl Resampler {
    pub fn new(from: u32) -> Self {
        Self { ratio: f64::from(from.max(1)) / f64::from(SAMPLE_RATE), pos: 0.0, buf: Vec::new() }
    }

    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.buf.extend_from_slice(input);
        while self.pos + 1.0 < count_f64(self.buf.len()) {
            let j = trunc_usize(self.pos);
            let frac = narrow_f32(self.pos - count_f64(j));
            let (a, b) = (self.buf[j], self.buf[j + 1]);
            out.push(a + (b - a) * frac);
            self.pos += self.ratio;
        }
        let consumed = trunc_usize(self.pos).min(self.buf.len());
        self.buf.drain(..consumed);
        self.pos -= count_f64(consumed);
    }
}

fn open_stream(captured: Arc<Mutex<Captured>>) -> Result<cpal::Stream, SttError> {
    let host = cpal::default_host();
    let device = host.default_input_device().context(NoMicrophoneSnafu)?;
    let config = device.default_input_config().context(MicrophoneConfigSnafu)?;
    let channels = config.channels() as usize;
    let rate = config.sample_rate();
    lock(&captured).rate = rate;

    let push = move |mono: &mut dyn Iterator<Item = f32>| push_samples(&captured, mono);
    let err = |e| tracing::warn!(error = %e, "flux du micro");
    let stream_config: cpal::StreamConfig = config.into();
    let stream =
        match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                stream_config,
                move |data: &[f32], _: &_| {
                    push(&mut data.chunks(channels).map(|f| f.iter().sum::<f32>() / count_f32(f.len())));
                },
                err,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                stream_config,
                move |data: &[i16], _: &_| {
                    push(
                        &mut data
                            .chunks(channels)
                            .map(|f| f.iter().map(|&s| f32::from(s) / 32768.0).sum::<f32>() / count_f32(f.len())),
                    );
                },
                err,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                stream_config,
                move |data: &[u16], _: &_| {
                    push(&mut data.chunks(channels).map(|f| {
                        f.iter().map(|&s| (f32::from(s) - 32768.0) / 32768.0).sum::<f32>() / count_f32(f.len())
                    }));
                },
                err,
                None,
            ),
            other => return UnsupportedFormatSnafu { format: format!("{other:?}") }.fail()?,
        };
    Ok(stream.context(MicrophoneStreamSnafu)?)
}

fn push_samples(captured: &Arc<Mutex<Captured>>, mono: &mut dyn Iterator<Item = f32>) {
    {
        let mut c = lock(captured);
        let start = c.samples.len();
        c.samples.extend(mono);
        let chunk = &c.samples[start..];
        if !chunk.is_empty() {
            let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / count_f32(chunk.len())).sqrt();
            c.level = (rms * 4.0).min(1.0);
        }
    }
}

/// Lecture minimale d'un WAV PCM 16 bits (mono ou stéréo).
fn read_wav(storage: &Storage, path: &std::path::Path) -> Result<(u32, Vec<f32>), SttError> {
    let bytes = storage.read(path).context(AudioFileSnafu { path })?;
    parse_wav(&bytes)
}

/// Entier petit-boutiste sur 4 octets, à `at`.
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(*bytes.get(at..at + 4)?.first_chunk()?))
}

/// Decodes a 16-bit PCM WAV of up to 8 channels: its sample rate and mono samples.
pub(crate) fn parse_wav(bytes: &[u8]) -> Result<(u32, Vec<f32>), SttError> {
    ensure!(
        bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        WavSnafu { reason: "not a WAV file" }
    );
    let (mut rate, mut channels, mut bits) = (0u32, 1usize, 16u16);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = usize::try_from(u32_at(bytes, i + 4).unwrap_or_default()).unwrap_or(usize::MAX);
        let body = &bytes[i + 8..i.saturating_add(8).saturating_add(len).min(bytes.len())];
        if id == b"fmt " && body.len() >= 16 {
            channels = u16::from_le_bytes([body[2], body[3]]).max(1) as usize;
            rate = u32_at(body, 4).unwrap_or_default();
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            ensure!(bits == 16, WavSnafu { reason: "16-bit PCM expected" });
            ensure!(channels <= 8, WavSnafu { reason: "at most 8 channels expected" });
            // Zero, or no `fmt ` chunk before the data: nothing tells how fast to play it.
            ensure!(rate > 0, WavSnafu { reason: "zero sample rate" });
            let samples: Vec<f32> =
                body.as_chunks::<2>().0.iter().map(|&b| f32::from(i16::from_le_bytes(b)) / 32768.0).collect();
            let mono = samples.chunks(channels).map(|f| f.iter().sum::<f32>() / count_f32(f.len())).collect();
            return Ok((rate, mono));
        }
        i = i.saturating_add(8).saturating_add(len).saturating_add(len & 1);
    }
    WavSnafu { reason: "no data chunk" }.fail()?
}

/// Rééchantillonnage linéaire (suffisant pour la parole).
#[must_use]
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 || input.is_empty() {
        return input.to_vec();
    }
    let ratio = f64::from(from) / f64::from(to);
    let n = trunc_usize(count_f64(input.len()) / ratio);
    (0..n)
        .map(|i| {
            let pos = count_f64(i) * ratio;
            let j = trunc_usize(pos);
            let frac = narrow_f32(pos - count_f64(j));
            let a = input[j];
            let b = *input.get(j + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod audio_tests;
