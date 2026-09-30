//! t-327 : un pack dont un widget boucle ne gèle plus la session.
//!
//! Binaire sans harnais : lancé avec `--render-worker`, il est le processus
//! de rendu ; sinon il est le test.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use oku_slides_core::Op;
use oku_slides_platform::RenderLaunch;
use oku_slides_session::{Fonts, Platform, Request, Session, Settings};

const TEST: &str = "looping_pack_render_returns_within_timeout_with_base_widgets";

/// Délai de rendu : celui des tests de packs qui bouclent (`LOOP_TIMEOUT_MS`).
const TIMEOUT_MS: u64 = 2_000;
/// Marge au-delà du délai : relance du travailleur et rendu avec la base.
const MARGIN_MS: u64 = 3_000;

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
    let pack = data.path().join("templates").join("boucle");
    std::fs::create_dir_all(&pack).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(pack.join("pack.json"), r#"{"id":"boucle","name":"Boucle","templates":[]}"#)
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(pack.join("widgets.typ"), LOOP_ON_RENDER).unwrap_or_else(|e| panic!("{e}"));

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
        { "op": "set_templates", "templates": "boucle" },
        { "op": "add_element",
          "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
          "element": { "id": "l", "kind": "boucle", "params": { "body": "x" } } }
    ]))
    .unwrap_or_else(|e| panic!("{e}"));
    let started = Instant::now();
    s.apply("Pack", ops, None).unwrap_or_else(|e| panic!("{e}"));
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_millis(TIMEOUT_MS + MARGIN_MS), "session gelée : {elapsed:?}");
    // L'avis `session-pack-disabled` nomme le pack écarté.
    let snapshot: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    let notice = snapshot["notice"].as_str().unwrap_or_default();
    assert!(notice.contains("« boucle » ont été désactivés"), "avis attendu : {notice:?}");
    let widgets: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    assert!(widgets["widgets"].get("boucle").is_none(), "le pack fautif est encore installé");
}
