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

/// Découpe l'audio en phrases, au fil de l'eau. Les positions sont comptées
/// depuis le début de la session ; seul l'audio encore utile est gardé.
pub struct Segmenter {
    config: VadConfig,
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
    pub fn new(config: VadConfig) -> Self {
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

    fn secs(&self, s: f32) -> usize {
        (s * SAMPLE_RATE as f32) as usize
    }

    /// Fin de l'audio reçu (position dans la session).
    fn end(&self) -> usize {
        self.base + self.audio.len()
    }

    /// Oublie l'audio qui ne peut plus servir : avant la phrase en cours ou,
    /// sans phrase, avant l'amorce d'une prochaine. Par paquets d'une seconde
    /// au moins, pour ne pas décaler le tampon à chaque morceau.
    fn forget(&mut self) {
        let margin = self.config.onset_frames * FRAME + self.secs(self.config.pre_roll);
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
        self.open.map(|start| Phrase { start, end: self.end() })
    }

    /// Clôt la phrase en cours (fin de l'écoute).
    pub fn close(&mut self) -> Option<Phrase> {
        let start = self.open.take()?;
        let end = (self.last_voice + self.secs(self.config.post_roll)).min(self.end()).max(start);
        Some(Phrase { start, end })
    }

    /// Silence depuis la dernière parole (s) ; depuis le début si rien entendu.
    pub fn silence(&self) -> f32 {
        (self.scanned - self.last_voice) as f32 / SAMPLE_RATE as f32
    }

    /// Audio d'une phrase (vide pour ce qui a déjà été oublié).
    pub fn slice(&self, p: Phrase) -> &[f32] {
        let local = |i: usize| i.saturating_sub(self.base).min(self.audio.len());
        &self.audio[local(p.start)..local(p.end)]
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
    fn une_longue_ecoute_silencieuse_ne_garde_que_l_amorce() {
        let mut seg = Segmenter::new(VadConfig::default());
        let chunk = vec![0.0f32; 800]; // 50 ms, comme la boucle d'écoute
        for _ in 0..(60 * 20) {
            assert!(seg.push(&chunk).is_empty());
        }
        assert!(seg.audio.len() < 2 * SAMPLE_RATE as usize, "{} échantillons gardés", seg.audio.len());
        assert_eq!(seg.end(), 60 * SAMPLE_RATE as usize, "les positions restent celles de la session");
        assert!((seg.silence() - 60.0).abs() < 0.1);
    }

    #[test]
    fn une_phrase_rendue_se_lit_avant_l_appel_suivant() {
        let mut seg = Segmenter::new(VadConfig { max_phrase: 1.0, ..Default::default() });
        // Trois secondes de silence, oubliées en partie, puis une phrase coupée
        // pour sa longueur (1 s).
        seg.push(&vec![0.0; SAMPLE_RATE as usize * 3]);
        let open_at = seg.scanned;
        seg.open = Some(open_at);
        seg.last_voice = open_at;
        seg.config.pause = 100.0;
        let out = seg.push(&vec![0.25; SAMPLE_RATE as usize * 2]);
        let first = out[0];
        assert_eq!(first.start, open_at);
        assert_eq!(seg.slice(first).len(), first.end - first.start, "l'audio de la phrase est encore là, entier");
        assert!(seg.base > 0, "le silence d'avant a été oublié");
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
