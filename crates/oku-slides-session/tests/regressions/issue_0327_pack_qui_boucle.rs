//! t-327 : un pack dont un widget boucle ne gèle plus la session.
//!
//! Binaire sans harnais : lancé avec `--render-worker`, il est le processus
//! de rendu ; sinon il est le test.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use oku_slides_core::Op;
use oku_slides_platform::RenderLaunch;
use oku_slides_session::{Fonts, Library, PackManifest, Platform, Request, Session, Settings, Storage};

const TEST: &str = "looping_pack_render_returns_within_timeout_with_base_widgets";

/// Délai de rendu. Il borne aussi le premier rendu, à froid, de la session :
/// il doit tenir sous la charge de toute la suite de tests.
const TIMEOUT_MS: u64 = 5_000;
/// Marge au-delà du délai : relance du travailleur et rendu avec la base,
/// sous charge.
const MARGIN_MS: u64 = 10_000;

fn main() {
    oku_slides_render_worker::exit_if_worker();
    // Protocole de découverte de nextest : sans harnais, on annonce l'unique test.
    if std::env::args().any(|a| a == "--list") {
        if !std::env::args().any(|a| a == "--ignored") {
            println!("{TEST}: test");
        }
        return;
    }
    looping_pack_render_returns_within_timeout_with_base_widgets();
    println!("issue_0327_pack_qui_boucle: ok");
}

const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let boucle = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
#let registry = (boucle,)
"#;

fn looping_pack_render_returns_within_timeout_with_base_widgets() {
    let data = tempfile::Builder::new().prefix("oku-slides-0327-").tempdir().unwrap_or_else(|e| panic!("{e}"));
    // Le pack « Boucle » est ajouté à la bibliothèque de la session.
    let id = uuid::Uuid::new_v4();
    let manifest = PackManifest {
        id,
        name: "Boucle".into(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".into(),
        templates: vec![],
    };
    let files = manifest.to_files(Some(LOOP_ON_RENDER.into())).unwrap_or_else(|e| panic!("{e}"));
    let version = files.version();
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    Library::new(Some(data.path().join("templates")), storage)
        .add_version(id, &files)
        .unwrap_or_else(|e| panic!("{e}"));

    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = TIMEOUT_MS;
    let program = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
    let platform = Platform::with_settings(settings)
        .unwrap_or_else(|e| panic!("{e}"))
        .with_render_launch(RenderLaunch::Process { program });
    let mut s = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("{e}"));
    s.open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json")))
        .unwrap_or_else(|e| panic!("{e}"));

    let ops: Vec<Op> = serde_json::from_value(serde_json::json!([
        { "op": "set_templates", "templates": { "id": id, "version": version } },
        { "op": "add_element",
          "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
          "element": { "id": "l", "kind": "boucle", "params": { "body": "x" } } }
    ]))
    .unwrap_or_else(|e| panic!("{e}"));
    let started = Instant::now();
    s.apply("Pack", ops, None).unwrap_or_else(|e| panic!("{e}"));
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_millis(TIMEOUT_MS + MARGIN_MS), "session gelée : {elapsed:?}");
    // L'avis `session-pack-disabled` nomme le pack écarté, par le nom de son manifeste.
    let snapshot: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    let notice = snapshot["notice"].as_str().unwrap_or_default();
    assert!(notice.contains("« Boucle » ont été désactivés"), "avis attendu : {notice:?}");
    let widgets: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    assert!(widgets["widgets"].get("boucle").is_none(), "le pack fautif est encore installé");
}
