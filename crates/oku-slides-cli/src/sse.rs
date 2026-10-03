//! Évènements du serveur de développement en SSE : chaque client de
//! `GET /api/events` a son canal ; son corps de réponse est un flux qui livre
//! chaque évènement dès qu'il est envoyé.

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use axum::body::{Bytes, HttpBody};
use http_body::Frame;
use tokio::sync::mpsc::{Receiver, Sender, channel};

/// Nombre maximal de clients SSE connectés à la fois : chacun garde une
/// connexion et un canal ouverts ; sans limite, leur nombre ne serait borné
/// que par celui des connexions ouvertes par des clients.
pub const MAX_SSE_CLIENTS: usize = 8;

/// Trames en attente par client : un client qui ne lit plus est oublié une
/// fois sa file pleine (son EventSource se reconnecte), au lieu de faire
/// grossir la mémoire sans fin.
pub const CHANNEL_CAPACITY: usize = 256;

#[derive(Debug, Default)]
struct Inner {
    /// Un canal par client ; fermé quand son corps de réponse est lâché.
    list: Vec<Sender<Bytes>>,
    /// `close` est passé : plus aucun abonnement.
    closed: bool,
}

/// Les clients SSE connectés. `send` s'appelle depuis n'importe quel fil.
#[derive(Debug, Default, Clone)]
pub struct Clients(Arc<Mutex<Inner>>);

impl Clients {
    /// Un nouveau client : le corps de sa réponse, ou `None` si
    /// `MAX_SSE_CLIENTS` sont déjà servis (après avoir oublié ceux partis) ou
    /// si le serveur s'arrête.
    pub fn subscribe(&self) -> Option<SseBody> {
        let mut inner = self.0.lock().ok()?;
        inner.list.retain(|tx| !tx.is_closed());
        if inner.closed || inner.list.len() >= MAX_SSE_CLIENTS {
            return None;
        }
        let (tx, rx) = channel(CHANNEL_CAPACITY);
        inner.list.push(tx);
        Some(SseBody { rx })
    }

    /// Envoie `event: name` / `data: json` à chacun, sans attendre ; oublie
    /// ceux qui sont partis ou dont la file est pleine.
    pub fn send(&self, name: &str, json: &str) {
        let frame = Bytes::from(format!("event: {name}\ndata: {json}\n\n"));
        if let Ok(mut inner) = self.0.lock() {
            inner.list.retain(|tx| tx.try_send(frame.clone()).is_ok());
        }
    }

    /// Termine tous les flux et refuse les suivants : l'arrêt gracieux du
    /// serveur attend la fin de chaque réponse, un flux ouvert le bloquerait.
    pub fn close(&self) {
        if let Ok(mut inner) = self.0.lock() {
            inner.closed = true;
            inner.list.clear();
        }
    }

    /// Nombre d'abonnés (tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.lock().map(|i| i.list.len()).unwrap_or(0)
    }
}

/// Corps d'une réponse SSE : une trame par évènement, livrée (et vidée par
/// le serveur) dès qu'elle arrive ; fin quand plus personne n'envoie. Le
/// lâcher (client parti) libère une place dans `Clients::subscribe`.
#[derive(Debug)]
pub struct SseBody {
    rx: Receiver<Bytes>,
}

impl HttpBody for SseBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        self.rx.poll_recv(cx).map(|frame| frame.map(|bytes| Ok(Frame::data(bytes))))
    }
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod sse_tests;
