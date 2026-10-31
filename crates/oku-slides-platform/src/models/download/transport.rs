//! One HTTP request on its own thread, waited on in cancellable slices.

use std::io::Read;
use std::sync::mpsc::{Receiver, RecvTimeoutError, sync_channel};
use std::time::{Duration, Instant};

/// Wait slice: cancellation is checked at least this often.
const SLICE: Duration = Duration::from_millis(200);

/// Largest chunk read from the body and sent at once.
const CHUNK: usize = 1 << 20;

/// The response head: status, announced length and `Content-Range`.
pub(super) struct Head {
    pub(super) status: u16,
    pub(super) length: Option<u64>,
    pub(super) content_range: Option<String>,
}

/// What a request's thread sends: head, chunks, end or failure.
pub(super) enum Fetched {
    Head(Head),
    Chunk(Vec<u8>),
    End,
    Http(Box<ureq::Error>),
    Read(std::io::Error),
}

/// Why waiting on a request's thread stops without a message.
pub(super) enum Stop {
    Cancelled,
    /// Nothing received for `read_timeout_secs`.
    Stalled,
}

/// Starts the `url` request on a separate thread, from byte `from` when set.
///
/// Its results arrive on the returned channel. A blocked read holds the thread until the
/// connection ends; nobody waits for it.
pub(super) fn fetch(agent: &ureq::Agent, url: String, from: Option<u64>) -> Receiver<Fetched> {
    let (tx, rx) = sync_channel(4);
    let agent = agent.clone();
    std::thread::spawn(move || {
        // Byte offsets must address the file itself, not a compressed form of it.
        let mut request = agent.get(&url).header("accept-encoding", "identity");
        if let Some(from) = from {
            request = request.header("range", format!("bytes={from}-"));
        }
        let mut resp = match request.call() {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Fetched::Http(Box::new(e)));
                return;
            }
        };
        let header = |name: &str| resp.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_owned);
        let head = Head {
            status: resp.status().as_u16(),
            length: header("content-length").and_then(|v| v.parse().ok()),
            content_range: header("content-range"),
        };
        if tx.send(Fetched::Head(head)).is_err() {
            return;
        }
        let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
        loop {
            // Read straight into the chunk sent: no copy.
            let mut chunk = vec![0u8; CHUNK];
            let msg = match reader.read(&mut chunk) {
                Ok(0) => Fetched::End,
                Ok(n) => {
                    chunk.truncate(n);
                    Fetched::Chunk(chunk)
                }
                Err(e) => Fetched::Read(e),
            };
            let last = !matches!(msg, Fetched::Chunk(_));
            // The abandoned download no longer reads: stop.
            if tx.send(msg).is_err() || last {
                return;
            }
        }
    });
    rx
}

/// The next message of `rx`, checking cancellation (`progress(at, total)`) every slice.
pub(super) fn next(
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
            // The thread always sends a last message: a closed channel is a lost connection.
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

/// Waits `wait` in slices; false if `progress` asks to cancel meanwhile.
pub(super) fn backoff(wait: Duration, at: u64, total: u64, progress: &mut impl FnMut(u64, u64) -> bool) -> bool {
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

/// The download agent: environment proxy, system certificates, bounded waits.
pub(in crate::models) fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .proxy(ureq::Proxy::try_from_env())
        .tls_config(crate::http::system_tls())
        // Bounds the request's wait; the body is bounded read by read (`next`).
        .timeout_connect(Some(timeout))
        .timeout_recv_response(Some(timeout))
        .build()
        .into()
}
