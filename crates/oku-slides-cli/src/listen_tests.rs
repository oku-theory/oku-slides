use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use speculoos::prelude::*;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

use super::{Limits, serve_connections};

/// Un serveur qui répond « ok » à tout, sur un port libre ; `stop` l'arrête.
async fn start(limits: Limits) -> (u16, Arc<Notify>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap_or_else(|e| panic!("écoute : {e}"));
    let port = listener.local_addr().unwrap_or_else(|e| panic!("adresse : {e}")).port();
    let app = Router::new().fallback(|| async { "ok" });
    let stop = Arc::new(Notify::new());
    let shutdown = {
        let stop = Arc::clone(&stop);
        async move { stop.notified().await }
    };
    let served = tokio::spawn(serve_connections(listener, app, limits, shutdown));
    (port, stop, served)
}

/// Ce que lit le client avant que la connexion se ferme (ou `None` si elle
/// reste ouverte plus de `within`).
async fn read_until_closed(stream: &mut TcpStream, within: Duration) -> Option<String> {
    let mut got = Vec::new();
    let mut buf = [0u8; 1024];
    let read = tokio::time::timeout(within, async {
        loop {
            match stream.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => got.extend_from_slice(&buf[..n]),
            }
        }
    });
    read.await.ok().map(|()| String::from_utf8_lossy(&got).into_owned())
}

/// Une requête complète sur `stream`, connexion gardée ; vrai si la réponse arrive.
async fn answered(stream: &mut TcpStream, port: u16) -> bool {
    let request = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    if stream.write_all(request.as_bytes()).await.is_err() {
        return false;
    }
    let mut buf = [0u8; 12];
    matches!(
        tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buf)).await,
        Ok(Ok(_)) if &buf == b"HTTP/1.1 200"
    )
}

/// Une connexion servie (en réessayant : la place d'une connexion fermée se
/// libère un instant après sa fermeture).
async fn connect_served(port: u16) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap_or_else(|e| panic!("connexion : {e}"));
        if answered(&mut stream, port).await {
            return stream;
        }
        assert_that!(Instant::now() < deadline).named("connexion servie à temps").is_true();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn a_connection_that_never_sends_its_headers_is_closed_after_the_delay() {
    let limits = Limits { header_timeout: Duration::from_millis(300), max_connections: 8 };
    let (port, stop, served) = start(limits).await;
    let mut slow = TcpStream::connect(("127.0.0.1", port)).await.unwrap_or_else(|e| panic!("connexion : {e}"));
    // Des en-têtes commencés, jamais finis.
    slow.write_all(b"GET / HTTP/1.1\r\nHost: 127.0").await.unwrap_or_else(|e| panic!("écriture : {e}"));
    let since = Instant::now();
    let read = read_until_closed(&mut slow, Duration::from_secs(5)).await;
    assert_that!(read.as_deref().is_some_and(|r| !r.contains("200"))).named("fermée sans réponse").is_true();
    assert_that!(since.elapsed() >= Duration::from_millis(250)).named("pas avant le délai").is_true();

    // Une connexion muette aussi.
    let mut mute = TcpStream::connect(("127.0.0.1", port)).await.unwrap_or_else(|e| panic!("connexion : {e}"));
    assert_that!(read_until_closed(&mut mute, Duration::from_secs(5)).await.is_some()).named("muette fermée").is_true();

    // Les autres sont servies.
    let _ok = connect_served(port).await;
    stop.notify_one();
    assert_that!(tokio::time::timeout(Duration::from_secs(5), served).await.is_ok()).named("arrêt").is_true();
}

#[tokio::test]
async fn beyond_the_cap_a_new_connection_is_closed_at_once_until_a_place_frees_up() {
    let limits = Limits { header_timeout: Duration::from_secs(30), max_connections: 2 };
    let (port, stop, served) = start(limits).await;
    let first = connect_served(port).await;
    let _second = connect_served(port).await;

    // Deux connexions gardées ouvertes : la troisième est fermée sans réponse.
    let mut third = TcpStream::connect(("127.0.0.1", port)).await.unwrap_or_else(|e| panic!("connexion : {e}"));
    let request = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    let _ = third.write_all(request.as_bytes()).await;
    let read = read_until_closed(&mut third, Duration::from_secs(2)).await;
    assert_that!(read).is_equal_to(Some(String::new()));

    // Une place se libère : une nouvelle connexion est servie.
    drop(first);
    let _again = connect_served(port).await;
    stop.notify_one();
    assert_that!(tokio::time::timeout(Duration::from_secs(5), served).await.is_ok()).named("arrêt").is_true();
}

#[tokio::test]
async fn shutdown_closes_idle_connections_and_returns() {
    let limits = Limits { header_timeout: Duration::from_secs(30), max_connections: 8 };
    let (port, stop, served) = start(limits).await;
    let mut idle = connect_served(port).await;
    stop.notify_one();
    assert_that!(tokio::time::timeout(Duration::from_secs(5), served).await.is_ok()).named("arrêt").is_true();
    assert_that!(read_until_closed(&mut idle, Duration::from_secs(2)).await.is_some()).named("fermée").is_true();
}

#[test]
fn limits_never_ask_more_places_than_a_semaphore_holds() {
    let limits = Limits::new(Duration::from_secs(1), usize::MAX);
    assert_that!(limits.max_connections).is_equal_to(tokio::sync::Semaphore::MAX_PERMITS);
}
