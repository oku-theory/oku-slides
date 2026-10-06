//! La CLI `oku-slides`, lancée pour de vrai : chaque test a son propre dossier
//! de données (OKU_SLIDES_PATHS__DATA) et choisit sa langue (OKU_SLIDES_I18N__LOCALE).

mod common;

use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{FIXTURE, Server, free_port, http, oku_slides, request, workdir};
use oku_slides_core::{Deck, PackFiles, PackRef, SourceBundle, SourceLimits, WidgetFile, WidgetRegistry};
use oku_slides_session::{PackLibrary, PackManifest, Storage};
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

/// Le thème du deck enregistré dans `pdf` : son `deck.json` extrait, et la version embarquée ?
fn saved_theme(dir: &Path, pdf: &Path) -> (serde_json::Value, bool) {
    let extract = dir.join("extract-theme");
    let opened = run(oku_slides(dir, "en").arg("open").arg(pdf).arg("--out").arg(&extract));
    assert_that!(opened.status.success()).is_true();
    let json = std::fs::read_to_string(extract.join("deck.json")).unwrap_or_else(|e| panic!("deck.json : {e}"));
    let deck: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("deck.json : {e}"));
    let bundle = sources_of(pdf);
    let embedded = bundle.deck().theme.as_ref().is_some_and(|t| bundle.themes.contains_key(&t.version));
    (deck["theme"].clone(), embedded)
}

#[test]
fn save_with_theme_name_embeds_that_builtin_theme() {
    let dir = workdir("save-theme");
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&pdf).args(["--theme", "PRO"]));
    assert_that!(saved.status.success()).is_true();
    let (theme, embedded) = saved_theme(&dir, &pdf);
    assert_that!(theme["id"].as_str()).is_equal_to(Some(oku_slides_session::PRO_THEME.to_string().as_str()));
    assert_that!(embedded).is_true();
}

#[test]
fn save_without_theme_embeds_the_default_theme() {
    let dir = workdir("save-default-theme");
    let pdf = dir.join("deck.oku-slides.pdf");
    assert_that!(run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&pdf)).status.success()).is_true();
    let (theme, embedded) = saved_theme(&dir, &pdf);
    assert_that!(theme["id"].as_str()).is_equal_to(Some(oku_slides_session::DEFAULT_THEME.to_string().as_str()));
    assert_that!(embedded).is_true();
}

#[test]
fn save_with_the_name_of_a_library_theme_embeds_its_current_version() {
    let dir = workdir("save-user-theme");
    let id = uuid::Uuid::new_v4();
    let manifest = oku_slides_session::ThemeManifest {
        id,
        name: "Rouge Vif".into(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: serde_json::json!({"color": {"accent": "#ff0000"}}).as_object().cloned().unwrap(),
    };
    let files = manifest.to_files().unwrap();
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = oku_slides_session::ThemeLibrary::new(Some(dir.join("data").join("themes")), storage);
    library.add_version(id, &files).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&pdf).args(["--theme", "rouge vif"]));
    assert_that!(saved.status.success()).is_true();
    let (theme, embedded) = saved_theme(&dir, &pdf);
    assert_that!(theme["id"].as_str()).is_equal_to(Some(id.to_string().as_str()));
    assert_that!(theme["version"].as_str()).is_equal_to(Some(files.version().as_str()));
    assert_that!(embedded).is_true();
}

#[test]
fn render_with_an_unknown_theme_fails_with_a_localized_message() {
    let dir = workdir("render-unknown-theme");
    let out_dir = dir.join("out");
    let out = run(oku_slides(&dir, "fr").args(["render", FIXTURE, "--theme", "inconnu", "--out"]).arg(&out_dir));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("Thème inconnu");
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
    assert_that!(schema.to_string()).contains("set_param");

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
/// serveur répond ensuite normalement. Un vrai corps, en « chunked » : sans
/// `Content-Length`, la limite tient pendant la lecture (l'annonce d'un
/// `Content-Length` trop grand est couverte par le test suivant).
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

    // Le serveur est prêt.
    assert_that!(http(port, "POST", "/api/workspace", r#"{"cmd":"view"}"#).0).is_equal_to(200);
    let size = 64 * 1024 * 1024 + 1;
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|e| panic!("connexion : {e}"));
    let head = format!(
        "POST /api/workspace HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
         Transfer-Encoding: chunked\r\n\r\n{size:x}\r\n"
    );
    std::io::Write::write_all(&mut s, head.as_bytes()).unwrap_or_else(|e| panic!("requête : {e}"));
    std::io::Write::write_all(&mut s, " ".repeat(size).as_bytes()).unwrap_or_else(|e| panic!("corps : {e}"));
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap_or_else(|e| panic!("délai : {e}"));
    let mut head = [0u8; 12];
    s.read_exact(&mut head).unwrap_or_else(|e| panic!("réponse : {e}"));
    assert_that!(&head).is_equal_to(b"HTTP/1.1 413");
    let (status, _) = http(port, "POST", "/api/workspace", r#"{"cmd":"view"}"#);
    assert_that!(status).is_equal_to(200);
}

/// Un `Content-Length` de plus de 64 Mio est refusé (413, texte) sans
/// attendre le corps.
#[test]
fn serve_announced_body_over_64_mib_answers_413_at_once() {
    use std::io::Write as _;
    let dir = workdir("serve-body-announced");
    let port = free_port();
    let _server = Server(
        oku_slides(&dir, "fr")
            .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
            .args(["serve", "--port", &port.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("serveur : {e}")),
    );
    // Le serveur est prêt.
    assert_that!(http(port, "POST", "/api/workspace", r#"{"cmd":"view"}"#).0).is_equal_to(200);

    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|e| panic!("connexion : {e}"));
    let head = format!(
        "POST /api/workspace HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n",
        64 * 1024 * 1024 + 1
    );
    s.write_all(head.as_bytes()).unwrap_or_else(|e| panic!("requête : {e}"));
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap_or_else(|e| panic!("délai : {e}"));
    let mut buf = [0u8; 1024];
    let n = s.read(&mut buf).unwrap_or_else(|e| panic!("pas de réponse sans le corps : {e}"));
    let reply = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
    assert_that!(reply.as_str()).starts_with("http/1.1 413");
    assert_that!(reply.as_str()).contains("content-type: text/plain; charset=utf-8");
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

/// Un flux d'évènements ouvert ne retient pas l'arrêt : SIGTERM termine le
/// serveur (code 0) en quelques secondes.
#[cfg(unix)]
#[test]
fn serve_stops_on_sigterm_with_an_open_event_stream() {
    use std::io::Write as _;
    let dir = workdir("sigterm-sse");
    let port = free_port();
    let mut server = oku_slides(&dir, "en")
        .env("OKU_SLIDES_PATHS__MODELS", dir.join("models"))
        .args(["serve", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("serveur : {e}"));
    assert_that!(http(port, "POST", "/api", r#"{"tab":"t-1","req":{"cmd":"snapshot"}}"#).0).is_equal_to(200);

    let mut events = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|e| panic!("connexion : {e}"));
    write!(events, "GET /api/events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
        .unwrap_or_else(|e| panic!("requête : {e}"));
    events.set_read_timeout(Some(Duration::from_secs(5))).unwrap_or_else(|e| panic!("délai : {e}"));
    let mut head = [0u8; 12];
    events.read_exact(&mut head).unwrap_or_else(|e| panic!("flux : {e}"));
    assert_that!(&head).is_equal_to(b"HTTP/1.1 200");

    let killed = Command::new("kill").args(["-TERM", &server.id().to_string()]).status();
    assert_that!(killed.is_ok_and(|k| k.success())).is_true();
    let deadline = Instant::now() + Duration::from_secs(10);
    let exit = loop {
        if let Some(exit) = server.try_wait().unwrap_or_else(|e| panic!("attente : {e}")) {
            break exit;
        }
        if Instant::now() > deadline {
            let _ = server.kill();
            panic!("le serveur ne s'arrête pas avec un flux ouvert");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_that!(exit.code()).is_equal_to(Some(0));
}

const LIMITS: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26 };

/// Le pack `quote` du plan 1.
const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Un widget `boucle` qui boucle au rendu, hors des gardes de Typst.
const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

fn base_registry() -> Arc<WidgetRegistry> {
    let probe = serde_json::from_str(include_str!("../../../fixtures/widgets/base-registry.json"))
        .unwrap_or_else(|e| panic!("registre de base : {e}"));
    Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("registre de base : {e}")))
}

/// Une première version du pack `id`, avec ses fichiers de widgets `(nom, source)`.
fn pack_files(id: uuid::Uuid, widgets: &[(&str, &str)]) -> PackFiles {
    let manifest = PackManifest {
        id,
        name: "Pack".to_owned(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".to_owned(),
        templates: Vec::new(),
        examples: Default::default(),
    };
    let widgets = widgets
        .iter()
        .map(|(name, source)| {
            let file = WidgetFile::parse(name).unwrap_or_else(|| panic!("nom de widget invalide : {name}"));
            (file, (*source).to_owned())
        })
        .collect();
    manifest.to_files(widgets).unwrap_or_else(|e| panic!("pack.json : {e}"))
}

/// Ajoute un pack de l'utilisateur à la bibliothèque de `dir` ; sa référence.
fn user_pack(dir: &Path, widgets: &[(&str, &str)]) -> PackRef {
    let id = uuid::Uuid::new_v4();
    let files = pack_files(id, widgets);
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = PackLibrary::new(Some(dir.join("data").join("templates")), storage);
    library.add_version(id, &files).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    PackRef { id, version: files.version() }
}

fn sources_of(pdf: &Path) -> SourceBundle {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("lecture du PDF : {e}"));
    let found = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("sources du PDF : {e}"))
        .unwrap_or_else(|| panic!("PDF sans sources"));
    SourceBundle::from_zip(&found.zip, LIMITS, base_registry()).unwrap_or_else(|e| panic!("sources : {e}")).0
}

#[test]
fn save_of_a_deck_without_templates_embeds_the_default_pack() {
    let dir = workdir("pack-default");
    let pdf = dir.join("deck.oku-slides.pdf");
    assert_that!(run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&pdf)).status.success()).is_true();
    let bundle = sources_of(&pdf);
    let r = bundle.deck().templates.clone().unwrap();
    assert_that!(r.id).is_equal_to(oku_slides_session::DEFAULT_PACK);
    assert_that!(bundle.packs.contains_key(&r.version)).is_true();
}

#[test]
fn save_embeds_the_referenced_pack_and_renders_its_widgets() {
    let dir = workdir("pack-quote");
    let r = user_pack(&dir, &[("quote", QUOTE)]);
    let deck = dir.join("deck.json");
    std::fs::write(
        &deck,
        serde_json::to_vec(&serde_json::json!({ "version": 4, "templates": r,
        "slides": [{ "id": "s", "kind": "content", "params": { "title": "T", "blocks": [
            { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "H" } }
        ] } }] }))
        .unwrap(),
    )
    .unwrap();
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").arg("save").arg(&deck).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    assert_that!(stdout(&saved)).contains(": 1 pages, PDF");
    assert_that!(sources_of(&pdf).packs.contains_key(&r.version)).is_true();
    let out = dir.join("out");
    assert_that!(run(oku_slides(&dir, "en").arg("render").arg(&pdf).arg("--out").arg(&out)).status.success()).is_true();
}

/// Un PDF reçu dont le pack boucle : la CLI s'arrête au délai, sans geler.
#[test]
fn render_of_a_pdf_whose_pack_loops_stops_at_the_timeout() {
    let dir = workdir("pack-loop");
    let base = dir.join("base.oku-slides.pdf");
    assert_that!(run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&base)).status.success()).is_true();
    let pdf = oku_slides_pdf::strip_sources(&std::fs::read(&base).unwrap()).unwrap();
    let id = uuid::Uuid::new_v4();
    let files = pack_files(id, &[("boucle", LOOP_ON_RENDER)]);
    let deck: Deck = serde_json::from_value(serde_json::json!({ "version": 4,
        "templates": { "id": id, "version": files.version() },
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
            { "id": "l", "kind": "boucle", "params": { "body": "x" } }] } }] }))
    .unwrap();
    let mut bundle = SourceBundle::new(deck, base_registry()).unwrap();
    bundle.add_pack(files);
    let looping = dir.join("boucle.oku-slides.pdf");
    std::fs::write(&looping, oku_slides_pdf::embed_sources(&pdf, &bundle.to_zip().unwrap()).unwrap()).unwrap();

    let started = Instant::now();
    let mut child = oku_slides(&dir, "en")
        .env("OKU_SLIDES_RENDER__TIMEOUT_MS", "1000")
        .arg("render")
        .arg(&looping)
        .arg("--out")
        .arg(dir.join("out"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            panic!("la CLI est gelée par le pack qui boucle");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_that!(status.success()).is_false();
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    assert_that!(err).contains("1000 ms");
}

/// Un deck JSON n'embarque pas son pack : signalé, rendu avec la base.
#[test]
fn render_of_a_json_deck_whose_pack_is_not_embedded_warns_and_uses_the_base() {
    let dir = workdir("pack-json");
    let r = user_pack(&dir, &[("quote", QUOTE)]);
    let deck = dir.join("deck.json");
    let json = serde_json::json!({ "version": 4, "templates": r, "slides": [{ "id": "s", "kind": "title" }] });
    std::fs::write(&deck, serde_json::to_vec(&json).unwrap()).unwrap();
    let rendered = run(oku_slides(&dir, "en").arg("render").arg(&deck).arg("--out").arg(dir.join("out")));
    assert_that!(rendered.status.success()).is_true();
    assert_that!(stderr(&rendered)).contains("missing from the file");
}

/// Un PDF reçu dont le pack `quote` n'est pas dans la bibliothèque de `dir`.
fn received_quote_pdf(dir: &Path) -> (std::path::PathBuf, PackRef) {
    let base = dir.join("base.oku-slides.pdf");
    assert_that!(run(oku_slides(dir, "en").args(["save", FIXTURE]).arg(&base)).status.success()).is_true();
    let pdf = oku_slides_pdf::strip_sources(&std::fs::read(&base).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    let id = uuid::Uuid::new_v4();
    let files = pack_files(id, &[("quote", QUOTE)]);
    let r = PackRef { id, version: files.version() };
    let deck: Deck = serde_json::from_value(serde_json::json!({ "version": 4, "templates": r,
        "slides": [{ "id": "s", "kind": "content", "params": { "title": "T", "blocks": [
            { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "H" } }
        ] } }] }))
    .unwrap_or_else(|e| panic!("{e}"));
    let mut bundle = SourceBundle::new(deck, base_registry()).unwrap_or_else(|e| panic!("{e}"));
    bundle.add_pack(files);
    let received = dir.join("recu.oku-slides.pdf");
    std::fs::write(
        &received,
        oku_slides_pdf::embed_sources(&pdf, &bundle.to_zip().unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}")),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    (received, r)
}

/// t-328 : `open --out` extrait le pack, que `save` réembarque sans la bibliothèque.
#[test]
fn open_out_then_save_keeps_a_received_pack_missing_from_the_library() {
    let dir = workdir("pack-recu");
    let (received, r) = received_quote_pdf(&dir);
    let extract = dir.join("extract");
    let opened = run(oku_slides(&dir, "en").arg("open").arg(&received).arg("--out").arg(&extract));
    assert_that!(opened.status.success()).is_true();
    let pack_dir = extract.join("packs").join(r.version.hex());
    assert_that!(pack_dir.join("pack.json").exists()).is_true();
    assert_that!(pack_dir.join("widgets").join("quote.typ").exists()).is_true();

    let pdf = dir.join("again.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").arg("save").arg(extract.join("deck.json")).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    let bundle = sources_of(&pdf);
    assert_that!(bundle.deck().templates.as_ref()).is_equal_to(Some(&r));
    assert_that!(bundle.packs.contains_key(&r.version)).is_true();
}

/// t-328 : une version introuvable partout (ou altérée à côté du deck) arrête `save`.
#[test]
fn save_of_a_deck_whose_pack_is_found_nowhere_fails_with_a_localized_error() {
    let dir = workdir("pack-introuvable");
    let (received, r) = received_quote_pdf(&dir);
    let extract = dir.join("extract");
    assert_that!(run(oku_slides(&dir, "en").arg("open").arg(&received).arg("--out").arg(&extract)).status.success())
        .is_true();
    // Le pack à côté du deck est altéré : sa version ne correspond plus.
    let widgets = extract.join("packs").join(r.version.hex()).join("widgets").join("quote.typ");
    std::fs::write(&widgets, format!("{QUOTE}\n// altéré\n")).unwrap();
    let pdf = dir.join("again.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "fr").arg("save").arg(extract.join("deck.json")).arg(&pdf));
    assert_that!(saved.status.success()).is_false();
    assert_that!(stderr(&saved)).contains("est introuvable");
    assert_that!(pdf.exists()).is_false();
}
