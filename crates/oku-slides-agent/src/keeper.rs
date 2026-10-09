//! Le gardien d'un onglet : seul passage vers les conversations de la session.
//!
//! Une tâche qui applique ses ordres un par un, hors du fil async : l'ordre de la
//! file règle les courses (écriture puis oubli, renommage puis écriture, oubli puis lecture).
//! Piloté par la pile ; public pour les tests d'intégration.

use std::sync::Arc;

use oku_slides_mcp::SurfaceHub;
use oku_slides_session::agent::{AgentScope, ScopeMove, TabSession};
use tokio::sync::{mpsc, oneshot, watch};

/// Un ordre au gardien.
#[derive(Debug)]
pub enum Keep {
    /// Fin du tour `n` : écrire le fil de sa portée (lue au moment d'écrire).
    Thread(u32),
    /// La session ACP ouverte pour le tour `turn` et l'empreinte de son agent.
    Session {
        /// Le tour qui l'a ouverte : sa portée est lue au moment d'écrire (renommages compris).
        turn: u32,
        /// L'identifiant de la session ACP.
        id: String,
        /// L'empreinte de l'agent.
        agent: String,
    },
    /// Nouvelle conversation dans `scope` : oubliée, son fil effacé du hub ; rend l'id de session oublié.
    Forget(AgentScope, oneshot::Sender<Option<String>>),
    /// La session connue de `scope` et l'empreinte de son agent, dans l'ordre de la file.
    Lookup(AgentScope, oneshot::Sender<Option<(String, Option<String>)>>),
    /// Montrer le fil de `scope` à l'interface.
    Show(AgentScope),
    /// Rien d'autre que les renommages notés par la session : envoyé à chaque changement du deck,
    /// pour qu'un ordre plus tardif visant un nouveau widget du nom d'avant ne soit jamais suivi.
    Sync,
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
            Self::Session { .. } => "session de la portée non enregistrée",
            Self::Forget(..) => "conversation de la portée non oubliée",
            Self::Lookup(..) => "session de la portée non relue",
            Self::Show(_) | Self::Sync | Self::Flush(_) => "ordre du gardien non appliqué",
        }
    }
}

impl Keeper {
    /// Démarre le gardien : il relit d'abord les conversations et les rend au hub.
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

    /// Attend la relecture des conversations.
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

    /// La session connue de `scope` (id, empreinte de l'agent), après les ordres déjà envoyés.
    pub async fn lookup(&self, scope: AgentScope) -> Option<(String, Option<String>)> {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Lookup(scope, tx));
        rx.await.ok().flatten()
    }

    /// Oublie la conversation de `scope` et efface son fil du hub ; rend l'id de session oublié.
    pub async fn forget(&self, scope: AgentScope) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        self.send(Keep::Forget(scope, tx));
        rx.await.ok().flatten()
    }
}

async fn run(tab: TabSession, hub: Arc<SurfaceHub>, mut rx: mpsc::UnboundedReceiver<Keep>, done: watch::Sender<bool>) {
    let (t, h) = (tab.clone(), hub.clone());
    // Les conversations relues portent déjà les nouveaux noms : les renommages d'avant sont sans objet.
    let read = move || {
        t.with(|s| {
            drop(s.take_scope_moves());
            s.conversations()
        })
    };
    match tokio::task::spawn_blocking(read).await {
        Ok(Ok(saved)) => h.restore(saved),
        Ok(Err(e)) => tracing::warn!(error = %e.one_line(), "conversations de l'agent non relues"),
        Err(e) => tracing::warn!(error = %e, "relecture des conversations perdue"),
    }
    done.send_replace(true);
    while let Some(k) = rx.recv().await {
        let (t, h) = (tab.clone(), hub.clone());
        if let Err(e) = tokio::task::spawn_blocking(move || apply(&t, &h, k)).await {
            tracing::warn!(error = %e, "ordre du gardien perdu");
        }
    }
}

/// Applique un ordre : d'abord les renommages notés par la session, au hub et à l'ordre.
///
/// Un ordre envoyé avant un renommage suit le widget renommé ; [`Keep::Sync`], envoyé à chaque
/// changement du deck, applique les renommages avant les ordres qui le suivent.
fn apply(tab: &TabSession, hub: &SurfaceHub, k: Keep) {
    let failure = k.failure();
    let moves = match tab.with(|s| s.take_scope_moves()) {
        Ok(moves) => moves,
        Err(e) => {
            tracing::warn!(error = %e.one_line(), "renommages des conversations non lus");
            Vec::new()
        }
    };
    let follow = |mut scope: AgentScope| {
        for ScopeMove::Renamed { from, to } in &moves {
            if scope == *from {
                scope = to.clone();
            }
        }
        scope
    };
    for ScopeMove::Renamed { from, to } in &moves {
        hub.rename_scope(from, to);
    }
    let written = match k {
        Keep::Thread(turn) => match hub.turn_scope(turn).map(|scope| (hub.thread_snapshot(&scope), scope)) {
            Some((Some(thread), scope)) => tab.with(|s| s.keep_thread(scope, thread)),
            Some((None, scope)) => {
                tracing::warn!(?scope, "fil trop lourd pour le budget : enregistrement sauté");
                Ok(())
            }
            None => Ok(()), // tour effacé entre-temps (nouvelle conversation) : rien à écrire
        },
        Keep::Session { turn, id, agent } => match hub.turn_scope(turn) {
            Some(scope) => tab.with(|s| s.keep_agent_session(scope, id, agent)),
            None => {
                // Tour effacé entre-temps (nouvelle conversation) : sa session n'est plus à garder.
                tracing::debug!(turn, "session d'un tour effacé : non enregistrée");
                Ok(())
            }
        },
        Keep::Forget(scope, reply) => {
            let scope = follow(scope);
            let gone = tab.with(|s| s.forget_conversation(&scope));
            // Dans l'ordre de la file : une fin de tour arrivée après l'oubli ne trouve plus son tour.
            hub.clear_thread(&scope);
            if reply.send(gone.as_ref().ok().cloned().flatten()).is_err() {
                tracing::debug!("oubli : l'acteur est parti (onglet fermé)");
            }
            gone.map(drop)
        }
        Keep::Lookup(scope, reply) => tab.with(|s| {
            let found = s.conversation(&follow(scope)).and_then(|c| Some((c.session.clone()?, c.agent.clone())));
            if reply.send(found).is_err() {
                tracing::debug!("lecture : l'acteur est parti (onglet fermé)");
            }
        }),
        Keep::Show(scope) => {
            hub.resync(follow(scope));
            Ok(())
        }
        Keep::Sync => Ok(()),
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
