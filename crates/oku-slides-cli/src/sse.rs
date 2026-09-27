//! Évènements du serveur de développement en SSE : chaque client de
//! `GET /api/events` a son canal ; son corps de réponse bloque jusqu'au
//! prochain évènement.

use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, Weak};

/// Nombre maximal de clients SSE connectés à la fois : chacun a son fil OS
/// (voir `serve.rs`), sans limite le nombre de fils ne serait borné que par
/// le nombre de connexions ouvertes par des clients.
pub const MAX_SSE_CLIENTS: usize = 8;

/// Un client, son canal et un témoin de vie : `SseReader` garde l'autre
/// moitié (`Arc`) de ce témoin, sa disparition dit qu'il est parti sans
/// attendre le prochain `send`.
type Client = (Sender<String>, Weak<()>);

/// Les clients SSE connectés.
#[derive(Debug, Default, Clone)]
pub struct Clients(Arc<Mutex<Vec<Client>>>);

impl Clients {
    /// Un nouveau client : le corps de sa réponse, ou `None` si
    /// `MAX_SSE_CLIENTS` sont déjà servis (après avoir oublié ceux partis).
    pub fn subscribe(&self) -> Option<SseReader> {
        let (tx, rx) = channel();
        let alive = Arc::new(());
        let mut list = self.0.lock().ok()?;
        list.retain(|(_, w)| w.strong_count() > 0);
        if list.len() >= MAX_SSE_CLIENTS {
            return None;
        }
        list.push((tx, Arc::downgrade(&alive)));
        Some(SseReader { rx, pending: Vec::new(), at: 0, _alive: alive })
    }

    /// Envoie `event: name` / `data: json` à chacun ; oublie ceux qui sont partis.
    pub fn send(&self, name: &str, json: &str) {
        let frame = format!("event: {name}\ndata: {json}\n\n");
        if let Ok(mut list) = self.0.lock() {
            list.retain(|(tx, _)| tx.send(frame.clone()).is_ok());
        }
    }

    /// Nombre d'abonnés (tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.lock().map(|l| l.len()).unwrap_or(0)
    }
}

/// Corps d'une réponse SSE : bloque jusqu'au prochain évènement ; fin quand
/// plus personne n'envoie.
#[derive(Debug)]
pub struct SseReader {
    rx: Receiver<String>,
    pending: Vec<u8>,
    at: usize,
    /// Tenu pour la durée de vie du lecteur : sa disparition libère une
    /// place dans `Clients::subscribe`.
    _alive: Arc<()>,
}

impl Read for SseReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.at >= self.pending.len() {
            match self.rx.recv() {
                Ok(frame) => {
                    self.pending = frame.into_bytes();
                    self.at = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let n = buf.len().min(self.pending.len() - self.at);
        buf[..n].copy_from_slice(&self.pending[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod sse_tests;
