//! A scripted raw-TCP hub for the download tests: each connection answered by a closure.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

/// Request heads the hub received, in order.
pub(super) type Seen = Arc<Mutex<Vec<String>>>;

/// A hub whose `answer(n, head, stream)` handles the n-th connection; returns its URL and log.
pub(super) fn scripted(answer: impl Fn(usize, &str, TcpStream) + Send + Sync + 'static) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let answer = Arc::new(answer);
    std::thread::spawn(move || {
        for (n, stream) in listener.incoming().enumerate() {
            let Ok(stream) = stream else { return };
            let (log, answer) = (log.clone(), answer.clone());
            std::thread::spawn(move || {
                let head = read_head(&stream);
                log.lock().unwrap().push(head.clone());
                answer(n, &head, stream);
            });
        }
    });
    (url, seen)
}

/// The request line and headers, lowercased.
fn read_head(stream: &TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
            return head.to_lowercase();
        }
        head.push_str(&line);
    }
}

/// Writes a status line, headers and `body` bytes; keeps the socket open.
pub(super) fn send(stream: &mut TcpStream, status: &str, headers: &[String], body: &[u8]) {
    let mut out = format!("HTTP/1.1 {status}\r\nconnection: close\r\n");
    for h in headers {
        out.push_str(h);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    let _ = stream.write_all(out.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// The `bytes=N-` start of a Range header, if the request carries one.
pub(super) fn range_start(head: &str) -> Option<usize> {
    head.lines().find_map(|l| l.strip_prefix("range: bytes=")).and_then(|r| r.trim().trim_end_matches('-').parse().ok())
}

/// Serves `body` honouring `Range`, as a hub does.
pub(super) fn serve_ranged(body: &[u8], head: &str, mut stream: TcpStream) {
    match range_start(head) {
        Some(start) if start < body.len() => {
            let rest = &body[start..];
            let headers = [
                format!("content-length: {}", rest.len()),
                format!("content-range: bytes {start}-{}/{}", body.len() - 1, body.len()),
            ];
            send(&mut stream, "206 Partial Content", &headers, rest);
        }
        Some(_) => send(&mut stream, "416 Range Not Satisfiable", &["content-length: 0".into()], b""),
        None => send(&mut stream, "200 OK", &[format!("content-length: {}", body.len())], body),
    }
}

/// Serves `body` from the asked offset like [`serve_ranged`], then closes after `max` bytes.
pub(super) fn serve_cut(body: &[u8], max: usize, head: &str, mut stream: TcpStream) {
    let start = range_start(head).unwrap_or(0);
    let rest = &body[start..];
    let mut headers = vec![format!("content-length: {}", rest.len())];
    let status = if start > 0 {
        headers.push(format!("content-range: bytes {start}-{}/{}", body.len() - 1, body.len()));
        "206 Partial Content"
    } else {
        "200 OK"
    };
    send(&mut stream, status, &headers, &rest[..max.min(rest.len())]);
}

/// The `Range` start of each request received, in order.
pub(super) fn ranges(seen: &Seen) -> Vec<Option<usize>> {
    seen.lock().unwrap().iter().map(|h| range_start(h)).collect()
}
