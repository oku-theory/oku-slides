//! Écriture de la reprise hors du chemin des requêtes (t-144).
//!
//! Chaque écriture atomique coûte quelques millisecondes (synchronisation
//! disque) : sur un gros deck, la reprise ajoutait ~17 ms à chaque commande.
//! Une session dépose désormais son état dans une case unique, qu'un fil
//! dédié écrit ; un dépôt fait pendant une écriture remplace le précédent
//! (le plus récent seul est écrit). Mémoire bornée : un état en attente et
//! un en cours d'écriture, au plus. L'ordre sûr en cas d'arrêt brutal ne
//! change pas : [`Resume::write_snapshot`] (fichiers, puis `deck.loro`, puis
//! `document.json`, puis orphelins), puis `state.json`.
//!
//! Un seul écrivain par dossier : le fil. Le rendre (fin de session, onglet
//! fermé) écrit d'abord ce qui reste, et plus rien ensuite.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use crate::agent::SavedConversation;
use crate::resume::{DocSnapshot, Resume, State};

/// Ce qu'il faut faire de la conversation de l'agent.
#[derive(Debug, Clone)]
pub(crate) enum AgentWrite {
    /// L'écrire.
    Write(SavedConversation),
    /// L'effacer.
    Delete,
}

/// Ce qu'il faut écrire : des documents (sous-dossier et instantané) et l'état.
pub(crate) struct Job {
    /// Vide : l'état seul (les documents n'ont pas changé).
    pub docs: Vec<(&'static str, DocSnapshot)>,
    pub state: State,
    /// La conversation à écrire ou effacer ; `None` : inchangée.
    pub agent: Option<AgentWrite>,
}

/// Fusionne deux dépôts : le plus récent l'emporte, sans perdre la conversation.
pub(crate) fn merge(prev: Job, next: Job) -> Job {
    // Sans documents, le dépôt garde ceux du précédent, pas encore écrits.
    let docs = if next.docs.is_empty() { prev.docs } else { next.docs };
    Job { docs, state: next.state, agent: next.agent.or(prev.agent) }
}

/// La case partagée entre la session et son fil.
#[derive(Default)]
struct Slot {
    job: Option<Job>,
    /// Dépôts faits, et dépôts écrits (ou abandonnés sur erreur) : `flush`
    /// attend que le second rattrape le premier.
    sent: u64,
    written: u64,
    /// La session rend le fil : il écrit ce qui reste, puis s'arrête.
    closed: bool,
}

#[derive(Default)]
struct Shared {
    slot: Mutex<Slot>,
    changed: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Le fil qui écrit la reprise d'une session.
pub(crate) struct Writer {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    /// Sans fil (création impossible) : écriture sur place, comme avant.
    inline: Option<Resume>,
}

impl std::fmt::Debug for Writer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Writer").field("threaded", &self.thread.is_some()).finish_non_exhaustive()
    }
}

impl Writer {
    /// Un fil d'écriture pour `resume` ; s'il ne peut être créé, l'écriture
    /// reste sur place (journalisé).
    pub(crate) fn new(resume: Resume) -> Self {
        let shared = Arc::new(Shared::default());
        let fallback = resume.clone();
        let theirs = Arc::clone(&shared);
        match std::thread::Builder::new().name("oku-resume".into()).spawn(move || run(&resume, &theirs)) {
            Ok(thread) => Self { shared, thread: Some(thread), inline: None },
            Err(e) => {
                tracing::warn!(error = %e, "fil de reprise non créé : écriture sur place");
                Self { shared, thread: None, inline: Some(fallback) }
            }
        }
    }

    /// Dépose un état à écrire ; ne bloque pas (sauf repli sur place).
    pub(crate) fn send(&self, job: Job) {
        if let Some(resume) = &self.inline {
            write(resume, &job);
            return;
        }
        let mut slot = self.shared.lock();
        slot.job = Some(match slot.job.take() {
            Some(prev) => merge(prev, job),
            None => job,
        });
        slot.sent += 1;
        self.shared.changed.notify_all();
    }

    /// Attend que tout ce qui a été déposé soit écrit.
    pub(crate) fn flush(&self) {
        if self.thread.is_none() {
            return;
        }
        let mut slot = self.shared.lock();
        let target = slot.sent;
        while slot.written < target {
            slot = self.shared.changed.wait(slot).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

impl Drop for Writer {
    /// Écrit ce qui reste, puis arrête le fil.
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.changed.notify_all();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::warn!("fil de reprise arrêté par une panique");
        }
    }
}

/// La boucle du fil : attend un dépôt, l'écrit, recommence ; s'arrête une
/// fois la case vide et fermée.
fn run(resume: &Resume, shared: &Shared) {
    loop {
        let (job, upto) = {
            let mut slot = shared.lock();
            while slot.job.is_none() && !slot.closed {
                slot = shared.changed.wait(slot).unwrap_or_else(PoisonError::into_inner);
            }
            match slot.job.take() {
                Some(job) => (job, slot.sent),
                None => return,
            }
        };
        // Une panique d'écriture ne tue pas le fil : `flush` n'attendrait
        // plus jamais.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| write(resume, &job)));
        if outcome.is_err() {
            tracing::warn!("écriture de reprise interrompue par une panique");
        }
        shared.lock().written = upto;
        shared.changed.notify_all();
    }
}

fn write(resume: &Resume, job: &Job) {
    let written = job
        .docs
        .iter()
        .try_for_each(|(sub, doc)| resume.write_snapshot(sub, doc))
        .and_then(|()| resume.write_state(&job.state));
    if let Err(e) = written {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "état de reprise non enregistré");
    }
    // Hors de la chaîne ci-dessus : un état non écrit n'empêche pas la conversation.
    let done = match &job.agent {
        Some(AgentWrite::Write(c)) => resume.write_conversation(c),
        Some(AgentWrite::Delete) => resume.delete_conversation(),
        None => Ok(()),
    };
    if let Err(e) = done {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "conversation de l'agent non enregistrée");
    }
}

#[cfg(test)]
#[path = "resume_writer_tests.rs"]
mod resume_writer_tests;
