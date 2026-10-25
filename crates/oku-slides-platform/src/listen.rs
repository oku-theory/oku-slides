//! Boucle d'accept des serveurs HTTP locaux (serveur de développement,
//! outils de l'agent) : chaque connexion HTTP/1 a un délai pour envoyer ses
//! en-têtes, et le nombre de connexions simultanées est plafonné (au-delà, une
//! nouvelle connexion est fermée aussitôt). L'arrêt ferme l'écoute, puis
//! chaque connexion une fois sa réponse en cours rendue, dans un délai de
//! grâce ; ce qui reste ensuite est abandonné.
//!
//! `axum::serve` n'offre ni l'un ni l'autre : une connexion muette y restait
//! ouverte sans fin, et rien ne bornait leur nombre (t-446, t-637).

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::graceful::GracefulShutdown;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Pause après une erreur d'accept (trop de fichiers ouverts…), pour ne pas
/// tourner à vide en attendant qu'une connexion se ferme.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

/// Les bornes des connexions (`[serve]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Délai pour recevoir les en-têtes d'une requête.
    pub header_timeout: Duration,
    /// Connexions simultanées au plus.
    pub max_connections: usize,
}

impl Limits {
    /// Bornes réglées ; le plafond est ramené à ce qu'un sémaphore accepte.
    #[must_use]
    pub fn new(header_timeout: Duration, max_connections: usize) -> Self {
        Self { header_timeout, max_connections: max_connections.min(Semaphore::MAX_PERMITS) }
    }
}

/// Sert `app` sur `listener` jusqu'à ce que `shutdown` se termine, puis laisse
/// aux connexions ouvertes `grace` pour finir leur réponse ; les autres
/// (flux sans fin, réponse bloquée) sont alors fermées. Rend la main une fois
/// toutes les connexions fermées : vrai si la grâce n'a pas suffi et que des
/// connexions ont été abandonnées. À l'appelant de dire si c'est anormal (un
/// flux SSE encore ouvert à l'arrêt du serveur d'outils est attendu).
pub async fn serve_connections(
    listener: TcpListener,
    app: Router,
    limits: Limits,
    grace: Duration,
    shutdown: impl Future<Output = ()>,
) -> bool {
    let places = Arc::new(Semaphore::new(limits.max_connections));
    let graceful = GracefulShutdown::new();
    let mut builder = http1::Builder::new();
    builder.timer(TokioTimer::new()).header_read_timeout(limits.header_timeout);
    let mut shutdown = pin!(shutdown);
    // Les connexions servies, pour pouvoir abandonner celles qui survivent à la grâce.
    let mut connections = JoinSet::new();
    loop {
        let (stream, peer) = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(accepted) => accepted,
                Err(e) => {
                    // `warn` : trop de fichiers ouverts (EMFILE) doit se voir ; la pause en limite le rythme.
                    tracing::warn!(error = %e, "connexion non acceptée");
                    tokio::time::sleep(ACCEPT_BACKOFF).await;
                    continue;
                }
            },
            // Les connexions finies ne restent pas dans l'ensemble.
            Some(joined) = connections.join_next() => {
                if let Err(e) = joined
                    && e.is_panic()
                {
                    tracing::error!(error = %e, "tâche de connexion en panique");
                }
                continue;
            }
            () = &mut shutdown => break,
        };
        let Ok(place) = Arc::clone(&places).try_acquire_owned() else {
            // `debug` : un client qui ouvre des connexions en boucle remplirait sinon le journal.
            tracing::debug!(%peer, "trop de connexions simultanées : fermée");
            drop(stream);
            continue;
        };
        let connection = builder.serve_connection(TokioIo::new(stream), TowerToHyperService::new(app.clone()));
        let connection = graceful.watch(connection);
        connections.spawn(async move {
            // Délai d'en-têtes dépassé, client parti : rien à signaler.
            if let Err(e) = connection.await {
                tracing::debug!(%peer, error = %e, "connexion terminée sur une erreur");
            }
            drop(place);
        });
    }
    // Plus de nouvelles connexions ; les ouvertes finissent leur réponse dans
    // la grâce, les autres sont abandonnées (leur socket se ferme).
    drop(listener);
    let abandoned = tokio::time::timeout(grace, graceful.shutdown()).await.is_err();
    if abandoned {
        tracing::debug!(connections = connections.len(), "arrêt : grâce dépassée, connexions abandonnées");
    }
    connections.shutdown().await;
    abandoned
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod listen_tests;
