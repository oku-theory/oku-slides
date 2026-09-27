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

use crate::resume::{DocSnapshot, Resume, State};

/// Ce qu'il faut écrire : des documents (sous-dossier et instantané) et l'état.
pub(crate) struct Job {
    /// Vide : l'état seul (les documents n'ont pas changé).
    pub docs: Vec<(&'static str, DocSnapshot)>,
    pub state: State,
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
        let merged = match slot.job.take() {
            // Sans documents, le dépôt garde ceux du précédent, pas encore écrits.
            Some(prev) if job.docs.is_empty() => Job { docs: prev.docs, state: job.state },
            _ => job,
        };
        slot.job = Some(merged);
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
}

#[cfg(test)]
#[path = "resume_writer_tests.rs"]
mod resume_writer_tests;
