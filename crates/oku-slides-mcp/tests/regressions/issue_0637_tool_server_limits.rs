//! t-637 : le serveur d'outils de l'agent servait par `axum::serve`, sans
//! délai d'en-têtes ni plafond de connexions : une connexion muette restait
//! ouverte sans fin, rien ne bornait leur nombre, l'arrêt attendait sans fin
//! les flux SSE ouverts, et un client refusé (401) gardait sa connexion.
//! Le serveur reprend les bornes de `[serve]` (ici 1 s, 2 connexions).

use std::num::{NonZeroU64, NonZeroUsize};
use std::time::{Duration, Instant};

use speculoos::prelude::*;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

use crate::common::{Harness, start_with_settings};

/// Le délai d'en-têtes réglé pour ces tests : large, pour que « aussitôt » s'en
/// distingue même sous charge.
const HEADER_TIMEOUT: Duration = Duration::from_secs(1);

/// Ce que « aussitôt » laisse au serveur, bien en deçà du délai d'en-têtes.
const AT_ONCE: Duration = Duration::from_millis(500);

/// Un serveur aux bornes de test, et son port.
async fn bounded() -> (Harness, u16) {
    let h = start_with_settings(|s| {
        s.serve.header_timeout_ms = NonZeroU64::new(1000).unwrap_or_else(|| panic!("délai nul"));
        s.serve.max_connections = NonZeroUsize::new(2).unwrap_or_else(|| panic!("plafond nul"));
    })
    .await;
    let port = h
        .server
        .url()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches("/mcp")
        .parse()
        .unwrap_or_else(|e| panic!("port de {} : {e}", h.server.url()));
    (h, port)
}

async fn connect(port: u16) -> TcpStream {
    TcpStream::connect(("127.0.0.1", port)).await.unwrap_or_else(|e| panic!("connexion : {e}"))
}

async fn send(stream: &mut TcpStream, request: &str) {
    stream.write_all(request.as_bytes()).await.unwrap_or_else(|e| panic!("écriture : {e}"));
}

/// Une requête sans corps ; `extra` : en-têtes en plus, chacun fini par `\r\n`.
fn get(port: u16, path: &str, extra: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}\r\n")
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

/// Les en-têtes d'une réponse (ligne d'état comprise, en minuscules), et
/// son corps s'il a une longueur annoncée ; la connexion reste ouverte.
async fn read_head(stream: &mut TcpStream) -> String {
    let mut got = Vec::new();
    let mut byte = [0u8; 1];
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        while !got.ends_with(b"\r\n\r\n") {
            match stream.read(&mut byte).await {
                Ok(0) | Err(_) => break,
                Ok(_) => got.push(byte[0]),
            }
        }
    });
    read.await.unwrap_or_else(|_| panic!("en-têtes de réponse attendus"));
    let head = String::from_utf8_lossy(&got).to_ascii_lowercase();
    let length = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|n| n.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).await.unwrap_or_else(|e| panic!("corps de réponse : {e}"));
    head
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_server_mute_connection_closed_after_header_timeout() {
    let (_h, port) = bounded().await;
    let mut mute = connect(port).await;
    let since = Instant::now();
    let read = read_until_closed(&mut mute, Duration::from_secs(5)).await;
    assert_that!(read.is_some_and(|r| !r.contains("HTTP/1.1 2"))).named("fermée sans réponse").is_true();
    assert_that!(since.elapsed() >= HEADER_TIMEOUT - Duration::from_millis(50)).named("pas avant le délai").is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_server_connection_beyond_cap_closed_at_once_until_a_place_frees_up() {
    let (_h, port) = bounded().await;
    // Deux connexions muettes tiennent les deux places jusqu'au délai d'en-têtes.
    let first = connect(port).await;
    let second = connect(port).await;
    let mut third = connect(port).await;
    send(&mut third, &get(port, "/mcp", "")).await;
    let since = Instant::now();
    let read = read_until_closed(&mut third, Duration::from_secs(2)).await;
    assert_that!(read).is_equal_to(Some(String::new()));
    assert_that!(since.elapsed() < AT_ONCE).named("aussitôt").is_true();

    // Les places se libèrent : une nouvelle connexion reçoit sa réponse.
    drop((first, second));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut again = connect(port).await;
        send(&mut again, &get(port, "/mcp", "")).await;
        let read = read_until_closed(&mut again, Duration::from_secs(2)).await.unwrap_or_default();
        if read.starts_with("HTTP/1.1 401") {
            break;
        }
        assert_that!(Instant::now() < deadline).named("connexion servie à temps").is_true();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_server_open_event_stream_closed_within_grace_after_cancel() {
    let (h, port) = bounded().await;
    let auth = format!("Authorization: {}\r\n", h.server.authorization());
    // Une session MCP : `initialize`, dont on ne garde que l'identifiant.
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}"#;
    let mut init = connect(port).await;
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Accept: application/json, text/event-stream\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    send(&mut init, &request).await;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        let n = init.read(&mut byte).await.unwrap_or_else(|e| panic!("réponse à initialize : {e}"));
        assert_that!(n).named("réponse à initialize complète").is_equal_to(1);
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
    let session = head
        .lines()
        .find_map(|l| l.strip_prefix("mcp-session-id:"))
        .map(str::trim)
        .unwrap_or_else(|| panic!("pas de session dans {head}"))
        .to_owned();
    drop(init);

    // Le flux SSE de la session, ouvert jusqu'à l'arrêt.
    let mut stream = connect(port).await;
    let extra =
        format!("{auth}Accept: text/event-stream\r\nMcp-Session-Id: {session}\r\nMcp-Protocol-Version: 2025-11-25\r\n");
    send(&mut stream, &get(port, "/mcp", &extra)).await;
    let mut status = [0u8; 12];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut status))
        .await
        .unwrap_or_else(|_| panic!("réponse du flux attendue"))
        .unwrap_or_else(|e| panic!("réponse du flux : {e}"));
    assert_that!(String::from_utf8_lossy(&status).into_owned()).is_equal_to("HTTP/1.1 200".to_owned());

    h.cancel.cancel();
    let since = Instant::now();
    let closed = read_until_closed(&mut stream, Duration::from_secs(5)).await;
    assert_that!(closed.is_some()).named("flux fermé à l'arrêt").is_true();
    assert_that!(since.elapsed() < Duration::from_secs(2)).named("dans la grâce (1 s), marge comprise").is_true();
}

/// hyper 1.11 relance le délai d'en-têtes dès qu'une connexion keep-alive
/// attend sa requête suivante : restée inactive au-delà du délai, elle est
/// fermée sans réponse. Avec `[serve] header_timeout_ms` (10 s par défaut),
/// le client de l'agent (undici, keep-alive de 4 s) ferme la sienne avant.
#[tokio::test(flavor = "multi_thread")]
async fn tool_server_idle_keep_alive_connection_closed_after_header_timeout() {
    let (h, port) = bounded().await;
    let auth = format!("Authorization: {}\r\n", h.server.authorization());
    let mut kept = connect(port).await;
    send(&mut kept, &get(port, "/sonde", &auth)).await;
    let head = read_head(&mut kept).await;
    assert_that!(head.contains("connection: close")).named("connexion gardée").is_false();

    tokio::time::sleep(HEADER_TIMEOUT * 2).await;
    let read = read_until_closed(&mut kept, Duration::from_millis(500)).await;
    assert_that!(read).named("fermée, inactive au-delà du délai").is_equal_to(Some(String::new()));
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_server_unauthorized_request_answered_then_connection_closed() {
    let (_h, port) = bounded().await;
    let mut refused = connect(port).await;
    send(&mut refused, &get(port, "/mcp", "")).await;
    let since = Instant::now();
    let read = read_until_closed(&mut refused, Duration::from_secs(2)).await.map(|r| r.to_ascii_lowercase());
    assert_that!(read.as_deref().is_some_and(|r| r.starts_with("http/1.1 401"))).named("401").is_true();
    assert_that!(read.as_deref().is_some_and(|r| r.contains("connection: close"))).named("Connection: close").is_true();
    assert_that!(since.elapsed() < AT_ONCE).named("fermée aussitôt").is_true();
}
