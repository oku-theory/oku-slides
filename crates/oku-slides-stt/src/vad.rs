//! Détection de la voix (earshot) et découpage en phrases.
//!
//! L'audio arrive par morceaux (16 kHz mono). Chaque trame de 16 ms reçoit un
//! score de voix ; une phrase commence après quelques trames parlées et se
//! termine après une courte pause. Seules les phrases sont transcrites : les
//! silences ne coûtent rien et n'inventent rien.

use earshot::Detector;
use oku_slides_platform::config::VadSettings;

use crate::SAMPLE_RATE;
use crate::num::{self, seconds};

/// Trame d'earshot : 256 échantillons (16 ms).
pub const FRAME: usize = 256;

/// Une phrase : intervalle d'échantillons dans l'audio de la session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phrase {
    pub start: usize,
    pub end: usize,
}

impl Phrase {
    pub fn seconds(&self) -> f32 {
        seconds(self.end - self.start)
    }
}

/// Découpe l'audio en phrases, au fil de l'eau. Les positions sont comptées
/// depuis le début de la session ; seul l'audio encore utile est gardé.
pub struct Segmenter {
    config: VadSettings,
    detector: Box<Detector>,
    /// Audio encore utile (16 kHz) : la phrase en cours, ou l'amorce d'une
    /// prochaine. `audio[0]` est l'échantillon `base` de la session.
    audio: Vec<f32>,
    base: usize,
    /// Échantillons déjà passés au détecteur (position dans la session).
    scanned: usize,
    voiced_run: usize,
    /// Début de la phrase en cours (échantillon), s'il y en a une.
    open: Option<usize>,
    /// Fin de la dernière trame parlée.
    last_voice: usize,
    /// Au moins une phrase entendue.
    pub heard: bool,
    /// Trame en cours parlée ?
    pub speaking: bool,
}

impl std::fmt::Debug for Segmenter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Segmenter")
            .field("base", &self.base)
            .field("scanned", &self.scanned)
            .field("open", &self.open)
            .field("heard", &self.heard)
            .finish_non_exhaustive()
    }
}

impl Segmenter {
    pub fn new(config: VadSettings) -> Self {
        Self {
            config,
            detector: Detector::default_boxed(),
            audio: Vec::new(),
            base: 0,
            scanned: 0,
            voiced_run: 0,
            open: None,
            last_voice: 0,
            heard: false,
            speaking: false,
        }
    }

    /// Fin de l'audio reçu (position dans la session).
    fn end(&self) -> usize {
        self.base + self.audio.len()
    }

    /// Oublie l'audio qui ne peut plus servir : avant la phrase en cours ou,
    /// sans phrase, avant l'amorce d'une prochaine. Par paquets d'une seconde
    /// au moins, pour ne pas décaler le tampon à chaque morceau.
    fn forget(&mut self) {
        let margin = self.config.onset_frames * FRAME + num::samples(self.config.pre_roll);
        let keep_from = self.open.unwrap_or(self.scanned.saturating_sub(margin));
        let drop = keep_from.saturating_sub(self.base);
        if drop >= SAMPLE_RATE as usize {
            self.audio.drain(..drop);
            self.base += drop;
        }
    }

    /// Ajoute de l'audio ; renvoie les phrases terminées. Elles se lisent
    /// (`slice`) avant l'appel suivant : c'est là que l'audio passé est oublié.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Phrase> {
        self.forget();
        self.audio.extend_from_slice(samples);
        let mut done = Vec::new();
        while self.scanned + FRAME <= self.end() {
            let at = self.scanned;
            let local = at - self.base;
            let score = self.detector.predict_f32(&self.audio[local..local + FRAME]);
            self.scanned += FRAME;
            let voiced = score >= self.config.threshold;
            self.speaking = voiced;
            if voiced {
                self.voiced_run += 1;
                self.last_voice = self.scanned;
                if self.open.is_none() && self.voiced_run >= self.config.onset_frames {
                    let begin = (at + FRAME)
                        .saturating_sub(self.config.onset_frames * FRAME + num::samples(self.config.pre_roll));
                    self.open = Some(begin);
                    self.heard = true;
                }
            } else {
                self.voiced_run = 0;
            }
            if let Some(start) = self.open {
                let quiet = self.scanned - self.last_voice;
                let long_enough = self.last_voice.saturating_sub(start) >= num::samples(self.config.min_phrase);
                // Une phrase courte (« Ask not… ») attend la suite : Whisper
                // transcrit mieux une phrase entière.
                let paused = quiet >= num::samples(self.config.pause)
                    && (long_enough || quiet >= num::samples(self.config.long_pause));
                let too_long = self.scanned - start >= num::samples(self.config.max_phrase);
                if paused || too_long {
                    let end = if paused {
                        (self.last_voice + num::samples(self.config.post_roll)).min(self.scanned)
                    } else {
                        self.scanned
                    };
                    done.push(Phrase { start, end });
                    // Une phrase coupée pour sa longueur continue aussitôt.
                    self.open = if too_long && !paused { Some(self.scanned) } else { None };
                }
            }
        }
        done
    }

    /// Phrase en cours (pas encore close).
    pub fn current(&self) -> Option<Phrase> {
        self.open.map(|start| Phrase { start, end: self.end() })
    }

    /// Clôt la phrase en cours (fin de l'écoute).
    pub fn close(&mut self) -> Option<Phrase> {
        let start = self.open.take()?;
        let end = (self.last_voice + num::samples(self.config.post_roll)).min(self.end()).max(start);
        Some(Phrase { start, end })
    }

    /// Silence depuis la dernière parole (s) ; depuis le début si rien entendu.
    pub fn silence(&self) -> f32 {
        seconds(self.scanned - self.last_voice)
    }

    /// Audio d'une phrase (vide pour ce qui a déjà été oublié).
    pub fn slice(&self, p: Phrase) -> &[f32] {
        let local = |i: usize| i.saturating_sub(self.base).min(self.audio.len());
        &self.audio[local(p.start)..local(p.end)]
    }
}

/// Découpe un audio complet en phrases (transcription d'un fichier).
pub fn split(audio: &[f32], config: VadSettings) -> Vec<Phrase> {
    let mut seg = Segmenter::new(config);
    let mut phrases = seg.push(audio);
    phrases.extend(seg.close());
    phrases
}

#[cfg(test)]
#[path = "vad_tests.rs"]
mod vad_tests;
