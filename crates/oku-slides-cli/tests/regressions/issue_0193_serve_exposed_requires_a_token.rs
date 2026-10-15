//! t-193 : hors de la boucle locale, `oku-slides serve` servait tout client du réseau.
//!
//! La garde d'origine n'authentifie pas : un client qui forge `Host` pilotait
//! l'API. Exposé, le serveur exige désormais le jeton affiché au lancement.

use std::io::{BufRead, BufReader};
use std::process::Stdio;

use speculoos::prelude::*;

use crate::common::{FIXTURE, Server, exchange, free_port, oku_slides, request, workdir};

const SNAPSHOT: &str = r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#;

/// La valeur d'un en-tête de réponse.
fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

#[test]
fn serve_exposed_to_the_network_requires_the_launch_token() {
    let dir = workdir("token");
    let port = free_port();
    let mut child = oku_slides(&dir, "en")
        .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
        .env("OKU_SLIDES_SERVE__HOST", "0.0.0.0")
        .args(["serve", FIXTURE, "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("serve : {e}"));
    let stdout = child.stdout.take().unwrap_or_else(|| panic!("sortie de serve"));
    let _server = Server(child);
    // L'URL à ouvrir : la seule ligne qui porte le jeton.
    let url = BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .find_map(|l| l.split_whitespace().find(|w| w.contains("?token=")).map(str::to_owned))
        .unwrap_or_else(|| panic!("serve n'affiche pas l'URL du jeton"));
    assert_that!(url.as_str()).starts_with(format!("http://localhost:{port}/?token="));
    let token = url.split_once("?token=").map(|(_, t)| t.to_owned()).unwrap_or_default();
    let host = format!("127.0.0.1:{port}");
    let json = ("Content-Type", "application/json");

    // Un client qui forge Host, sans jeton ou avec un faux : 401.
    assert_that!(request(port, "POST", "/api", &[("Host", &host), json], SNAPSHOT).0).is_equal_to(401);
    let wrong = [("Host", host.as_str()), json, ("Authorization", "Bearer faux")];
    assert_that!(request(port, "POST", "/api", &wrong, SNAPSHOT).0).is_equal_to(401);
    assert_that!(request(port, "GET", "/?token=faux", &[("Host", &host)], "").0).is_equal_to(401);

    // Le jeton en en-tête.
    let bearer = format!("Bearer {token}");
    let with_bearer = [("Host", host.as_str()), json, ("Authorization", bearer.as_str())];
    assert_that!(request(port, "POST", "/api", &with_bearer, SNAPSHOT).0).is_equal_to(200);

    // La page ouverte avec ?token= pose le cookie et redirige sans le jeton.
    let (status, headers, _) = exchange(port, "GET", &format!("/?token={token}"), &[("Host", &host)], "");
    assert_that!(status).is_equal_to(303);
    assert_that!(header(&headers, "location")).is_equal_to(Some("/"));
    let set_cookie = header(&headers, "set-cookie").unwrap_or_else(|| panic!("cookie : {headers:?}"));
    assert_that!(set_cookie).contains("HttpOnly");
    assert_that!(set_cookie).contains("SameSite=Strict");
    let cookie = set_cookie.split(';').next().unwrap_or_default();
    let with_cookie = [("Host", host.as_str()), json, ("Cookie", cookie)];
    assert_that!(request(port, "POST", "/api", &with_cookie, SNAPSHOT).0).is_equal_to(200);
}
