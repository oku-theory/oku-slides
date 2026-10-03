//! t-419 : `oku-slides serve` laissait des requêtes sans réponse.
//!
//! Le navigateur ouvre plusieurs connexions d'un coup (aperçus des slides
//! demandés en parallèle) et les garde ouvertes (keep-alive). La réserve de
//! fils de `tiny_http` met alors certaines connexions en file sans fil pour
//! les lire : leur requête attendait qu'une autre connexion se ferme.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Stdio;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use speculoos::prelude::*;

use crate::common::{FIXTURE, Server, free_port, http, oku_slides, workdir};

/// Connexions ouvertes d'un coup, comme un navigateur qui charge les aperçus.
const BURST: usize = 8;
/// Vagues successives, les connexions des précédentes restant ouvertes.
const ROUNDS: usize = 4;

#[test]
fn serve_answers_every_request_of_a_burst_of_keep_alive_connections() {
    let dir = workdir("burst");
    let port = free_port();
    let _server = Server(
        oku_slides(&dir, "en")
            .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
            .args(["serve", FIXTURE, "--port", &port.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("serve : {e}")),
    );
    // Le serveur est prêt (cette connexion-là se ferme après sa réponse).
    assert_that!(http(port, "POST", "/api", r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#).0).is_equal_to(200);

    let body = r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#;
    let request = format!(
        "POST /api HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: keep-alive\r\n\r\n{body}",
        body.len()
    );
    // Plusieurs vagues : chaque connexion d'une vague s'ouvre en même temps que
    // les autres (barrière), et toutes restent ouvertes jusqu'à la fin, comme
    // les connexions keep-alive d'un navigateur.
    let mut kept = Vec::new();
    for wave in 0..ROUNDS {
        let barrier = Arc::new(Barrier::new(BURST));
        let handles: Vec<_> = (0..BURST)
            .map(|_| {
                let (barrier, request) = (barrier.clone(), request.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|e| panic!("connexion : {e}"));
                    s.write_all(request.as_bytes()).unwrap_or_else(|e| panic!("requête : {e}"));
                    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap_or_else(|e| panic!("délai : {e}"));
                    let mut head = [0u8; 12];
                    let ok = s.read_exact(&mut head).is_ok() && head.starts_with(b"HTTP/1.1 200");
                    (ok, s)
                })
            })
            .collect();
        let results: Vec<(bool, TcpStream)> =
            handles.into_iter().map(|h| h.join().unwrap_or_else(|_| panic!("fil de la vague {wave}"))).collect();
        let answered = results.iter().filter(|(ok, _)| *ok).count();
        assert_that!(answered).named(&format!("vague {wave}")).is_equal_to(BURST);
        kept.extend(results.into_iter().map(|(_, s)| s));
    }
}
