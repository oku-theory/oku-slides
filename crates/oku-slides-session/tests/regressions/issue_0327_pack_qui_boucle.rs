//! t-327 : un pack dont un widget boucle ne gèle plus la session.
//!
//! Binaire sans harnais : lancé avec `--render-worker`, il est le processus
//! de rendu ; sinon il est le test.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oku_slides_core::{Op, WidgetFile};
use oku_slides_platform::RenderLaunch;
use oku_slides_session::{Fonts, PackLibrary, PackManifest, Platform, Request, Session, Settings, Storage};

const TEST: &str = "looping_pack_render_falls_back_to_base_widgets";

/// Mémoire du processus de rendu : le widget qui boucle la dépasse vite ; le
/// rendu légitime (deck d'exemple, base) en reste loin.
const MEMORY_MIB: u64 = 1024;
/// Délai de rendu : filet seulement. Il borne aussi chaque appel légitime
/// (rendu à froid, rendu avec la base dans le travailleur relancé), qu'une
/// suite très chargée a déjà fait dépasser 5 s (t-577) : large marge.
/// Le test ne mesure aucune durée : qu'`apply` rende la main, quelle que soit
/// la borne atteinte (mémoire, délai, arrêt), prouve que la session ne gèle pas.
const TIMEOUT_MS: u64 = 120_000;

fn main() {
    oku_slides_render_worker::exit_if_worker();
    // Protocole de découverte de nextest : sans harnais, on annonce l'unique test.
    if std::env::args().any(|a| a == "--list") {
        if !std::env::args().any(|a| a == "--ignored") {
            println!("{TEST}: test");
        }
        return;
    }
    looping_pack_render_falls_back_to_base_widgets();
    println!("issue_0327_pack_qui_boucle: ok");
}

/// Boucle sans fin au rendu en gardant ~1 Kio par tour : la borne de mémoire
/// l'arrête en quelques secondes, sans dépendre de la charge de la machine
/// (le délai reste un filet). Croissance linéaire : jamais d'allocation géante
/// entre deux relevés de la mémoire.
const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let a = (); for _ in range(100000) { for i in range(100000) { a.push(str(i) * 1000) } }; [#a.len()] })
"#;

fn looping_pack_render_falls_back_to_base_widgets() {
    let data = tempfile::Builder::new().prefix("oku-slides-0327-").tempdir().unwrap_or_else(|e| panic!("{e}"));
    // Le pack « Boucle » est ajouté à la bibliothèque de la session.
    let id = uuid::Uuid::new_v4();
    let manifest = PackManifest {
        id,
        name: "Boucle".into(),
        namespace: oku_slides_core::Namespace::parse("boucle").unwrap_or_else(|| panic!("espace de noms")),
        requires: vec![],
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".into(),
        scope: oku_slides_core::PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::new(),
        styles: BTreeMap::new(),
        tokens: Vec::new(),
    };
    let files = manifest
        .to_files(
            oku_slides_core::PackKind::Widgets,
            BTreeMap::from([(WidgetFile::parse("boucle").unwrap_or_else(|| panic!("nom")), LOOP_ON_RENDER.into())]),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let version = files.version();
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    PackLibrary::new(
        Some(data.path().join("widgets")),
        None,
        oku_slides_session::ThemeLibrary::new(None, storage.clone()),
        storage,
    )
    .add_version(id, &files)
    .unwrap_or_else(|e| panic!("{e}"));

    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = TIMEOUT_MS;
    settings.render.memory_mib = MEMORY_MIB;
    settings.render.probe_budget_ms = TIMEOUT_MS;
    let program = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
    let platform = Platform::with_settings(settings)
        .unwrap_or_else(|e| panic!("{e}"))
        .with_render_launch(RenderLaunch::Process { program });
    let mut s =
        Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session non ouverte : {}", e.one_line()));
    s.open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json")))
        .unwrap_or_else(|e| panic!("ouverture refusée : {}", e.one_line()));

    let ops: Vec<Op> = serde_json::from_value(serde_json::json!([
        { "op": "add_pack", "kind": "widgets", "namespace": "boucle", "pin": { "id": id, "version": version } },
        { "op": "add_element",
          "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
          "element": { "id": "l", "kind": "boucle/boucle", "params": { "body": "x" } } }
    ]))
    .unwrap_or_else(|e| panic!("{e}"));
    s.apply("Pack", ops, None).unwrap_or_else(|e| panic!("modification refusée : {}", e.one_line()));
    // L'avis `session-packs-disabled` nomme le pack écarté, par le nom de son manifeste.
    let snapshot: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    let notice = snapshot["notice"].as_str().unwrap_or_default();
    assert!(notice.contains("désactivés parce qu'ils ont échoué : Boucle"), "avis attendu : {notice:?}");
    let widgets: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    assert!(widgets["widgets"].get("boucle/boucle").is_none(), "le pack fautif est encore installé");
}
