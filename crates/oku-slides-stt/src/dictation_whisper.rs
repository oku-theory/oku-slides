//! Phrase-by-phrase dictation (Whisper, API): closed phrases at final quality,
//! a preview of the current phrase (Whisper only).
//!
//! `listen_loop` (listen.rs) sends [`Job`]s; this thread transcribes them in order, closed
//! phrases first, then utterance closes ([`Job::Flush`]), then the preview.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use oku_slides_i18n::Localize;

use crate::api::{ApiConfig, ApiTranscriber};
use crate::error::ZeroSettingSnafu;
use crate::phrase_engine::PhraseEngine;
use crate::utterance::Spoken;
use crate::vad::{Phrase, Segmenter};
use crate::{Config, Inner, Model, State, Stt, SttError, Transcriber, Transcript, context_of, keep};

/// Travail pour le fil de transcription.
pub(crate) enum Job {
    /// A closed phrase: final text, stored at absolute phrase `index`; dropped if `epoch` is stale.
    Final { index: usize, audio: Vec<f32>, epoch: u64 },
    /// Listening: closes the utterance of the phrases before index `upto`, keeping the later ones.
    Flush { upto: usize, epoch: u64 },
    /// Phrase en cours : aperçu.
    Preview { audio: Vec<f32> },
    /// Fin de la dictée : assembler le texte.
    Done,
}

/// Ce qui identifie un moteur chargé : un modèle local, ou une API et ses réglages.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EngineKey {
    /// Modèle Whisper sur le disque.
    Local(Model),
    /// API de transcription, avec ses réglages.
    Api(ApiConfig),
}

/// Le moteur que demandent ces réglages.
pub(crate) fn engine_key(config: &Config) -> EngineKey {
    match config.model {
        Model::Api => EngineKey::Api(config.api.clone()),
        model => EngineKey::Local(model),
    }
}

impl Stt {
    /// Stores the text of final phrase `index`, unless its utterance was discarded meanwhile.
    fn store_final(&self, g: &mut Inner, index: usize, epoch: u64, result: Result<Transcript, SttError>) {
        match result {
            Ok(_) if g.epoch != epoch => {}
            Ok(t) => {
                // La première phrase fixe la langue pour la suite.
                if g.detected.is_none() && g.config.language.is_none() {
                    g.detected.clone_from(&t.language);
                }
                g.phrases.set(index, keep(&t));
                g.partial.clear();
            }
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "transcription");
                g.error = Some(e.localize(&self.localizer));
            }
        }
    }

    /// Moteur chargé une fois (et gardé tant qu'on ne change ni de modèle ni d'API).
    pub(crate) fn engine(&self) -> Result<Arc<dyn PhraseEngine>, SttError> {
        static LOADING: Mutex<()> = Mutex::new(());
        let key = {
            let g = self.state();
            let key = engine_key(&g.config);
            if let Some((k, e)) = &g.engine
                && *k == key
            {
                return Ok(e.clone());
            }
            key
        };
        let _loading = LOADING.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((k, e)) = &self.state().engine
            && *k == key
        {
            return Ok(e.clone());
        }
        let started = Instant::now();
        let engine: Arc<dyn PhraseEngine> = match &key {
            EngineKey::Api(config) => {
                // Négatif, NaN ou démesuré : erreur de réglage, pas de panique.
                let timeout = Duration::try_from_secs_f32(self.settings.api_timeout_secs)
                    .map_err(|_| SttError::from(ZeroSettingSnafu { field: "stt.api_timeout_secs" }.build()))?;
                Arc::new(ApiTranscriber::new(config.clone(), timeout)?)
            }
            EngineKey::Local(model) => Arc::new(Transcriber::load(&model.path(&self.dir), &self.storage)?),
        };
        tracing::debug!(?key, ms = started.elapsed().as_millis(), "moteur de dictée chargé");
        self.state().engine = Some((key, engine.clone()));
        Ok(engine)
    }

    /// Queues a closed phrase for transcription; false when it is too short to keep.
    pub(crate) fn send_final(&self, seg: &Segmenter, phrase: Phrase, jobs: &mpsc::Sender<Job>) -> bool {
        // Un simple claquement ne fait pas une phrase.
        if phrase.seconds() < self.settings.min_phrase_secs {
            return false;
        }
        let (index, epoch) = {
            let mut g = self.state();
            (g.phrases.push(Spoken { text: None, start: phrase.start, end: phrase.end }), g.epoch)
        };
        let Some(index) = index else {
            tracing::error!("phrase indices exhausted: phrase dropped");
            return false;
        };
        jobs.send(Job::Final { index, audio: seg.slice(phrase).to_vec(), epoch }).ok();
        true
    }

    /// Transcription : phrases closes en qualité finale, aperçu de la phrase en cours.
    pub(crate) fn transcribe_loop(&self, session: u64, rx: &mpsc::Receiver<Job>, busy: &Arc<AtomicBool>) {
        // Chargement du modèle pendant qu'on commence à parler.
        let engine = match self.engine() {
            Ok(e) => e,
            Err(e) => {
                let mut g = self.state();
                if g.session == session {
                    g.error = Some(e.localize(&self.localizer));
                    g.state = State::Idle;
                    g.session += 1;
                    // The duplex closes with the capture: its voice no longer reads Oku.
                    g.barge = None;
                }
                return;
            }
        };
        let mut pending: Vec<Job> = Vec::new();
        loop {
            // Les phrases closes passent avant l'aperçu ; un aperçu périmé est sauté.
            while let Ok(job) = rx.try_recv() {
                pending.push(job);
            }
            if pending.is_empty() {
                match rx.recv() {
                    Ok(job) => pending.push(job),
                    Err(_) => return,
                }
                continue;
            }
            // Closed phrases first, then utterance closes in order, then the latest preview.
            let pick = pending
                .iter()
                .position(|j| matches!(j, Job::Final { .. }))
                .or_else(|| pending.iter().position(|j| matches!(j, Job::Flush { .. })))
                .or_else(|| pending.iter().rposition(|j| matches!(j, Job::Preview { .. })))
                .unwrap_or(0);
            let job = pending.remove(pick);
            if matches!(job, Job::Preview { .. }) {
                pending.retain(|j| !matches!(j, Job::Preview { .. }));
            }
            if !self.alive(session) {
                return;
            }
            let (language, context) = {
                let g = self.state();
                let lang = g.config.language.clone().or_else(|| g.detected.clone());
                (lang, context_of(&g.vocabulary, &g.committed()))
            };
            match job {
                Job::Final { index, audio, epoch } => {
                    if self.state().epoch != epoch {
                        continue; // discarded utterance: not worth transcribing
                    }
                    let result = engine.transcribe(&audio, language.as_deref(), Some(&context), false);
                    let mut g = self.state();
                    if g.session != session {
                        return;
                    }
                    self.store_final(&mut g, index, epoch, result);
                }
                Job::Preview { audio } => {
                    let result = engine.transcribe(&audio, language.as_deref(), Some(&context), true);
                    busy.store(false, Ordering::Relaxed);
                    let mut g = self.state();
                    if g.session != session {
                        return;
                    }
                    if let Ok(t) = result
                        && g.state == State::Recording
                    {
                        g.partial = keep(&t);
                    }
                }
                Job::Flush { upto, epoch } => {
                    let mut g = self.state();
                    if g.session != session {
                        return;
                    }
                    if g.epoch == epoch {
                        let spoken = g.phrases.take_upto(upto);
                        g.close_utterance(&spoken, self.settings.min_phrase_secs);
                    }
                }
                Job::Done => {
                    if pending.iter().any(|j| matches!(j, Job::Final { .. })) {
                        // Des phrases restent : on les traite d'abord.
                        pending.push(Job::Done);
                        continue;
                    }
                    let mut g = self.state();
                    if g.session != session {
                        return;
                    }
                    g.complete();
                    return;
                }
            }
        }
    }
}
