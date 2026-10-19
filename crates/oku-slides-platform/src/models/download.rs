//! Boucle de téléchargement : reprise, nouvelles tentatives, SHA-256 au fil de l'eau.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, sync_channel};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use snafu::{Backtrace, ResultExt, Snafu};

use super::{COMPLETE, DownloadErrorKind, PinnedFile};
use crate::config::ModelsSettings;
use crate::storage::{Storage, StorageError};

/// Un téléchargement de modèle qui a échoué.
#[derive(Debug, Snafu)]
pub struct DownloadError(Box<DownloadInner>);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
enum DownloadInner {
    #[snafu(display("cannot fetch {file}"))]
    Http { file: String, source: Box<ureq::Error>, backtrace: Backtrace },
    #[snafu(display("cannot read {file} from the network"))]
    HttpRead { file: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("{file}: nothing received from the hub for {secs} s"))]
    Stalled { file: String, secs: u64, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Write { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("incomplete download: {got} of {expected} bytes"))]
    Incomplete { got: u64, expected: u64, backtrace: Backtrace },
    #[snafu(display("{file} does not match its pinned SHA-256"))]
    Integrity { file: String, backtrace: Backtrace },
    #[snafu(display("download cancelled"))]
    Cancelled { backtrace: Backtrace },
}

impl DownloadError {
    /// Ce qui a échoué, pour le message montré à l'utilisateur.
    pub fn kind(&self) -> DownloadErrorKind {
        match &*self.0 {
            DownloadInner::Http { .. } | DownloadInner::HttpRead { .. } | DownloadInner::Stalled { .. } => {
                DownloadErrorKind::Network
            }
            DownloadInner::Write { .. } => DownloadErrorKind::Write,
            DownloadInner::Incomplete { got, expected, .. } => {
                DownloadErrorKind::Incomplete { got: *got, expected: *expected }
            }
            DownloadInner::Integrity { .. } => DownloadErrorKind::Integrity,
            DownloadInner::Cancelled { .. } => DownloadErrorKind::Cancelled,
        }
    }
}

/// Tranche d'attente : l'annulation est vérifiée au moins aussi souvent.
const SLICE: Duration = Duration::from_millis(200);

/// Ce que le fil d'une requête envoie : en-têtes, morceaux, fin ou échec.
enum Fetched {
    /// Réponse reçue ; sa taille annoncée.
    Head(Option<u64>),
    Chunk(Vec<u8>),
    End,
    Http(Box<ureq::Error>),
    Read(std::io::Error),
}

/// Pourquoi l'attente du fil d'une requête s'arrête sans message.
enum Stop {
    Cancelled,
    /// Rien reçu pendant `read_timeout_secs`.
    Stalled,
}

/// Lance la requête `url` sur un fil à part ; ses résultats arrivent par le canal rendu.
///
/// Une lecture bloquée garde le fil jusqu'à la fin de la connexion ; personne ne l'attend.
fn fetch(agent: &ureq::Agent, url: String) -> Receiver<Fetched> {
    let (tx, rx) = sync_channel(4);
    let agent = agent.clone();
    std::thread::spawn(move || {
        let mut resp = match agent.get(&url).call() {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Fetched::Http(Box::new(e)));
                return;
            }
        };
        let expected = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
        if tx.send(Fetched::Head(expected)).is_err() {
            return;
        }
        let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let msg = match reader.read(&mut buf) {
                Ok(0) => Fetched::End,
                Ok(n) => Fetched::Chunk(buf[..n].to_vec()),
                Err(e) => Fetched::Read(e),
            };
            let last = !matches!(msg, Fetched::Chunk(_));
            // Le téléchargement abandonné ne lit plus : on s'arrête.
            if tx.send(msg).is_err() || last {
                return;
            }
        }
    });
    rx
}

/// Le prochain message de `rx`, en vérifiant l'annulation (`progress(at, total)`) à chaque tranche.
fn next(
    rx: &Receiver<Fetched>,
    timeout: Duration,
    at: u64,
    total: u64,
    progress: &mut impl FnMut(u64, u64) -> bool,
) -> Result<Fetched, Stop> {
    let since = Instant::now();
    loop {
        match rx.recv_timeout(SLICE) {
            Ok(msg) => return Ok(msg),
            // Le fil envoie toujours un dernier message : un canal fermé est une connexion perdue.
            Err(RecvTimeoutError::Disconnected) => return Err(Stop::Stalled),
            Err(RecvTimeoutError::Timeout) => {}
        }
        if !progress(at.min(total), total) {
            return Err(Stop::Cancelled);
        }
        if since.elapsed() >= timeout {
            return Err(Stop::Stalled);
        }
    }
}

/// Attend `wait` par tranches ; faux si `progress` demande l'annulation entre-temps.
fn backoff(wait: Duration, at: u64, total: u64, progress: &mut impl FnMut(u64, u64) -> bool) -> bool {
    let until = Instant::now() + wait;
    loop {
        if !progress(at.min(total), total) {
            return false;
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        std::thread::sleep(left.min(SLICE));
    }
}

/// Télécharge `files` dans `dir` (chacun en `.part`, vérifié puis renommé ; les fichiers déjà
/// là sont gardés), puis écrit [`COMPLETE`]. Pas de reprise à l'octet : un fichier coupé repart
/// de zéro (t-1052). L'intégrité n'est vérifiée que pour les fichiers écrits ici ; un fichier
/// déjà présent est gardé d'après sa seule longueur.
/// `progress(reçu, total)` est appelé régulièrement, même en attente du dépôt ; s'il renvoie
/// `false`, on annule. Rien reçu pendant `read_timeout_secs` : échec réseau.
pub fn download(
    files: &[PinnedFile],
    dir: &Path,
    total: u64,
    settings: &ModelsSettings,
    storage: &Storage,
    mut progress: impl FnMut(u64, u64) -> bool,
) -> Result<(), DownloadError> {
    let timeout = Duration::from_secs(settings.read_timeout_secs);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(ureq::Proxy::try_from_env())
        // Bornes du fil de la requête ; le corps, lui, est borné lecture par lecture (`next`).
        .timeout_connect(Some(timeout))
        .timeout_recv_response(Some(timeout))
        .build()
        .into();
    let mut done = 0u64;
    for pinned in files {
        let file = pinned.name;
        let path = dir.join(file);
        if let Ok(Some(len)) = storage.len(&path) {
            done += len;
            continue;
        }
        let url =
            format!("{}/{}/resolve/{}/{file}", settings.hub_url.trim_end_matches('/'), pinned.repo, pinned.revision);
        let stopped = |stop: Stop| match stop {
            Stop::Cancelled => CancelledSnafu.build(),
            Stop::Stalled => StalledSnafu { file, secs: settings.read_timeout_secs }.build(),
        };
        // Hugging Face limite parfois le débit (429) : on réessaie en patientant.
        let mut attempt = 0;
        let (rx, expected) = loop {
            if !progress(done.min(total), total) {
                return CancelledSnafu.fail()?;
            }
            let rx = fetch(&agent, url.clone());
            match next(&rx, timeout, done, total, &mut progress).map_err(stopped)? {
                Fetched::Head(expected) => break (rx, expected),
                Fetched::Http(e) => match *e {
                    ureq::Error::StatusCode(429 | 500 | 502 | 503) if attempt < settings.download_retries => {
                        attempt += 1;
                        let wait = Duration::from_secs(settings.download_backoff_secs * u64::from(attempt));
                        if !backoff(wait, done, total, &mut progress) {
                            return CancelledSnafu.fail()?;
                        }
                    }
                    e => return Err(Box::new(e)).context(HttpSnafu { file })?,
                },
                Fetched::Read(e) => return Err(e).context(HttpReadSnafu { file })?,
                // Le fil envoie les en-têtes avant tout morceau.
                Fetched::Chunk(_) | Fetched::End => return Err(stopped(Stop::Stalled))?,
            }
        };
        let part = dir.join(format!("{file}.part"));
        let mut out = storage.writer(&part).context(WriteSnafu { path: &part })?;
        let mut hasher = pinned.sha256.map(|_| Sha256::new());
        let mut got = 0u64;
        loop {
            let fetched = next(&rx, timeout, done + got, total, &mut progress);
            let chunk = match fetched {
                Ok(Fetched::Chunk(chunk)) => chunk,
                Ok(Fetched::End) => break,
                failed => {
                    drop(out);
                    discard(storage, &part);
                    return match failed {
                        Ok(Fetched::Read(e)) => Err(e).context(HttpReadSnafu { file })?,
                        Err(stop) => Err(stopped(stop))?,
                        // Le fil n'envoie ni en-têtes ni échec HTTP après les en-têtes.
                        Ok(_) => Err(stopped(Stop::Stalled))?,
                    };
                }
            };
            if let Some(h) = hasher.as_mut() {
                h.update(&chunk);
            }
            out.write_all(&chunk).context(WriteSnafu { path: &part })?;
            got += chunk.len() as u64;
            if !progress((done + got).min(total), total) {
                drop(out);
                discard(storage, &part);
                return CancelledSnafu.fail()?;
            }
        }
        out.close().context(WriteSnafu { path: &part })?;
        if let Some(expected) = expected
            && got != expected
        {
            discard(storage, &part);
            return IncompleteSnafu { got, expected }.fail()?;
        }
        if let (Some(pin), Some(h)) = (pinned.sha256, hasher)
            && !hex::encode(h.finalize()).eq_ignore_ascii_case(pin)
        {
            discard(storage, &part);
            return IntegritySnafu { file }.fail()?;
        }
        storage.rename(&part, &path).context(WriteSnafu { path: &path })?;
        done += got;
    }
    let marker = dir.join(COMPLETE);
    storage.write(&marker, Vec::new()).context(WriteSnafu { path: &marker })?;
    progress(total, total);
    Ok(())
}

/// Retire un fichier partiel ; un échec ne change rien pour l'utilisateur
/// (le prochain téléchargement le réécrit), il est seulement signalé.
fn discard(storage: &Storage, part: &Path) {
    if let Err(e) = storage.delete(part) {
        tracing::warn!(path = %part.display(), error = %crate::errors::one_line(&e), "fichier partiel");
    }
}
