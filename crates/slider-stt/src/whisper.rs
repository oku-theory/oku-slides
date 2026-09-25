//! Whisper sur CTranslate2 (le moteur de faster-whisper), via ct2rs.
//!
//! Adapté de `ct2rs::Whisper` (licence MIT, Junpei Kawamoto) pour deux besoins
//! de la dictée : donner le texte déjà dicté en contexte (`<|startofprev|>`),
//! ce qui garde la cohérence d'une phrase à l'autre, et récupérer la langue
//! détectée pour la garder ensuite.

use std::path::Path;

use ct2rs::Tokenizer as _;
use ct2rs::sys;
use ct2rs::tokenizers::hf;
use mel_spec::mel::{log_mel_spectrogram, mel, norm_mel};
use mel_spec::stft::Spectrogram;
use ndarray::{Array2, Array3, Axis, s, stack};
use serde::Deserialize;

/// Résultat d'une transcription.
#[derive(Debug, Clone, Default)]
pub struct Transcript {
    pub text: String,
    /// Langue utilisée (`fr`, `en`…), détectée si elle n'était pas imposée.
    pub language: Option<String>,
    /// Probabilité qu'il n'y ait pas de parole (0..1).
    pub no_speech: f32,
}

pub struct Transcriber {
    whisper: sys::Whisper,
    tokenizer: hf::Tokenizer,
    pre: Preprocessor,
}

/// Réglages du spectrogramme (preprocessor_config.json).
struct Preprocessor {
    feature_size: usize,
    hop_length: usize,
    n_fft: usize,
    n_samples: usize,
    nb_max_frames: usize,
    sampling_rate: usize,
    mel_filters: Array2<f64>,
}

/// Nombre maximal de jetons de contexte (Whisper en accepte 224).
const MAX_PROMPT_TOKENS: usize = 200;

impl std::fmt::Debug for Transcriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transcriber").field("sampling_rate", &self.pre.sampling_rate).finish_non_exhaustive()
    }
}

impl Transcriber {
    /// `dir` : dossier d'un modèle CTranslate2. Les poids sont quantifiés en int8
    /// au chargement (rapide sur processeur, peu de mémoire).
    pub fn load(dir: &Path) -> Result<Self, String> {
        let err = |e: &dyn std::fmt::Display| format!("modèle de dictée illisible ({}) : {e}", dir.display());
        let config = sys::Config {
            compute_type: sys::ComputeType::INT8,
            num_threads_per_replica: threads(),
            ..Default::default()
        };
        let whisper = sys::Whisper::new(dir, config).map_err(|e| err(&e))?;
        let mut tokenizer = hf::Tokenizer::new(dir).map_err(|e| err(&e))?;
        // Le contexte est encodé sans jetons spéciaux automatiques.
        tokenizer.disable_spacial_token();
        let pre = Preprocessor::read(&dir.join("preprocessor_config.json")).map_err(|e| err(&e))?;
        Ok(Self { whisper, tokenizer, pre })
    }

    /// Transcrit un audio mono 16 kHz.
    /// `quick` : décodage glouton (aperçu) ; sinon recherche en faisceau (texte final).
    pub fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        context: Option<&str>,
        quick: bool,
    ) -> Result<Transcript, String> {
        let (mut features, chunks) = self.mel(audio);
        if chunks == 0 {
            return Ok(Transcript::default());
        }
        let shape = features.shape().to_vec();
        let view = sys::StorageView::new(&shape, features.as_slice_mut().expect("contigu"), Default::default())
            .map_err(|e| e.to_string())?;

        let lang = match language {
            Some(l) => l.to_owned(),
            None => {
                let detected = self.whisper.detect_language(&view).map_err(|e| e.to_string())?;
                let best =
                    detected.into_iter().next().and_then(|v| v.into_iter().next()).ok_or("langue introuvable")?;
                best.language.trim_start_matches("<|").trim_end_matches("|>").to_owned()
            }
        };

        let mut prompt: Vec<String> = Vec::new();
        if let Some(ctx) = context.map(str::trim).filter(|c| !c.is_empty()) {
            let tokens = self.tokenizer.encode(&format!(" {ctx}")).map_err(|e| e.to_string())?;
            let keep = tokens.len().saturating_sub(MAX_PROMPT_TOKENS);
            prompt.push("<|startofprev|>".into());
            prompt.extend(tokens.into_iter().skip(keep));
        }
        prompt.extend([
            "<|startoftranscript|>".into(),
            format!("<|{lang}|>"),
            "<|transcribe|>".into(),
            "<|notimestamps|>".into(),
        ]);

        // Au plus ~12 jetons par seconde de parole : une boucle d'hallucination
        // (« dududu… ») ne peut pas s'emballer.
        let seconds = audio.len() as f32 / self.pre.sampling_rate as f32;
        let budget = (16.0 + 12.0 * seconds) as usize;
        let mut options = sys::WhisperOptions {
            beam_size: if quick { 1 } else { 5 },
            return_no_speech_prob: true,
            max_length: (prompt.len() + budget).min(448),
            ..Default::default()
        };
        let (mut text, mut no_speech) = self.run(&view, &prompt, chunks, &options)?;
        if repetitive(&text) {
            // Nouvelle tentative qui interdit les répétitions.
            options.repetition_penalty = 1.3;
            options.no_repeat_ngram_size = 3;
            options.beam_size = 1;
            (text, no_speech) = self.run(&view, &prompt, chunks, &options)?;
            if repetitive(&text) {
                text.clear();
            }
        }
        Ok(Transcript { text, language: Some(lang), no_speech })
    }

    fn run(
        &self,
        view: &sys::StorageView,
        prompt: &[String],
        chunks: usize,
        options: &sys::WhisperOptions,
    ) -> Result<(String, f32), String> {
        let results =
            self.whisper.generate(view, &vec![prompt.to_vec(); chunks], options).map_err(|e| e.to_string())?;
        let mut text = String::new();
        let mut no_speech = 0f32;
        for r in results {
            no_speech = no_speech.max(r.no_speech_prob);
            let Some(seq) = r.sequences.into_iter().next() else { continue };
            let seq: Vec<String> = seq.into_iter().filter(|t| !(t.starts_with("<|") && t.ends_with("|>"))).collect();
            text.push_str(&self.tokenizer.decode(seq).map_err(|e| e.to_string())?);
        }
        Ok((text, no_speech))
    }

    /// Spectrogramme log-mel par tranches de 30 s.
    fn mel(&self, samples: &[f32]) -> (Array3<f32>, usize) {
        let p = &self.pre;
        let mut stft = Spectrogram::new(p.n_fft, p.hop_length);
        let mut chunks = Vec::new();
        for chunk in samples.chunks(p.n_samples) {
            let mut m = Array2::zeros((p.feature_size, p.nb_max_frames));
            for (i, frame) in chunk.chunks(p.hop_length).enumerate() {
                if let Some(fft) = stft.add(frame) {
                    let mel = norm_mel(&log_mel_spectrogram(&fft, &p.mel_filters)).mapv(|v| v as f32);
                    m.slice_mut(s![.., i]).assign(&mel.slice(s![.., 0]));
                }
            }
            chunks.push(m);
        }
        let n = chunks.len();
        if n == 0 {
            return (Array3::zeros((0, 0, 0)), 0);
        }
        let views: Vec<_> = chunks.iter().map(|a| a.view()).collect();
        let stacked = stack(Axis(0), &views).expect("tranches de même forme");
        (stacked.as_standard_layout().into_owned(), n)
    }
}

impl Preprocessor {
    fn read(path: &Path) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Raw {
            feature_size: usize,
            hop_length: usize,
            n_fft: usize,
            n_samples: usize,
            nb_max_frames: usize,
            sampling_rate: usize,
            mel_filters: Option<Vec<Vec<f64>>>,
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let raw: Raw = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let mel_filters = match raw.mel_filters {
            Some(rows) => {
                let (r, c) = (rows.len(), rows.first().map_or(0, Vec::len));
                Array2::from_shape_vec((r, c), rows.into_iter().flatten().collect()).map_err(|e| e.to_string())?
            }
            None => mel(raw.sampling_rate as f64, raw.n_fft, raw.feature_size, None, None, false, true),
        };
        Ok(Self {
            feature_size: raw.feature_size,
            hop_length: raw.hop_length,
            n_fft: raw.n_fft,
            n_samples: raw.n_samples,
            nb_max_frames: raw.nb_max_frames,
            sampling_rate: raw.sampling_rate,
            mel_filters,
        })
    }
}

/// Texte qui tourne en boucle (le critère de « taux de compression » de
/// Whisper, approché par la part de quadrigrammes de caractères répétés).
pub(crate) fn repetitive(text: &str) -> bool {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    // Un motif de 1 à 4 caractères répété 6 fois d'affilée (« jjjjjj », « mjmjmj »).
    for len in 1..=4 {
        let mut run = 1;
        for i in len..chars.len() {
            if chars[i] == chars[i - len] {
                run += 1;
                if run >= len * 6 {
                    return true;
                }
            } else {
                run = 1;
            }
        }
    }
    if chars.len() < 40 {
        return false;
    }
    let grams: std::collections::HashSet<&[char]> = chars.windows(4).collect();
    let total = chars.len() - 3;
    total as f32 / grams.len() as f32 > 2.4
}

/// Fils de calcul : les cœurs disponibles, jusqu'à 8.
pub(crate) fn threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8)
}

#[cfg(test)]
mod tests {
    use super::repetitive;

    #[test]
    fn repere_les_boucles() {
        assert!(repetitive("Arudun, slid, sud, affaidaudududududududududududududududududududududududududududududud"));
        assert!(!repetitive("Ajoute une slide sur la feuille de route du deuxième trimestre, avec trois colonnes."));
        assert!(!repetitive("Ask what you can do for your country, ask what your country can do for you."));
        assert!(repetitive("Houdun slidzu la feuille de route d'un mjmjmjmjjjjjjjjjjjj"));
        assert!(!repetitive("Il a dit : « non, non, non » et c'est tout. Rendez-vous le 11/11/2026."));
    }
}
