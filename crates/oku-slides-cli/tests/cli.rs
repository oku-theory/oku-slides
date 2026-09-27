//! La CLI `oku-slides`, lancée pour de vrai : chaque test a son propre dossier
//! de données (OKU_SLIDES_PATHS__DATA) et choisit sa langue (OKU_SLIDES_I18N__LOCALE).

mod common;

use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use common::{FIXTURE, Server, free_port, http, oku_slides, request, workdir};
use speculoos::prelude::*;

fn run(cmd: &mut Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"))
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
    let fr = run(oku_slides(&dir, "fr").args(["check", FIXTURE]));
    assert_that!(fr.status.success()).is_true();
    assert_that!(stderr(&fr)).contains("asset manquant : sha256:0000");
    let en = run(oku_slides(&dir, "en").args(["check", FIXTURE]));
    assert_that!(stderr(&en)).contains("missing asset: sha256:0000");
}

#[test]
fn check_valid_deck_prints_slide_count() {
    let dir = workdir("valid");
    let deck = dir.join("deck.json");
    std::fs::write(&deck, r#"{"slides":[{"id":"s","kind":"title"}]}"#).unwrap();
    let out = run(oku_slides(&dir, "en").args(["check"]).arg(&deck));
    assert_that!(stdout(&out)).is_equal_to("valid deck: 1 slides\n".to_owned());
}

#[test]
fn check_missing_file_fails_with_localized_context() {
    let dir = workdir("missing");
    let out = run(oku_slides(&dir, "fr").args(["check"]).arg(dir.join("absent.json")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("lecture de");
}

#[test]
fn render_writes_one_svg_per_page_and_the_layout() {
    let dir = workdir("render");
    let out_dir = dir.join("out");
    let out = run(oku_slides(&dir, "en").args(["render", FIXTURE, "--theme", "pro", "--out"]).arg(&out_dir));
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

    let saved = run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&pdf).arg("--assets").arg(&assets));
    assert_that!(saved.status.success()).is_true();
    assert_that!(stdout(&saved)).contains("asset ");
    assert_that!(stdout(&saved)).contains(": 8 pages, PDF");

    let extract = dir.join("extract");
    let opened = run(oku_slides(&dir, "en").arg("open").arg(&pdf).arg("--out").arg(&extract));
    assert_that!(stdout(&opened)).contains(": 8 slides, 1 assets, 0 fonts");
    let deck: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(extract.join("deck.json")).unwrap()).unwrap();
    assert_that!(deck["slides"].as_array().map(Vec::len)).is_equal_to(Some(8));
    assert_that!(std::fs::read_dir(extract.join("assets")).unwrap().count()).is_equal_to(1);

    let bare = dir.join("bare.pdf");
    let stripped = run(oku_slides(&dir, "en").arg("strip").arg(&pdf).arg(&bare));
    assert_that!(stdout(&stripped)).contains("copy without sources");
    let reopened = run(oku_slides(&dir, "en").arg("open").arg(&bare));
    assert_that!(reopened.status.success()).is_false();
    assert_that!(stderr(&reopened)).contains("not created by Oku Slides");
}

#[test]
fn save_with_embed_fonts_puts_the_fonts_in_the_sources() {
    let dir = workdir("fonts");
    let fonts = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts");
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").args(["--fonts", fonts, "save", FIXTURE]).arg(&pdf).arg("--embed-fonts"));
    assert_that!(saved.status.success()).is_true();
    let opened = run(oku_slides(&dir, "en").arg("open").arg(&pdf));
    assert_that!(stdout(&opened)).does_not_contain(" 0 fonts");
}

#[test]
fn schema_prints_json_even_into_a_closed_pipe() {
    let dir = workdir("schema");
    let out = run(oku_slides(&dir, "en").arg("schema"));
    let schema: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_that!(schema.to_string()).contains("set_text");

    // Le lecteur ferme le tube tout de suite : pas de panique, pas d'erreur.
    let mut child = oku_slides(&dir, "en").arg("schema").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
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
    let out = run(oku_slides(&dir, "fr").args(["bench", FIXTURE, "--iterations", "1"]));
    assert_that!(out.status.success()).is_true();
    assert_that!(stdout(&out)).contains("aller-retour du deck              identique");
}

#[test]
fn bench_zero_iterations_is_refused() {
    let dir = workdir("bench0");
    let out = run(oku_slides(&dir, "en").args(["bench", FIXTURE, "--iterations", "0"]));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("--iterations must be > 0");
}

#[test]
fn help_is_in_the_configured_language() {
    let dir = workdir("help");
    assert_that!(stdout(&run(oku_slides(&dir, "fr").arg("--help")))).contains("Rendu et stockage des decks Oku Slides");
    let save = stdout(&run(oku_slides(&dir, "en").args(["save", "--help"])));
    assert_that!(save).contains("Also embed the --fonts fonts in the sources");
}

#[test]
fn invalid_setting_stops_with_a_clear_error() {
    let dir = workdir("settings");
    let out = run(oku_slides(&dir, "en").env("OKU_SLIDES_SERVE__PORT", "not-a-port").arg("schema"));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("serve.port");
}

// ---- Serveur de développement ------------------------------------------------

#[test]
fn serve_answers_the_api_the_dictation_and_static_files() {
    let dir = workdir("serve");
    let ui = dir.join("ui");
    std::fs::create_dir_all(&ui).unwrap();
    std::fs::write(ui.join("index.html"), "<h1>Oku Slides</h1>").unwrap();
    std::fs::write(dir.join("secret.txt"), "privé").unwrap();
    let port = free_port();
    let _server = Server(
        oku_slides(&dir, "fr")
            .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
            .args(["serve", FIXTURE, "--port", &port.to_string(), "--ui"])
            .arg(&ui)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );

    let (status, snapshot) = http(port, "POST", "/api", r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#);
    assert_that!(status).is_equal_to(200);
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_that!(snapshot["slides"].as_array().map(Vec::len)).is_equal_to(Some(8));

    let (status, error) = http(port, "POST", "/api", r#"{"tab":"t-1","req":{"cmd":"slide","id":"absente"}}"#);
    assert_that!(status).is_equal_to(400);
    assert_that!(error).contains("Slide introuvable : absente");

    let (status, error) = http(port, "POST", "/api", "pas du json");
    assert_that!(status).is_equal_to(400);
    assert_that!(error).contains("requête invalide");

    // Un deuxième onglet : vierge, distinct du premier.
    let (status, view) = http(port, "POST", "/api/workspace", r#"{"cmd":"new_tab","window":"w-1"}"#);
    assert_that!(status).is_equal_to(200);
    assert_that!(view.as_str()).contains(r#""id":"t-2""#);
    let (status, blank) = http(port, "POST", "/api", r#"{"tab":"t-2","req":{"cmd":"snapshot"}}"#);
    assert_that!(status).is_equal_to(200);
    let blank: serde_json::Value = serde_json::from_str(&blank).unwrap();
    assert_that!(blank["slides"].as_array().map(Vec::len)).is_equal_to(Some(2));
    let (status, error) = http(port, "POST", "/api", r#"{"tab":"t-9","req":{"cmd":"snapshot"}}"#);
    assert_that!(status).is_equal_to(400);
    assert_that!(error).is_equal_to(r#"{"error":"Cet onglet n'est plus ouvert."}"#.to_owned());
    // Une page web d'une autre origine ne pilote pas le serveur.
    let local = format!("localhost:{port}");
    let with = |host: &str, origin: Option<&str>, body: &str| {
        let mut headers = vec![("Host", host), ("Content-Type", "application/json")];
        headers.extend(origin.map(|o| ("Origin", o)));
        request(port, "POST", "/api/workspace", &headers, body).0
    };
    let evil = with(&local, Some("http://evil.example"), r#"{"cmd":"new_window"}"#);
    assert_that!(evil).is_equal_to(403);
    // DNS rebinding : un nom étranger qui pointe sur 127.0.0.1.
    let rebound = with("evil.example", None, r#"{"cmd":"view"}"#);
    assert_that!(rebound).is_equal_to(403);
    let same = with(&local, Some(&format!("http://localhost:{port}")), r#"{"cmd":"view"}"#);
    assert_that!(same).is_equal_to(200);
    // Le proxy de Vite (`just web`, port 5173) garde l'en-tête `Host` du navigateur.
    let vite = with("localhost:5173", Some("http://localhost:5173"), r#"{"cmd":"view"}"#);
    assert_that!(vite).is_equal_to(200);
    // Les requêtes refusées n'ont rien changé : toujours une fenêtre, deux onglets.
    let (status, view) = http(port, "POST", "/api/workspace", r#"{"cmd":"view"}"#);
    assert_that!(status).is_equal_to(200);
    let view: serde_json::Value = serde_json::from_str(&view).unwrap();
    let windows = view["windows"].as_array().unwrap();
    assert_that!(windows.len()).is_equal_to(1);
    let tabs: Vec<&str> = windows[0]["tabs"].as_array().unwrap().iter().filter_map(|t| t["id"].as_str()).collect();
    assert_that!(tabs).is_equal_to(vec!["t-1", "t-2"]);

    let (status, stt) = http(port, "POST", "/api/stt", r#"{"cmd":"status"}"#);
    assert_that!(status).is_equal_to(200);
    assert_that!(stt).contains("\"state\":\"idle\"");

    let (status, page) = http(port, "GET", "/", "");
    assert_that!(status).is_equal_to(200);
    assert_that!(page).contains("Oku Slides");

    let (status, _) = http(port, "GET", "/../secret.txt", "");
    assert_that!(status).is_equal_to(404);
}

/// Le serveur de développement n'a pas de fenêtre native à quitter : fermer
/// la seule fenêtre est refusé (409, `{error}`), sans rien fermer.
#[test]
fn serve_close_window_on_the_only_window_answers_409_and_keeps_it() {
    let dir = workdir("serve-close-window");
    let ui = dir.join("ui");
    std::fs::create_dir_all(&ui).unwrap_or_else(|e| panic!("dossier ui : {e}"));
    std::fs::write(ui.join("index.html"), "<h1>Oku Slides</h1>").unwrap_or_else(|e| panic!("index.html : {e}"));
    let port = free_port();
    let _server = Server(
        oku_slides(&dir, "fr")
            .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
            .args(["serve", "--port", &port.to_string(), "--ui"])
            .arg(&ui)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("serveur : {e}")),
    );

    let (status, reply) = http(port, "POST", "/api/workspace", r#"{"cmd":"close_window","window":"w-1"}"#);
    assert_that!(status).is_equal_to(409);
    let reply: serde_json::Value = serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réponse illisible : {e}"));
    assert_that!(reply["error"].as_str().is_some_and(|m| !m.is_empty())).is_true();

    let (status, view) = http(port, "POST", "/api/workspace", r#"{"cmd":"view"}"#);
    assert_that!(status).is_equal_to(200);
    let view: serde_json::Value = serde_json::from_str(&view).unwrap_or_else(|e| panic!("vue illisible : {e}"));
    assert_that!(view["windows"].as_array().map(Vec::len)).is_equal_to(Some(1));
    assert_that!(view["windows"][0]["label"].as_str()).is_equal_to(Some("w-1"));
}

/// Un corps de plus de 64 Mio est refusé (413) sans être traité ; le
/// serveur répond ensuite normalement. Un vrai corps : `Content-Length`
/// seul, plus grand que le corps envoyé, n'est pas fiable avec tiny_http.
#[test]
fn serve_body_over_64_mib_answers_413() {
    let dir = workdir("serve-body-cap");
    let ui = dir.join("ui");
    std::fs::create_dir_all(&ui).unwrap_or_else(|e| panic!("dossier ui : {e}"));
    let port = free_port();
    let _server = Server(
        oku_slides(&dir, "fr")
            .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
            .args(["serve", "--port", &port.to_string(), "--ui"])
            .arg(&ui)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("serveur : {e}")),
    );

    let big = " ".repeat(64 * 1024 * 1024 + 1);
    let (status, _) = http(port, "POST", "/api/workspace", &big);
    assert_that!(status).is_equal_to(413);
    let (status, _) = http(port, "POST", "/api/workspace", r#"{"cmd":"view"}"#);
    assert_that!(status).is_equal_to(200);
}

/// Fermer le dernier onglet de la seule fenêtre viderait l'espace de travail :
/// l'interface resterait sur « chargement ». Refusé (409), rien n'est fermé.
#[test]
fn serve_close_last_tab_of_the_only_window_answers_409_and_keeps_it() {
    let dir = workdir("serve-close-last-tab");
    let ui = dir.join("ui");
    std::fs::create_dir_all(&ui).unwrap_or_else(|e| panic!("dossier ui : {e}"));
    std::fs::write(ui.join("index.html"), "<h1>Oku Slides</h1>").unwrap_or_else(|e| panic!("index.html : {e}"));
    let port = free_port();
    let _server = Server(
        oku_slides(&dir, "fr")
            .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
            .args(["serve", "--port", &port.to_string(), "--ui"])
            .arg(&ui)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("serveur : {e}")),
    );

    // Deux onglets : le premier se ferme normalement.
    let (status, _) = http(port, "POST", "/api/workspace", r#"{"cmd":"new_tab","window":"w-1"}"#);
    assert_that!(status).is_equal_to(200);
    let (status, _) = http(port, "POST", "/api/workspace", r#"{"cmd":"close","tab":"t-1","discard":true}"#);
    assert_that!(status).is_equal_to(200);

    // Le dernier : refusé.
    let (status, reply) = http(port, "POST", "/api/workspace", r#"{"cmd":"close","tab":"t-2","discard":true}"#);
    assert_that!(status).is_equal_to(409);
    let reply: serde_json::Value = serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réponse illisible : {e}"));
    assert_that!(reply["error"].as_str().is_some_and(|m| !m.is_empty())).is_true();

    let (status, view) = http(port, "POST", "/api/workspace", r#"{"cmd":"view"}"#);
    assert_that!(status).is_equal_to(200);
    let view: serde_json::Value = serde_json::from_str(&view).unwrap_or_else(|e| panic!("vue illisible : {e}"));
    assert_that!(view["windows"].as_array().map(Vec::len)).is_equal_to(Some(1));
    assert_that!(view["windows"][0]["tabs"][0]["id"].as_str()).is_equal_to(Some("t-2"));
}

#[cfg(unix)]
#[test]
fn serve_stops_cleanly_on_sigterm() {
    let dir = workdir("sigterm");
    let port = free_port();
    let mut server = oku_slides(&dir, "en")
        .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
        .args(["serve", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let (status, _) = http(port, "POST", "/api", r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#);
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
