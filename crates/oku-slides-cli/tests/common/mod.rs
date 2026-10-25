//! Outillage des tests de la CLI : dossier de travail, lancement, serveur, requêtes HTTP, fixtures d'import zippées.
#![allow(
    dead_code,
    reason = "cli.rs et regressions/ n'utilisent pas les mêmes aides ; `expect` échouerait dans l'un des deux"
)]

use std::fmt::Write as _;
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::time::{Duration, Instant};

use zip::write::SimpleFileOptions;

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
    let before = context.iter().fold(String::new(), |mut before, (name, o)| {
        let _ = write!(before, "{}\n\n", describe(name, o));
        before
    });
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
        .map_or_else(|e| panic!("port libre : {e}"), |a| a.port())
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
    let headers = headers.iter().fold(String::new(), |mut lines, (k, v)| {
        let _ = write!(lines, "{k}: {v}\r\n");
        lines
    });
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

/// Le dossier des fixtures d'import (`fixtures/import`).
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import")
}

/// Les fichiers de `dir`, récursivement, sans `.DS_Store`, ajoutés à `out`.
pub fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let p = e.unwrap_or_else(|e| panic!("{e}")).path();
        if p.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if p.is_dir() { files(&p, out) } else { out.push(p) }
    }
}

/// Le `.pptx` de la fixture décompressée `name`, zippé dans `dir` ; son chemin.
pub fn pptx(dir: &Path, name: &str) -> PathBuf {
    pptx_with(dir, name, &[])
}

/// Comme [`pptx`], les parts de `replaced` (chemin dans le zip, contenu) remplaçant celles de la fixture.
pub fn pptx_with(dir: &Path, name: &str, replaced: &[(&str, &[u8])]) -> PathBuf {
    let root = fixtures().join(name);
    let mut paths = Vec::new();
    files(&root, &mut paths);
    paths.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for p in paths {
        let rel = p.strip_prefix(&root).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        let data = match replaced.iter().find(|(part, _)| *part == rel) {
            Some((_, bytes)) => bytes.to_vec(),
            None => std::fs::read(&p).unwrap_or_else(|e| panic!("{e}")),
        };
        w.start_file(rel, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
        w.write_all(&data).unwrap_or_else(|e| panic!("zip : {e}"));
    }
    let bytes = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let path = dir.join(format!("{name}.pptx"));
    std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{e}"));
    path
}

/// Les épinglages des packs intégrés `base` (widgets) et `essentiel` (layouts), et leurs fichiers.
pub fn builtin_pins() -> (oku_slides_core::PackPin, oku_slides_core::PackPin, Vec<oku_slides_core::PackFiles>) {
    let storage = oku_slides_session::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = oku_slides_session::PackLibrary::new(
        None,
        None,
        oku_slides_session::ThemeLibrary::new(None, storage.clone()),
        storage,
    );
    let pin = |id| {
        let v = library.current_version(id).unwrap_or_else(|| panic!("pack intégré {id}"));
        (oku_slides_core::PackPin { id, version: v.version, requires: v.manifest.requires }, v.files)
    };
    let (base, base_files) = pin(oku_slides_session::BASE_PACK);
    let (essentiel, essentiel_files) = pin(oku_slides_session::ESSENTIEL_PACK);
    (base, essentiel, vec![base_files, essentiel_files])
}

/// Un deck JSON (version 9) de `slides` qui épingle les packs de widgets `widgets`, plus `base`
/// et le pack de layouts `essentiel` : un deck qui épingle un pack n'en reçoit pas d'autre.
pub fn deck_json(widgets: serde_json::Value, slides: serde_json::Value) -> serde_json::Value {
    let (base, essentiel, _) = builtin_pins();
    let mut widgets = widgets;
    if let Some(map) = widgets.as_object_mut() {
        map.insert("base".to_owned(), serde_json::json!(base));
    }
    let mut deck = serde_json::json!({ "version": 9, "widgets": widgets, "layouts": { "essentiel": essentiel } });
    deck["slides"] = slides;
    deck
}
