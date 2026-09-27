//! t-181 : `oku-slides serve` exécutait les requêtes d'une page tierce.
//!
//! Un POST `text/plain` part d'un autre site sans pré-vérification CORS, et un
//! nom d'hôte qui résout vers 127.0.0.1 (DNS rebinding) lit les réponses : une
//! page ouverte dans le navigateur pilotait l'agent, ou `open`/`save`.
//!
//! Les refus visent `GET /` plutôt que le flux `/api/events` : si la garde
//! régressait, le flux resterait ouvert et le test attendrait au lieu d'échouer.

use std::process::Stdio;

use speculoos::prelude::*;

use crate::common::{FIXTURE, Server, free_port, http, oku_slides, request, workdir};

/// Nombre de slides du document ouvert, lu par l'interface locale.
fn slide_count(port: u16) -> Option<usize> {
    let (_, snapshot) = http(port, "POST", "/api", r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#);
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap_or_else(|e| panic!("snapshot : {e}"));
    snapshot["slides"].as_array().map(Vec::len)
}

#[test]
fn serve_refuses_requests_from_another_origin() {
    let dir = workdir("origin");
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
    let host = format!("127.0.0.1:{port}");
    let json = "application/json";
    // L'interface locale est servie : directement, et à travers le proxy de Vite.
    assert_that!(slide_count(port)).is_equal_to(Some(8));
    let vite = [("Host", "localhost:5173"), ("Content-Type", json), ("Origin", "http://localhost:5173")];
    assert_that!(request(port, "POST", "/api", &vite, r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#).0).is_equal_to(200);

    // CSRF : un fetch no-cors d'une autre page, qui ne doit rien exécuter.
    let plain = [("Host", host.as_str()), ("Content-Type", "text/plain")];
    let (status, body) = request(port, "POST", "/api", &plain, r#"{"tab":"t-1","req":{"cmd":"new"}}"#);
    assert_that!(status).is_equal_to(403);
    let error: serde_json::Value = serde_json::from_str(&body).unwrap_or_else(|e| panic!("réponse : {e}"));
    assert_that!(error["error"].as_str().is_some_and(|e| !e.is_empty())).is_true();
    assert_that!(slide_count(port)).is_equal_to(Some(8));
    let prompt = r#"{"cmd":"prompt","tab":"t-1","text":"supprime tout","ui":{}}"#;
    assert_that!(request(port, "POST", "/api/agent", &plain, prompt).0).is_equal_to(403);

    // Une autre origine, même sur un autre port de cette machine.
    for origin in ["https://evil.example", "http://localhost:3000"] {
        let foreign = [("Host", host.as_str()), ("Content-Type", json), ("Origin", origin)];
        assert_that!(request(port, "POST", "/api", &foreign, r#"{"tab":"t-1","req":{"cmd":"new"}}"#).0)
            .named(origin)
            .is_equal_to(403);
    }
    assert_that!(slide_count(port)).is_equal_to(Some(8));

    // Une balise d'une autre page : pas d'Origin, mais Sec-Fetch-Site.
    let tag = [("Host", host.as_str()), ("Sec-Fetch-Site", "cross-site")];
    assert_that!(request(port, "GET", "/", &tag, "").0).is_equal_to(403);

    // La pré-vérification CORS n'obtient jamais d'accord.
    let preflight = [("Host", host.as_str()), ("Origin", "https://evil.example")];
    assert_that!(request(port, "OPTIONS", "/api", &preflight, "").0).is_equal_to(403);

    // DNS rebinding : un nom étranger pour cette machine.
    let rebound = format!("evil.example:{port}");
    assert_that!(request(port, "GET", "/", &[("Host", &rebound)], "").0).is_equal_to(403);
    let rebound_json = [("Host", rebound.as_str()), ("Content-Type", json)];
    assert_that!(request(port, "POST", "/api", &rebound_json, r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#).0)
        .is_equal_to(403);
}
