//! Outillage des tests de la CLI : dossier de travail, lancement, serveur, requêtes HTTP.
#![allow(
    dead_code,
    reason = "cli.rs et regressions/ n'utilisent pas les mêmes aides ; `expect` échouerait dans l'un des deux"
)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

/// Un dossier de travail neuf pour un test.
pub fn workdir(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("oku-slides-cli-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("dossier de test : {e}"));
    dir
}

/// La CLI de test, avec son dossier de données et sa langue.
pub fn oku_slides(dir: &Path, locale: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_oku-slides"));
    cmd.env("OKU_SLIDES_PATHS__DATA", dir.join("data")).env("OKU_SLIDES_I18N__LOCALE", locale).env("NO_COLOR", "1");
    cmd
}

/// Le processus `oku-slides serve`, tué à la fin du test.
pub struct Server(pub Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Un port TCP libre sur la boucle locale.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or_else(|e| panic!("port libre : {e}"))
}

/// Une requête du client JSON de l'interface (`Host` local, `Content-Type: application/json`,
/// même pour un GET) ; renvoie le statut et le corps.
pub fn http(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    let host = format!("127.0.0.1:{port}");
    request(port, method, path, &[("Host", &host), ("Content-Type", "application/json")], body)
}

/// Une requête HTTP/1.1 minimale avec ses en-têtes ; renvoie le statut et le corps.
pub fn request(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> (u16, String) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => break s,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("serveur injoignable : {e}"),
        }
    };
    let headers: String = headers.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap_or_else(|e| panic!("requête : {e}"));
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap_or_else(|e| panic!("réponse : {e}"));
    let status = response.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let body = response.split_once("\r\n\r\n").map(|(_, b)| b.to_owned()).unwrap_or_default();
    (status, body)
}
