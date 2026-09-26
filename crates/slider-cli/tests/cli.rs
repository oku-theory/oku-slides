//! La CLI `slider`, lancée pour de vrai : chaque test a son propre dossier
//! de données (OKU_SLIDES_PATHS__DATA) et choisit sa langue (OKU_SLIDES_I18N__LOCALE).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use speculoos::prelude::*;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

/// Un dossier de travail neuf pour un test.
fn workdir(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("oku-slides-cli-{name}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("dossier de test : {e}"));
    dir
}

fn slider(dir: &Path, locale: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_slider"));
    cmd.env("OKU_SLIDES_PATHS__DATA", dir.join("data")).env("OKU_SLIDES_I18N__LOCALE", locale).env("NO_COLOR", "1");
    cmd
}

fn run(cmd: &mut Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("slider ne se lance pas : {e}"))
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn check_fixture_reports_the_missing_asset_in_each_language() {
    let dir = workdir("check");
    let fr = run(slider(&dir, "fr").args(["check", FIXTURE]));
    assert_that!(fr.status.success()).is_true();
    assert_that!(stderr(&fr)).contains("asset manquant : sha256:0000");
    let en = run(slider(&dir, "en").args(["check", FIXTURE]));
    assert_that!(stderr(&en)).contains("missing asset: sha256:0000");
}

#[test]
fn check_valid_deck_prints_slide_count() {
    let dir = workdir("valid");
    let deck = dir.join("deck.json");
    std::fs::write(&deck, r#"{"slides":[{"id":"s","kind":"title"}]}"#).unwrap();
    let out = run(slider(&dir, "en").args(["check"]).arg(&deck));
    assert_that!(stdout(&out)).is_equal_to("valid deck: 1 slides\n".to_owned());
}

#[test]
fn check_missing_file_fails_with_localized_context() {
    let dir = workdir("missing");
    let out = run(slider(&dir, "fr").args(["check"]).arg(dir.join("absent.json")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("lecture de");
}

#[test]
fn render_writes_one_svg_per_page_and_the_layout() {
    let dir = workdir("render");
    let out_dir = dir.join("out");
    let out = run(slider(&dir, "en").args(["render", FIXTURE, "--theme", "pro", "--out"]).arg(&out_dir));
    assert_that!(out.status.success()).is_true();
    assert_that!(stdout(&out)).starts_with("8 pages →");
    assert_that!(out_dir.join("page-08.svg").exists()).is_true();
    let layout = std::fs::read_to_string(out_dir.join("layout.json")).unwrap();
    assert_that!(layout).contains("\"slides\"");
}

#[test]
fn save_open_strip_round_trip_keeps_deck_and_assets() {
    let dir = workdir("save");
    let assets = dir.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("logo.bin"), [1u8, 2, 3]).unwrap();
    let pdf = dir.join("deck.oku-slides.pdf");

    let saved = run(slider(&dir, "en").args(["save", FIXTURE]).arg(&pdf).arg("--assets").arg(&assets));
    assert_that!(saved.status.success()).is_true();
    assert_that!(stdout(&saved)).contains("asset ");
    assert_that!(stdout(&saved)).contains(": 8 pages, PDF");

    let extract = dir.join("extract");
    let opened = run(slider(&dir, "en").arg("open").arg(&pdf).arg("--out").arg(&extract));
    assert_that!(stdout(&opened)).contains(": 8 slides, 1 assets, 0 fonts");
    let deck: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(extract.join("deck.json")).unwrap()).unwrap();
    assert_that!(deck["slides"].as_array().map(Vec::len)).is_equal_to(Some(8));
    assert_that!(std::fs::read_dir(extract.join("assets")).unwrap().count()).is_equal_to(1);

    let bare = dir.join("bare.pdf");
    let stripped = run(slider(&dir, "en").arg("strip").arg(&pdf).arg(&bare));
    assert_that!(stdout(&stripped)).contains("copy without sources");
    let reopened = run(slider(&dir, "en").arg("open").arg(&bare));
    assert_that!(reopened.status.success()).is_false();
    assert_that!(stderr(&reopened)).contains("not created by Oku Slides");
}

#[test]
fn save_with_embed_fonts_puts_the_fonts_in_the_sources() {
    let dir = workdir("fonts");
    let fonts = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts");
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(slider(&dir, "en").args(["--fonts", fonts, "save", FIXTURE]).arg(&pdf).arg("--embed-fonts"));
    assert_that!(saved.status.success()).is_true();
    let opened = run(slider(&dir, "en").arg("open").arg(&pdf));
    assert_that!(stdout(&opened)).does_not_contain(" 0 fonts");
}

#[test]
fn schema_prints_json_even_into_a_closed_pipe() {
    let dir = workdir("schema");
    let out = run(slider(&dir, "en").arg("schema"));
    let schema: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_that!(schema.to_string()).contains("set_text");

    // Le lecteur ferme le tube tout de suite : pas de panique, pas d'erreur.
    let mut child = slider(&dir, "en").arg("schema").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    assert_that!(err).does_not_contain("panicked");
    assert_that!(status.success()).is_true();
}

#[test]
fn bench_measures_every_step_and_round_trips() {
    let dir = workdir("bench");
    let out = run(slider(&dir, "fr").args(["bench", FIXTURE, "--iterations", "1"]));
    assert_that!(out.status.success()).is_true();
    assert_that!(stdout(&out)).contains("aller-retour du deck              identique");
}

#[test]
fn bench_zero_iterations_is_refused() {
    let dir = workdir("bench0");
    let out = run(slider(&dir, "en").args(["bench", FIXTURE, "--iterations", "0"]));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("--iterations must be > 0");
}

#[test]
fn help_is_in_the_configured_language() {
    let dir = workdir("help");
    assert_that!(stdout(&run(slider(&dir, "fr").arg("--help")))).contains("Rendu et stockage des decks Oku Slides");
    let save = stdout(&run(slider(&dir, "en").args(["save", "--help"])));
    assert_that!(save).contains("Also embed the --fonts fonts in the sources");
}

#[test]
fn invalid_setting_stops_with_a_clear_error() {
    let dir = workdir("settings");
    let out = run(slider(&dir, "en").env("OKU_SLIDES_SERVE__PORT", "not-a-port").arg("schema"));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("serve.port");
}

// ---- Serveur de développement ------------------------------------------------

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or_else(|e| panic!("port libre : {e}"))
}

/// Une requête HTTP/1.1 minimale ; renvoie le statut et le corps.
fn http(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => break s,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("serveur injoignable : {e}"),
        }
    };
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap_or_else(|e| panic!("requête : {e}"));
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap_or_else(|e| panic!("réponse : {e}"));
    let status = response.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let body = response.split_once("\r\n\r\n").map(|(_, b)| b.to_owned()).unwrap_or_default();
    (status, body)
}

#[test]
fn serve_answers_the_api_the_dictation_and_static_files() {
    let dir = workdir("serve");
    let ui = dir.join("ui");
    std::fs::create_dir_all(&ui).unwrap();
    std::fs::write(ui.join("index.html"), "<h1>Oku Slides</h1>").unwrap();
    std::fs::write(dir.join("secret.txt"), "privé").unwrap();
    let port = free_port();
    let _server = Server(
        slider(&dir, "fr")
            .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
            .args(["serve", FIXTURE, "--port", &port.to_string(), "--ui"])
            .arg(&ui)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );

    let (status, snapshot) = http(port, "POST", "/api", r#"{"cmd":"snapshot"}"#);
    assert_that!(status).is_equal_to(200);
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_that!(snapshot["slides"].as_array().map(Vec::len)).is_equal_to(Some(8));

    let (status, error) = http(port, "POST", "/api", r#"{"cmd":"slide","id":"absente"}"#);
    assert_that!(status).is_equal_to(400);
    assert_that!(error).contains("Slide introuvable : absente");

    let (status, error) = http(port, "POST", "/api", "pas du json");
    assert_that!(status).is_equal_to(400);
    assert_that!(error).contains("requête invalide");

    let (status, stt) = http(port, "POST", "/api/stt", r#"{"cmd":"status"}"#);
    assert_that!(status).is_equal_to(200);
    assert_that!(stt).contains("\"state\":\"idle\"");

    let (status, page) = http(port, "GET", "/", "");
    assert_that!(status).is_equal_to(200);
    assert_that!(page).contains("Oku Slides");

    let (status, _) = http(port, "GET", "/../secret.txt", "");
    assert_that!(status).is_equal_to(404);
}

#[cfg(unix)]
#[test]
fn serve_stops_cleanly_on_sigterm() {
    let dir = workdir("sigterm");
    let port = free_port();
    let mut server = slider(&dir, "en")
        .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
        .args(["serve", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let (status, _) = http(port, "POST", "/api", r#"{"cmd":"snapshot"}"#);
    assert_that!(status).is_equal_to(200);

    let killed = Command::new("kill").args(["-TERM", &server.id().to_string()]).status().unwrap();
    assert_that!(killed.success()).is_true();
    let deadline = Instant::now() + Duration::from_secs(10);
    let exit = loop {
        if let Some(exit) = server.try_wait().unwrap() {
            break exit;
        }
        assert_that!(Instant::now() < deadline).is_true();
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut printed = String::new();
    server.stdout.take().unwrap().read_to_string(&mut printed).unwrap();
    assert_that!(exit.success()).is_true();
    assert_that!(printed).ends_with("server stopped\n");
}
