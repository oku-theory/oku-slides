//! Le chemin des verrous : de l'espace de travail à la session d'un onglet.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use snafu::OptionExt;

use super::{Slot, Workspace};
use crate::error::{Result, UnknownTabSnafu};
use crate::views::TabInfo;
use crate::{Request, Session};

/// Un verrou, repris s'il est empoisonné : une panique ne bloque pas l'app.
pub(super) fn relock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Workspace {
    /// Le verrou de l'espace de travail, repris s'il est empoisonné.
    ///
    /// Ne jamais le garder pendant un appel à `apply` ou `request` : le
    /// verrou n'est pas réentrant.
    pub fn lock(ws: &Mutex<Self>) -> MutexGuard<'_, Self> {
        relock(ws)
    }

    /// Une requête pour l'onglet `tab` ; `true` : la barre est à redessiner.
    ///
    /// Seule la session de l'onglet est verrouillée pendant le rendu ;
    /// l'espace de travail l'est avant et après, jamais en même temps.
    pub fn request(ws: &Mutex<Self>, tab: &str, req: Request) -> (Result<String>, bool) {
        let (reply, changed) = Self::with_session(ws, tab, |s| s.handle(req));
        (reply.and_then(|r| r), changed)
    }

    /// `f` sur la session de l'onglet (réveillée au besoin) ; `true` : la barre est à redessiner.
    ///
    /// Même discipline que [`Workspace::request`] : l'espace de travail est
    /// verrouillé avant et après, jamais pendant `f`.
    ///
    /// Après [`Workspace::settle`], refusée comme un onglet fermé
    /// (`UnknownTab`) : sa reprise est écrite, plus rien ne doit la changer.
    pub fn with_session<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool) {
        let (session, closed) = {
            let w = Self::lock(ws);
            match w.session(tab) {
                Ok(session) => (session, Arc::clone(&w.closed)),
                Err(e) => return (Err(e), false),
            }
        };
        let (out, info) = {
            let mut slot = relock(&session);
            // Relu sous le verrou de la session : `settle` pose `closed` avant
            // de vider chaque session sous ce même verrou ; vu faux ici, le
            // changement de `f` sera donc vidé après.
            if closed.load(Ordering::SeqCst) {
                return (Err(UnknownTabSnafu { id: tab }.build().into()), false);
            }
            match slot.ready() {
                Ok(s) => {
                    let out = f(s);
                    (Ok(out), Some(s.tab_info()))
                }
                Err(e) => (Err(e), None),
            }
        };
        let changed = info.is_some_and(|info| Self::refresh(ws, tab, info));
        (out, changed)
    }

    /// Les infos d'un onglet après usage de sa session ; `true` si elles ont changé.
    ///
    /// Elles vivent aussi dans `workspace.json` (la barre du prochain
    /// lancement) : écrites dès qu'elles changent.
    pub(super) fn refresh(ws: &Mutex<Self>, tab: &str, info: TabInfo) -> bool {
        let mut w = Self::lock(ws);
        let changed = w.note(tab, info);
        if changed {
            w.save_layout();
        }
        changed
    }

    /// Réveille les onglets endormis parmi `tabs`, hors du verrou de l'espace de travail.
    ///
    /// Avant une décision qui doit lire leur état réel (fermer sans
    /// question, remplacer un onglet vierge) : les infos enregistrées
    /// peuvent être en retard (arrêt brutal entre deux écritures). Un échec
    /// est journalisé ; l'onglet reste endormi et la décision prudente.
    pub(super) fn wake(ws: &Mutex<Self>, tabs: &[String]) {
        let sessions: Vec<(String, Arc<Mutex<Slot>>)> = {
            let w = Self::lock(ws);
            tabs.iter().filter_map(|id| Some((id.clone(), w.session(id).ok()?))).collect()
        };
        for (id, session) in sessions {
            let info = {
                // Jamais d'attente : une session occupée n'est pas endormie
                // (déjà relue, ou en train de l'être), la décision reste prudente.
                let mut slot = match session.try_lock() {
                    Ok(slot) => slot,
                    Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner(),
                    Err(std::sync::TryLockError::WouldBlock) => continue,
                };
                if !slot.is_dormant() {
                    continue;
                }
                match slot.ready() {
                    Ok(s) => s.tab_info(),
                    Err(e) => {
                        tracing::warn!(tab = %id, error = %e.one_line(), "onglet endormi non relu");
                        continue;
                    }
                }
            };
            Self::refresh(ws, &id, info);
        }
    }

    /// La session d'un onglet, peut-être endormie.
    pub(super) fn session(&self, tab: &str) -> Result<Arc<Mutex<Slot>>> {
        Ok(self.tabs.get(tab).map(|t| Arc::clone(&t.session)).context(UnknownTabSnafu { id: tab })?)
    }

    /// Les infos d'un onglet après une requête ; `true` si elles ont changé.
    pub(super) fn note(&mut self, tab: &str, info: TabInfo) -> bool {
        match self.tabs.get_mut(tab) {
            Some(t) if t.info != info => {
                t.info = info;
                true
            }
            _ => false,
        }
    }
}
