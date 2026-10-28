//! Le chemin des verrous : de l'espace de travail à la session d'un onglet.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use snafu::OptionExt;

use super::deck_packs::Forward;
use super::{Slot, Tab, Workspace};
use crate::error::{Result, UnknownTabSnafu};
use crate::views::TabInfo;
use crate::{Request, Session, TabDocument};

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

    /// A request for tab `tab`; `true`: the tab bar must be redrawn.
    ///
    /// Locks the workspace before and after, never during the call. A deck-pack
    /// tab's due save is then forwarded to its owner deck: the call returns once
    /// the owner has answered. Owners a forward changed go to `take_touched`.
    ///
    /// # Blocking
    ///
    /// Waits for the tab's session, then, for a forward, for its owner's.
    pub fn request(ws: &Mutex<Self>, tab: &str, req: Request) -> (Result<String>, bool) {
        let (reply, changed) = Self::acquire(ws, tab, Forward::Yes, |s| s.handle(req));
        (reply.and_then(|r| r), changed)
    }

    /// `f` on the tab's session, woken if needed; `true`: redraw the tab bar.
    ///
    /// Same discipline as [`Workspace::request`], which it shares: the
    /// workspace is locked before and after, never during `f`, and a deck-pack
    /// tab's due save is forwarded afterwards. After [`Workspace::settle`],
    /// refused as a closed tab (`UnknownTab`): its resume is written, nothing
    /// may change it any more.
    ///
    /// # Blocking
    ///
    /// Waits for the tab's session, then, for a forward, for its owner's.
    pub fn with_session<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool) {
        Self::acquire(ws, tab, Forward::Yes, f)
    }

    /// Les infos d'un onglet après usage de sa session ; `true` si elles ont changé.
    ///
    /// Elles vivent aussi dans `workspace.json` (la barre du prochain
    /// lancement) : écrites dès qu'elles changent.
    ///
    /// A deck-pack tab whose resume failed (its session no longer edits its
    /// pack) is set aside, and its owner shows `notice-tab-resume-failed`.
    pub(super) fn refresh(ws: &Mutex<Self>, tab: &str, info: TabInfo) -> bool {
        let mut w = Self::lock(ws);
        if w.lost_link(tab, &info) {
            let owner = w.tabs.get(tab).and_then(|t| t.info.document.deck_tab().map(str::to_owned));
            let text = w.platform.localizer.tr("notice-tab-resume-failed");
            drop(w);
            Self::set_aside(ws, &[tab.to_owned()]);
            if let Some(owner) = owner
                && let Err(e) = Self::with_quiet(ws, &owner, |s| s.set_notice(text)).0
            {
                tracing::warn!(tab = %owner, error = %e.one_line(), "set-aside notice not shown");
            }
            return true;
        }
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
        let (sessions, generation) = {
            let w = Self::lock(ws);
            let sessions: Vec<_> =
                tabs.iter().filter_map(|id| Some((id.clone(), w.session(id).ok()?, w.deck_links_of(id)))).collect();
            (sessions, w.layout_generation)
        };
        for (id, session, links) in sessions {
            let (info, refused) = {
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
                match slot.ready_with_links(generation, &links) {
                    Ok((s, refused)) => (s.tab_info(), refused),
                    Err(e) => {
                        tracing::warn!(tab = %id, error = %e.one_line(), "onglet endormi non relu");
                        continue;
                    }
                }
            };
            Self::set_aside(ws, &refused);
            Self::refresh(ws, &id, info);
        }
    }

    /// Sets aside the tabs an owner refused, then stops their resume, lock released.
    pub(super) fn set_aside(ws: &Mutex<Self>, tabs: &[String]) {
        if tabs.is_empty() {
            return;
        }
        let gone: Vec<(String, Tab)> = Self::lock(ws).set_aside_unlinked(tabs);
        for (id, tab) in gone {
            relock(&tab.session).stop_resume(&id);
        }
    }

    /// La session d'un onglet, peut-être endormie.
    pub(super) fn session(&self, tab: &str) -> Result<Arc<Mutex<Slot>>> {
        Ok(self.tabs.get(tab).map(|t| Arc::clone(&t.session)).context(UnknownTabSnafu { id: tab })?)
    }

    /// A tab's info after a request; `true` if it changed.
    ///
    /// A deck-pack tab keeps the owner and pack the workspace fixed: an info
    /// naming another owner is refused (logged), and its pack moves only
    /// through `retarget_deck_pack`.
    pub(super) fn note(&mut self, tab: &str, mut info: TabInfo) -> bool {
        let Some(t) = self.tabs.get_mut(tab) else { return false };
        if let TabDocument::DeckPack { deck_tab, pack } = &t.info.document {
            match &mut info.document {
                TabDocument::DeckPack { deck_tab: owner, pack: reported } if owner == deck_tab => *reported = *pack,
                _ => {
                    tracing::warn!(tab, "deck-pack tab info names another document: kept as cached");
                    return false;
                }
            }
        }
        if t.info == info {
            return false;
        }
        t.info = info;
        true
    }
}
