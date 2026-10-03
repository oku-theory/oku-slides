//! Dictée par Whisper : phrases closes en qualité finale, aperçu de la phrase en cours.
//!
//! `listen_loop` (lib.rs) envoie des [`Job`] ; ce fil les transcrit dans l'ordre,
//! les phrases closes avant l'aperçu.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Instant;

use oku_slides_i18n::Localize;

use crate::vad::{Phrase, Segmenter};
use crate::{State, Stt, SttError, Transcriber, context_of, keep};

/// Travail pour le fil de transcription.
pub(crate) enum Job {
    /// Phrase close : texte définitif, rangé à l'indice donné.
    Final { index: usize, audio: Vec<f32> },
    /// Phrase en cours : aperçu.
    Preview { audio: Vec<f32> },
    /// Fin de la dictée : assembler le texte.
    Done,
}

impl Stt {
    /// Modèle chargé une fois (et gardé tant qu'on ne change pas de modèle).
    pub(crate) fn engine(&self) -> Result<Arc<Transcriber>, SttError> {
        let model = {
            let g = self.state();
            if let Some((m, e)) = &g.engine
                && *m == g.config.model
            {
                return Ok(e.clone());
            }
            g.config.model
        };
        static LOADING: Mutex<()> = Mutex::new(());
        let _loading = LOADING.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((m, e)) = &self.state().engine
            && *m == model
        {
            return Ok(e.clone());
        }
        let started = Instant::now();
        let engine = Arc::new(Transcriber::load(&model.path(&self.dir), &self.storage)?);
        tracing::debug!(?model, ms = started.elapsed().as_millis(), "modèle de dictée chargé");
        self.state().engine = Some((model, engine.clone()));
        Ok(engine)
    }

    pub(crate) fn send_final(&self, seg: &Segmenter, phrase: Phrase, jobs: &mpsc::Sender<Job>) {
        // Un simple claquement ne fait pas une phrase.
        if phrase.seconds() < self.settings.min_phrase_secs {
            return;
        }
        let index = {
            let mut g = self.state();
            g.phrases.push(None);
            g.phrases.len() - 1
        };
        jobs.send(Job::Final { index, audio: seg.slice(phrase).to_vec() }).ok();
    }

    /// Transcription : phrases closes en qualité finale, aperçu de la phrase en cours.
    pub(crate) fn transcribe_loop(&self, session: u64, rx: mpsc::Receiver<Job>, busy: Arc<AtomicBool>) {
        // Chargement du modèle pendant qu'on commence à parler.
        let engine = match self.engine() {
            Ok(e) => e,
            Err(e) => {
                let mut g = self.state();
                if g.session == session {
                    g.error = Some(e.localize(&self.localizer));
                    g.state = State::Idle;
                    g.session += 1;
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
            let pick = pending
                .iter()
                .position(|j| matches!(j, Job::Final { .. }))
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
                Job::Final { index, audio } => {
                    let result = engine.transcribe(&audio, language.as_deref(), Some(&context), false);
                    let mut g = self.state();
                    if g.session != session {
                        return;
                    }
                    match result {
                        Ok(t) => {
                            // La première phrase fixe la langue pour la suite.
                            if g.detected.is_none() && g.config.language.is_none() {
                                g.detected = t.language.clone();
                            }
                            g.phrases[index] = Some(keep(&t));
                            g.partial.clear();
                        }
                        Err(e) => {
                            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "transcription");
                            g.error = Some(e.localize(&self.localizer));
                        }
                    }
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
