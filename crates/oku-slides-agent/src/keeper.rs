//! Le gardien d'un onglet : seul passage vers la conversation de la session.
//!
//! Une tâche qui applique ses ordres un par un, hors du fil async : l'ordre de la
//! file règle les courses (écriture puis oubli, oubli puis lecture).
//! Piloté par la pile ; public pour les tests d'intégration.

use std::sync::Arc;

use oku_slides_mcp::SurfaceHub;
use oku_slides_session::agent::TabSession;
use tokio::sync::{mpsc, oneshot, watch};

/// Un ordre au gardien.
#[derive(Debug)]
pub enum Keep {
    /// Fin du tour `n` : écrire le fil de l'onglet, s'il contient encore ce tour.
    Thread(u32),
    /// La session ACP ouverte pour le tour `turn` et l'empreinte de son agent.
    Session {
        /// Le tour qui l'a ouverte : effacé entre-temps, la session n'est pas gardée.
        turn: u32,
        /// L'identifiant de la session ACP.
        id: String,
        /// L'empreinte de l'agent.
        agent: String,
    },
    /// Nouvelle conversation : oubliée, son fil effacé du hub ; rend l'id de session oublié.
    Forget(oneshot::Sender<Option<String>>),
    /// La session connue et l'empreinte de son agent, dans l'ordre de la file.
    Lookup(oneshot::Sender<Option<(String, Option<String>)>>),
    /// Montrer le fil de l'onglet à l'interface.
    Show,
    /// Attendre que les ordres précédents soient appliqués.
    #[doc(hidden)]
    Flush(oneshot::Sender<()>),
}

/// Le gardien d'un onglet, vu par l'acteur, le relais et la pile.
#[derive(Debug, Clone)]
pub struct Keeper {
    tx: mpsc::UnboundedSender<Keep>,
    restored: watch::Receiver<bool>,
}

impl Keep {
    /// Ce qui n'a pas été fait si l'ordre échoue sur la session.
    fn failure(&self) -> &'static str {
        match self {
            Self::Thread(_) => "fil de la conversation non enregistré",
            Self::Session { .. } => "session de l'agent non enregistrée",
            Self::Forget(_) => "conversation non oubliée",
            Self::Lookup(_) => "session de l'agent non relue",
            Self::Show | Self::Flush(_) => "ordre du gardien non appliqué",
        }
    }
}

impl Keeper {
    /// Démarre le gardien : il relit d'abord la conversation et la rend au hub.
    pub fn spawn(tab: TabSession, hub: Arc<SurfaceHub>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (done, restored) = watch::channel(false);
        tokio::spawn(run(tab, hub, rx, done));
        Self { tx, restored }
    }

    /// Envoie un ordre ; un gardien arrêté (onglet fermé) l'ignore.
    pub fn send(&self, k: Keep) {
        if self.tx.send(k).is_err() {
            tracing::debug!("gardien arrêté : ordre ignoré");
        }
    }

    /// Attend la relecture de la conversation.
    pub async fn restored(&self) {
        let mut rx = self.restored.clone();
        if rx.wait_for(|done| *done).await.is_err() {
            tracing::debug!("gardien arrêté avant la relecture");
        }
    }

    /// Attend que les ordres déjà envoyés soient appliqués.
    pub async fn flush(&self) {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Flush(tx));
        if rx.await.is_err() {
            tracing::debug!("gardien arrêté pendant l'attente");
        }
    }

    /// La session connue (id, empreinte de l'agent), après les ordres déjà envoyés.
    pub async fn lookup(&self) -> Option<(String, Option<String>)> {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Lookup(tx));
        rx.await.ok().flatten()
    }

    /// Oublie la conversation et efface son fil du hub ; rend l'id de session oublié.
    pub async fn forget(&self) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Forget(tx));
        rx.await.ok().flatten()
    }
}

async fn run(tab: TabSession, hub: Arc<SurfaceHub>, mut rx: mpsc::UnboundedReceiver<Keep>, done: watch::Sender<bool>) {
    let (t, h) = (tab.clone(), hub.clone());
    match tokio::task::spawn_blocking(move || t.with(|s| s.conversation().cloned())).await {
        Ok(Ok(saved)) => h.restore(saved),
        Ok(Err(e)) => tracing::warn!(error = %e.one_line(), "conversation de l'agent non relue"),
        Err(e) => tracing::warn!(error = %e, "relecture de la conversation perdue"),
    }
    done.send_replace(true);
    while let Some(k) = rx.recv().await {
        let (t, h) = (tab.clone(), hub.clone());
        if let Err(e) = tokio::task::spawn_blocking(move || apply(&t, &h, k)).await {
            tracing::warn!(error = %e, "ordre du gardien perdu");
        }
    }
}

/// Applique un ordre à la session et au hub de l'onglet.
fn apply(tab: &TabSession, hub: &SurfaceHub, k: Keep) {
    let failure = k.failure();
    let written = match k {
        // Tour effacé entre-temps (nouvelle conversation) : rien à écrire.
        Keep::Thread(turn) if !hub.in_thread(turn) => Ok(()),
        Keep::Thread(_) => match hub.thread_snapshot() {
            Some(thread) => tab.with(|s| s.keep_thread(thread)),
            None => {
                tracing::warn!("fil trop lourd pour le budget : enregistrement sauté");
                Ok(())
            }
        },
        Keep::Session { turn, id, agent } => {
            if hub.in_thread(turn) {
                tab.with(|s| s.keep_agent_session(id, agent))
            } else {
                // Tour effacé entre-temps (nouvelle conversation) : sa session n'est plus à garder.
                tracing::debug!(turn, "session d'un tour effacé : non enregistrée");
                Ok(())
            }
        }
        Keep::Forget(reply) => {
            let gone = tab.with(|s| s.forget_conversation());
            // Dans l'ordre de la file : une fin de tour arrivée après l'oubli ne trouve plus son tour.
            hub.clear_thread();
            if reply.send(gone.as_ref().ok().cloned().flatten()).is_err() {
                tracing::debug!("oubli : l'acteur est parti (onglet fermé)");
            }
            gone.map(drop)
        }
        Keep::Lookup(reply) => tab.with(|s| {
            let found = s.conversation().and_then(|c| Some((c.session.clone()?, c.agent.clone())));
            if reply.send(found).is_err() {
                tracing::debug!("lecture : l'acteur est parti (onglet fermé)");
            }
        }),
        Keep::Show => {
            hub.resync();
            Ok(())
        }
        Keep::Flush(ack) => {
            if ack.send(()).is_err() {
                tracing::debug!("attente du gardien abandonnée");
            }
            Ok(())
        }
    };
    if let Err(e) = written {
        tracing::warn!(error = %e.one_line(), "{failure}");
    }
}
