//! Capture du micro (cpal) dans un fil dédié : le flux audio reste sur ce
//! fil (il n'est pas `Send` partout), les échantillons sont partagés.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub const SAMPLE_RATE: u32 = 16_000;

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

impl Recorder {
    /// Démarre l'enregistrement sur le micro par défaut.
    pub fn start() -> Result<Self, String> {
        let captured = Arc::new(Mutex::new(Captured::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let (cap, stp) = (captured.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            // Pour les tests sans micro : `ECHO_FAKE_MIC=fichier.wav` rejoue un fichier.
            if let Some(wav) = std::env::var_os("ECHO_FAKE_MIC") {
                match read_wav(std::path::Path::new(&wav)) {
                    Ok((rate, samples)) => {
                        ready_tx.send(Ok(())).ok();
                        cap.lock().unwrap().rate = rate;
                        let chunk = (rate / 50) as usize;
                        for part in samples.chunks(chunk) {
                            if stp.load(Ordering::Relaxed) {
                                return;
                            }
                            push_samples(&cap, &mut part.iter().copied());
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        while !stp.load(Ordering::Relaxed) {
                            push_samples(&cap, &mut std::iter::repeat_n(0.0, chunk));
                            std::thread::sleep(Duration::from_millis(20));
                        }
                    }
                    Err(e) => {
                        ready_tx.send(Err(e)).ok();
                    }
                }
                return;
            }
            let stream = match open_stream(cap) {
                Ok(s) => s,
                Err(e) => {
                    ready_tx.send(Err(e)).ok();
                    return;
                }
            };
            if let Err(e) = stream.play() {
                ready_tx.send(Err(format!("le micro ne démarre pas : {e}"))).ok();
                return;
            }
            ready_tx.send(Ok(())).ok();
            while !stp.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(20));
            }
            drop(stream);
        });
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self { captured, stop, thread: Some(thread) }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("le micro ne répond pas".into()),
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
        match self.captured.lock() {
            Ok(mut c) => (std::mem::take(&mut c.samples), c.rate),
            Err(_) => (Vec::new(), SAMPLE_RATE),
        }
    }

    pub fn level(&self) -> f32 {
        self.captured.lock().map(|c| c.level).unwrap_or(0.0)
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
        Self { ratio: from.max(1) as f64 / SAMPLE_RATE as f64, pos: 0.0, buf: Vec::new() }
    }

    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.buf.extend_from_slice(input);
        while self.pos + 1.0 < self.buf.len() as f64 {
            let j = self.pos as usize;
            let frac = (self.pos - j as f64) as f32;
            let (a, b) = (self.buf[j], self.buf[j + 1]);
            out.push(a + (b - a) * frac);
            self.pos += self.ratio;
        }
        let consumed = (self.pos as usize).min(self.buf.len());
        self.buf.drain(..consumed);
        self.pos -= consumed as f64;
    }
}

fn open_stream(captured: Arc<Mutex<Captured>>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("aucun micro trouvé")?;
    let config = device.default_input_config().map_err(|e| format!("micro inutilisable : {e}"))?;
    let channels = config.channels() as usize;
    let rate = config.sample_rate();
    captured.lock().unwrap().rate = rate;

    let push = move |mono: &mut dyn Iterator<Item = f32>| push_samples(&captured, mono);
    let err = |e| tracing::warn!(error = %e, "flux du micro");
    let stream_config: cpal::StreamConfig = config.into();
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            stream_config,
            move |data: &[f32], _: &_| push(&mut data.chunks(channels).map(|f| f.iter().sum::<f32>() / f.len() as f32)),
            err,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            stream_config,
            move |data: &[i16], _: &_| {
                push(
                    &mut data
                        .chunks(channels)
                        .map(|f| f.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / f.len() as f32),
                )
            },
            err,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            stream_config,
            move |data: &[u16], _: &_| {
                push(
                    &mut data
                        .chunks(channels)
                        .map(|f| f.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).sum::<f32>() / f.len() as f32),
                )
            },
            err,
            None,
        ),
        other => return Err(format!("format audio non pris en charge : {other:?}")),
    };
    stream.map_err(|e| format!("micro inutilisable : {e}"))
}

fn push_samples(captured: &Arc<Mutex<Captured>>, mono: &mut dyn Iterator<Item = f32>) {
    if let Ok(mut c) = captured.lock() {
        let start = c.samples.len();
        c.samples.extend(mono);
        let chunk = &c.samples[start..];
        if !chunk.is_empty() {
            let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
            c.level = (rms * 4.0).min(1.0);
        }
    }
}

/// Lecture minimale d'un WAV PCM 16 bits (mono ou stéréo).
fn read_wav(path: &std::path::Path) -> Result<(u32, Vec<f32>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("pas un fichier WAV".into());
    }
    let (mut rate, mut channels, mut bits) = (0u32, 1usize, 16u16);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
        let body = &bytes[i + 8..(i + 8 + len).min(bytes.len())];
        if id == b"fmt " && body.len() >= 16 {
            channels = u16::from_le_bytes([body[2], body[3]]).max(1) as usize;
            rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            if bits != 16 {
                return Err("WAV 16 bits attendu".into());
            }
            let samples: Vec<f32> =
                body.as_chunks::<2>().0.iter().map(|&b| i16::from_le_bytes(b) as f32 / 32768.0).collect();
            let mono = samples.chunks(channels).map(|f| f.iter().sum::<f32>() / f.len() as f32).collect();
            return Ok((rate, mono));
        }
        i += 8 + len + (len & 1);
    }
    Err("WAV sans données".into())
}

/// Rééchantillonnage linéaire (suffisant pour la parole).
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let n = (input.len() as f64 / ratio) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f64 * ratio;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = input[j];
            let b = *input.get(j + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}
