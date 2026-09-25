//! Détection de la voix (earshot) et découpage en phrases.
//!
//! L'audio arrive par morceaux (16 kHz mono). Chaque trame de 16 ms reçoit un
//! score de voix ; une phrase commence après quelques trames parlées et se
//! termine après une courte pause. Seules les phrases sont transcrites : les
//! silences ne coûtent rien et n'inventent rien.

use earshot::Detector;

use crate::SAMPLE_RATE;

/// Trame d'earshot : 256 échantillons (16 ms).
pub const FRAME: usize = 256;
const FRAME_SECS: f32 = FRAME as f32 / SAMPLE_RATE as f32;

/// Réglages du découpage.
#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    /// Score à partir duquel une trame est parlée.
    pub threshold: f32,
    /// Trames parlées d'affilée pour ouvrir une phrase.
    pub onset_frames: usize,
    /// Pause qui clôt une phrase (s)…
    pub pause: f32,
    /// …si elle dure déjà au moins ceci (s) ; plus courte, elle attend la suite,
    pub min_phrase: f32,
    /// sauf après une pause plus longue que celle-ci (s).
    pub long_pause: f32,
    /// Audio gardé avant le début et après la fin de la parole (s).
    pub pre_roll: f32,
    pub post_roll: f32,
    /// Longueur maximale d'une phrase (s) : Whisper lit 30 s à la fois.
    pub max_phrase: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            onset_frames: 3,
            pause: 0.7,
            min_phrase: 2.5,
            long_pause: 1.2,
            pre_roll: 0.3,
            post_roll: 0.25,
            max_phrase: 25.0,
        }
    }
}

/// Une phrase : intervalle d'échantillons dans l'audio de la session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phrase {
    pub start: usize,
    pub end: usize,
}

impl Phrase {
    pub fn seconds(&self) -> f32 {
        (self.end - self.start) as f32 / SAMPLE_RATE as f32
    }
}

pub struct Segmenter {
    config: VadConfig,
    detector: Box<Detector>,
    /// Tout l'audio de la session (16 kHz).
    pub audio: Vec<f32>,
    /// Échantillons déjà passés au détecteur.
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

impl Segmenter {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            detector: Detector::default_boxed(),
            audio: Vec::new(),
            scanned: 0,
            voiced_run: 0,
            open: None,
            last_voice: 0,
            heard: false,
            speaking: false,
        }
    }

    fn secs(&self, s: f32) -> usize {
        (s * SAMPLE_RATE as f32) as usize
    }

    /// Ajoute de l'audio ; renvoie les phrases terminées.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Phrase> {
        self.audio.extend_from_slice(samples);
        let mut done = Vec::new();
        while self.scanned + FRAME <= self.audio.len() {
            let at = self.scanned;
            let score = self.detector.predict_f32(&self.audio[at..at + FRAME]);
            self.scanned += FRAME;
            let voiced = score >= self.config.threshold;
            self.speaking = voiced;
            if voiced {
                self.voiced_run += 1;
                self.last_voice = self.scanned;
                if self.open.is_none() && self.voiced_run >= self.config.onset_frames {
                    let begin =
                        (at + FRAME).saturating_sub(self.config.onset_frames * FRAME + self.secs(self.config.pre_roll));
                    self.open = Some(begin);
                    self.heard = true;
                }
            } else {
                self.voiced_run = 0;
            }
            if let Some(start) = self.open {
                let quiet = self.scanned - self.last_voice;
                let long_enough = self.last_voice.saturating_sub(start) >= self.secs(self.config.min_phrase);
                // Une phrase courte (« Ask not… ») attend la suite : Whisper
                // transcrit mieux une phrase entière.
                let paused = quiet >= self.secs(self.config.pause)
                    && (long_enough || quiet >= self.secs(self.config.long_pause));
                let too_long = self.scanned - start >= self.secs(self.config.max_phrase);
                if paused || too_long {
                    let end = if paused {
                        (self.last_voice + self.secs(self.config.post_roll)).min(self.scanned)
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
        self.open.map(|start| Phrase { start, end: self.audio.len() })
    }

    /// Clôt la phrase en cours (fin de l'écoute).
    pub fn close(&mut self) -> Option<Phrase> {
        let start = self.open.take()?;
        let end = (self.last_voice + self.secs(self.config.post_roll)).min(self.audio.len()).max(start);
        Some(Phrase { start, end })
    }

    /// Silence depuis la dernière parole (s) ; depuis le début si rien entendu.
    pub fn silence(&self) -> f32 {
        (self.scanned - self.last_voice) as f32 / SAMPLE_RATE as f32
    }

    pub fn slice(&self, p: Phrase) -> &[f32] {
        &self.audio[p.start.min(self.audio.len())..p.end.min(self.audio.len())]
    }

    pub fn frame_secs() -> f32 {
        FRAME_SECS
    }
}

/// Découpe un audio complet en phrases (transcription d'un fichier).
pub fn split(audio: &[f32], config: VadConfig) -> Vec<Phrase> {
    let mut seg = Segmenter::new(config);
    let mut phrases = seg.push(audio);
    phrases.extend(seg.close());
    phrases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_silence_ne_donne_aucune_phrase() {
        let silence = vec![0.0f32; SAMPLE_RATE as usize * 3];
        assert!(split(&silence, VadConfig::default()).is_empty());
    }

    #[test]
    fn une_phrase_trop_longue_est_coupee() {
        let mut seg = Segmenter::new(VadConfig { max_phrase: 1.0, ..Default::default() });
        seg.open = Some(0);
        seg.last_voice = 0;
        // Pas de voix réelle ici : on vérifie seulement la coupe par la longueur.
        seg.config.pause = 100.0;
        let out = seg.push(&vec![0.0; SAMPLE_RATE as usize * 2]);
        assert!(!out.is_empty());
        assert!(out[0].seconds() <= 1.1);
    }
}
