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
use oku_slides_platform::storage::Storage;
use serde::Deserialize;
use snafu::{OptionExt, ResultExt, ensure};

use crate::error::{
    FeaturesSnafu, MelFiltersSnafu, MissingModelFileSnafu, ModelFileSnafu, ModelLoadSnafu, ModelSettingsSnafu,
    NoLanguageSnafu, NotContiguousSnafu, SttError, TranscribeSnafu, ZeroSettingSnafu, boxed,
};

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
#[derive(Debug)]
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
    /// `dir` : dossier d'un modèle CTranslate2. Ses fichiers sont lus par
    /// `storage` et donnés à CTranslate2 en mémoire, un par un (chacun est
    /// libéré dès qu'il est copié). Les poids sont quantifiés en int8 au
    /// chargement (rapide sur processeur, peu de mémoire).
    pub fn load(dir: &Path, storage: &Storage) -> Result<Self, SttError> {
        let files = read_model(storage, dir)?;
        let file = |name: &'static str| {
            files
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, b)| b.as_slice())
                .context(MissingModelFileSnafu { dir, name })
        };
        let json = file("tokenizer.json")?;
        let mut tokenizer: hf::Tokenizer =
            tokenizers::Tokenizer::from_bytes(json).map_err(boxed).context(ModelLoadSnafu { dir })?.into();
        // Le contexte est encodé sans jetons spéciaux automatiques.
        tokenizer.disable_spacial_token();
        let pre = Preprocessor::parse(file("preprocessor_config.json")?)?;

        // L'identifiant ne sert qu'aux messages de CTranslate2.
        let mut reader = sys::ModelMemoryReader::new("whisper").map_err(boxed).context(ModelLoadSnafu { dir })?;
        for (name, bytes) in files {
            reader.register_file(&name, &bytes);
        }
        let config = sys::Config {
            compute_type: sys::ComputeType::INT8,
            num_threads_per_replica: threads(),
            ..Default::default()
        };
        let whisper =
            sys::Whisper::new_from_memory(&mut reader, config).map_err(boxed).context(ModelLoadSnafu { dir })?;
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
    ) -> Result<Transcript, SttError> {
        let (mut features, chunks) = self.pre.mel(audio)?;
        if chunks == 0 {
            return Ok(Transcript::default());
        }
        let shape = features.shape().to_vec();
        // `mel` rend un tableau en disposition standard : il est contigu.
        let data = features.as_slice_mut().context(NotContiguousSnafu)?;
        let view = sys::StorageView::new(&shape, data, Default::default())
            .map_err(boxed)
            .context(TranscribeSnafu { step: "features" })?;

        let lang = match language {
            Some(l) => l.to_owned(),
            None => {
                let detected =
                    self.whisper.detect_language(&view).map_err(boxed).context(TranscribeSnafu { step: "language" })?;
                let best = detected.into_iter().next().and_then(|v| v.into_iter().next()).context(NoLanguageSnafu)?;
                language_code(&best.language).to_owned()
            }
        };

        let context = match context.map(str::trim).filter(|c| !c.is_empty()) {
            Some(ctx) => Some(
                self.tokenizer
                    .encode(&format!(" {ctx}"))
                    .map_err(boxed)
                    .context(TranscribeSnafu { step: "context" })?,
            ),
            None => None,
        };
        let prompt = build_prompt(context, &lang);
        let mut options = decoding_options(prompt.len(), audio.len(), self.pre.sampling_rate, quick);
        let (mut text, mut no_speech) = self.run(&view, &prompt, chunks, &options)?;
        if repetitive(&text) {
            forbid_repetitions(&mut options);
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
    ) -> Result<(String, f32), SttError> {
        let results = self
            .whisper
            .generate(view, &vec![prompt.to_vec(); chunks], options)
            .map_err(boxed)
            .context(TranscribeSnafu { step: "decoding" })?;
        let mut text = String::new();
        let mut no_speech = 0f32;
        for r in results {
            no_speech = no_speech.max(r.no_speech_prob);
            let Some(seq) = r.sequences.into_iter().next() else { continue };
            text.push_str(
                &self.tokenizer.decode(spoken(seq)).map_err(boxed).context(TranscribeSnafu { step: "tokens" })?,
            );
        }
        Ok((text, no_speech))
    }
}

/// Fichiers d'un modèle (nom, contenu), lus par `storage` ; les fichiers
/// cachés (`.complete`, téléchargements `.part`) sont ignorés.
pub(crate) fn read_model(storage: &Storage, dir: &Path) -> Result<Vec<(String, Vec<u8>)>, SttError> {
    let mut out = Vec::new();
    for path in storage.list_files(dir).context(ModelFileSnafu { path: dir })? {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if name.starts_with('.') || name.ends_with(".part") {
            continue;
        }
        let bytes = storage.read(&path).context(ModelFileSnafu { path: &path })?;
        out.push((name.to_owned(), bytes));
    }
    ensure!(!out.is_empty(), MissingModelFileSnafu { dir, name: "model.bin" });
    Ok(out)
}

impl Preprocessor {
    /// Lit `preprocessor_config.json` ; sans filtres mel fournis, ils sont calculés.
    fn parse(bytes: &[u8]) -> Result<Self, SttError> {
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
        let raw: Raw = serde_json::from_slice(bytes).context(ModelSettingsSnafu)?;
        // Des tailles nulles feraient boucler ou paniquer le découpage en tranches.
        for (field, value) in [
            ("feature_size", raw.feature_size),
            ("hop_length", raw.hop_length),
            ("n_fft", raw.n_fft),
            ("n_samples", raw.n_samples),
            ("nb_max_frames", raw.nb_max_frames),
            ("sampling_rate", raw.sampling_rate),
        ] {
            ensure!(value > 0, ZeroSettingSnafu { field });
        }
        let mel_filters = match raw.mel_filters {
            Some(rows) => {
                let (r, c) = (rows.len(), rows.first().map_or(0, Vec::len));
                Array2::from_shape_vec((r, c), rows.into_iter().flatten().collect()).context(MelFiltersSnafu)?
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

    /// Spectrogramme log-mel par tranches de 30 s.
    fn mel(&self, samples: &[f32]) -> Result<(Array3<f32>, usize), SttError> {
        let mut stft = Spectrogram::new(self.n_fft, self.hop_length);
        let mut chunks = Vec::new();
        for chunk in samples.chunks(self.n_samples) {
            let mut m = Array2::zeros((self.feature_size, self.nb_max_frames));
            // Au plus `nb_max_frames` trames, même si les réglages en promettent plus.
            for (i, frame) in chunk.chunks(self.hop_length).take(self.nb_max_frames).enumerate() {
                if let Some(fft) = stft.add(frame) {
                    let mel = norm_mel(&log_mel_spectrogram(&fft, &self.mel_filters)).mapv(|v| v as f32);
                    m.slice_mut(s![.., i]).assign(&mel.slice(s![.., 0]));
                }
            }
            chunks.push(m);
        }
        let n = chunks.len();
        if n == 0 {
            return Ok((Array3::zeros((0, 0, 0)), 0));
        }
        let views: Vec<_> = chunks.iter().map(|a| a.view()).collect();
        let stacked = stack(Axis(0), &views).context(FeaturesSnafu { step: "mel" })?;
        Ok((stacked.as_standard_layout().into_owned(), n))
    }
}

/// `<|fr|>` → `fr`.
fn language_code(token: &str) -> &str {
    token.trim_start_matches("<|").trim_end_matches("|>")
}

/// Jetons d'amorce : le contexte (ses 200 derniers jetons) puis la consigne
/// « transcrire en `lang`, sans horodatage ».
fn build_prompt(context: Option<Vec<String>>, lang: &str) -> Vec<String> {
    let mut prompt = Vec::new();
    if let Some(tokens) = context {
        let skip = tokens.len().saturating_sub(MAX_PROMPT_TOKENS);
        prompt.push("<|startofprev|>".into());
        prompt.extend(tokens.into_iter().skip(skip));
    }
    prompt.extend([
        "<|startoftranscript|>".into(),
        format!("<|{lang}|>"),
        "<|transcribe|>".into(),
        "<|notimestamps|>".into(),
    ]);
    prompt
}

/// Options de décodage. Au plus ~12 jetons par seconde de parole : une boucle
/// d'hallucination (« dududu… ») ne peut pas s'emballer.
fn decoding_options(prompt_len: usize, samples: usize, sampling_rate: usize, quick: bool) -> sys::WhisperOptions {
    let seconds = samples as f32 / sampling_rate.max(1) as f32;
    let budget = (16.0 + 12.0 * seconds) as usize;
    sys::WhisperOptions {
        beam_size: if quick { 1 } else { 5 },
        return_no_speech_prob: true,
        max_length: (prompt_len + budget).min(448),
        ..Default::default()
    }
}

/// Pour la nouvelle tentative après une boucle : répétitions interdites.
fn forbid_repetitions(options: &mut sys::WhisperOptions) {
    options.repetition_penalty = 1.3;
    options.no_repeat_ngram_size = 3;
    options.beam_size = 1;
}

/// Les jetons de texte d'une séquence, sans les jetons spéciaux `<|…|>`.
fn spoken(seq: Vec<String>) -> Vec<String> {
    seq.into_iter().filter(|t| !(t.starts_with("<|") && t.ends_with("|>"))).collect()
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
    std::thread::available_parallelism().map_or(4, |n| n.get()).min(8)
}

#[cfg(test)]
#[path = "whisper_tests.rs"]
mod whisper_tests;
