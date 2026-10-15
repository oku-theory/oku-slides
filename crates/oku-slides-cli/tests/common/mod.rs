//! Outillage des tests de la CLI : dossier de travail, lancement, serveur, requêtes HTTP.
#![allow(
    dead_code,
    reason = "cli.rs et regressions/ n'utilisent pas les mêmes aides ; `expect` échouerait dans l'un des deux"
)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Output};
use std::time::{Duration, Instant};

pub const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

/// Un dossier de travail neuf, supprimé à la fin du test.
///
/// Créé atomiquement sous un nom aléatoire (t-273) : jamais celui d'un
/// lancement précédent, même quand macOS recycle le PID. À déclarer avant le
/// [`Server`] qui l'utilise : il est supprimé une fois le serveur arrêté.
#[derive(Debug)]
pub struct Workdir(tempfile::TempDir);

impl std::ops::Deref for Workdir {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.0.path()
    }
}

impl AsRef<Path> for Workdir {
    fn as_ref(&self) -> &Path {
        self.0.path()
    }
}

pub fn workdir(name: &str) -> Workdir {
    let dir = tempfile::Builder::new()
        .prefix(&format!("oku-slides-cli-{name}-"))
        .tempdir()
        .unwrap_or_else(|e| panic!("dossier de test : {e}"));
    Workdir(dir)
}

/// La CLI de test, avec son dossier de données et sa langue.
pub fn oku_slides(dir: &Path, locale: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_oku-slides"));
    cmd.env("OKU_SLIDES_PATHS__DATA", dir.join("data")).env("OKU_SLIDES_I18N__LOCALE", locale).env("NO_COLOR", "1");
    cmd
}

/// Le compte rendu d'une commande terminée : statut, stdout et stderr, pour qu'un échec de test
/// dise ce que la commande a répondu (t-635).
pub fn describe(what: &str, out: &Output) -> String {
    format!(
        "{what} : {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Les comptes rendus de `context` (commandes précédentes) puis celui de `what`, formatés
/// seulement quand un test échoue.
fn report(what: &str, out: &Output, context: &[(&str, &Output)]) -> String {
    let before: String = context.iter().map(|(name, o)| format!("{}\n\n", describe(name, o))).collect();
    format!("{before}{}", describe(what, out))
}

/// Échoue si la commande n'a pas réussi, en décrivant aussi les commandes de `context`.
#[track_caller]
pub fn assert_success(what: &str, out: &Output, context: &[(&str, &Output)]) {
    assert!(out.status.success(), "{}", report(what, out, context));
}

/// Échoue si la sortie standard CONTIENT `needle`, en décrivant aussi les commandes de `context`.
#[track_caller]
pub fn assert_stdout_lacks(what: &str, out: &Output, needle: &str, context: &[(&str, &Output)]) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains(needle), "stdout contient {needle:?}\n{}", report(what, out, context));
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
    let (status, _, body) = exchange(port, method, path, headers, body);
    (status, body)
}

/// Comme [`request`], avec aussi les en-têtes de la réponse (noms en minuscules).
pub fn exchange(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, Vec<(String, String)>, String) {
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
    let (head, body) = response.split_once("\r\n\r\n").unwrap_or((response.as_str(), ""));
    let headers = head
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    (status, headers, body.to_owned())
}
