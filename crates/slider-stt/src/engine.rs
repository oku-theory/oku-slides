//! Moteurs de transcription interchangeables.
//!
//! - `WhisperCpp` : whisper.cpp (Metal sur Mac), le moteur de l'app.
//! - `Ct2` (option `ct2`) : CTranslate2, le moteur de faster-whisper, pour comparer.

use std::path::Path;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Transcrit un audio mono 16 kHz.
pub trait Engine: Send + Sync {
    fn name(&self) -> &'static str;

    /// `quick` : pour l'aperçu, on privilégie la vitesse à la précision.
    fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        prompt: Option<&str>,
        quick: bool,
    ) -> Result<String, String>;
}

// ---- whisper.cpp -------------------------------------------------------------

pub struct WhisperCpp {
    ctx: WhisperContext,
}

impl WhisperCpp {
    /// `path` : un modèle ggml (`ggml-*.bin`).
    pub fn load(path: &Path) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|e| format!("modèle illisible ({}) : {e}", path.display()))?;
        Ok(Self { ctx })
    }
}

impl Engine for WhisperCpp {
    fn name(&self) -> &'static str {
        "whisper.cpp"
    }

    fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        prompt: Option<&str>,
        quick: bool,
    ) -> Result<String, String> {
        let mut state = self.ctx.create_state().map_err(|e| e.to_string())?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_translate(false);
        params.set_no_timestamps(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        // Pas de nouvelles tentatives à température plus haute pour l'aperçu :
        // elles peuvent coûter plusieurs passes quand le décodage hésite.
        if quick {
            params.set_temperature_inc(0.0);
        }
        params.set_n_threads(threads() as i32);
        if let Some(p) = prompt.filter(|p| !p.is_empty()) {
            params.set_initial_prompt(p);
        }
        state.full(params, audio).map_err(|e| e.to_string())?;
        let text =
            state.as_iter().filter_map(|s| s.to_str_lossy().ok().map(|t| t.into_owned())).collect::<Vec<_>>().join("");
        Ok(text)
    }
}

pub(crate) fn threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8)
}

// ---- CTranslate2 (faster-whisper) ---------------------------------------------

#[cfg(feature = "ct2")]
pub struct Ct2 {
    whisper: ct2rs::Whisper,
    beam: usize,
}

#[cfg(feature = "ct2")]
impl Ct2 {
    /// `dir` : un dossier de modèle CTranslate2 (model.bin, config.json,
    /// tokenizer.json, preprocessor_config.json). `int8` quantifie au chargement.
    pub fn load(dir: &Path, int8: bool, beam: usize) -> Result<Self, String> {
        let config = ct2rs::Config {
            compute_type: if int8 { ct2rs::ComputeType::INT8 } else { ct2rs::ComputeType::DEFAULT },
            num_threads_per_replica: threads(),
            ..Default::default()
        };
        let whisper = ct2rs::Whisper::new(dir, config)
            .map_err(|e| format!("modèle CTranslate2 illisible ({}) : {e}", dir.display()))?;
        Ok(Self { whisper, beam: beam.max(1) })
    }
}

#[cfg(feature = "ct2")]
impl Engine for Ct2 {
    fn name(&self) -> &'static str {
        "ctranslate2"
    }

    fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        _prompt: Option<&str>,
        quick: bool,
    ) -> Result<String, String> {
        let options = ct2rs::WhisperOptions { beam_size: if quick { 1 } else { self.beam }, ..Default::default() };
        let out = self.whisper.generate(audio, language, false, &options).map_err(|e| e.to_string())?;
        Ok(out.join(" "))
    }
}
