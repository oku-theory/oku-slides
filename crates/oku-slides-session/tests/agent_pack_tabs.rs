//! A deck's agent opens a library pack through the workspace: a pack tab beside its deck (t-1195).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use common::data_dir;
use oku_slides_core::{Namespace, PackKind, PackScope};
use oku_slides_session::agent::{TabGate, TabOpen, TabSession};
use oku_slides_session::{
    Fonts, PackLibrary, PackManifest, Platform, SessionErrorKind, Settings, Workspace, WorkspaceRequest,
};
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

const ACME: Uuid = Uuid::from_u128(0xac3e_0000_0000_4000_8000_0000_0000_0001);

const CARD: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("card", params: (body: p.rich()), (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

fn ns(s: &str) -> Namespace {
    Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"))
}

/// A workspace whose library holds the widget pack `acme`; its deck tab `t-1` pins it.
struct Deck {
    ws: Arc<Mutex<Workspace>>,
    /// The deck tab's agent, as the MCP tools see it.
    agent: TabSession,
    /// Dropped last.
    _data: common::Data,
}

fn deck() -> Deck {
    let data = data_dir();
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    let manifest = PackManifest {
        id: ACME,
        name: "acme".into(),
        namespace: ns("acme"),
        requires: vec![ns("base")],
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::new(),
        styles: BTreeMap::new(),
        tokens: Vec::new(),
    };
    let files = manifest.to_files(PackKind::Widgets, BTreeMap::new()).unwrap_or_else(|e| panic!("manifeste : {e}"));
    PackLibrary::user(&platform).add_version(ACME, &files).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    let workspace = Workspace::new(Fonts::default(), &platform, None)
        .unwrap_or_else(|e| panic!("espace de travail : {}", e.one_line()));
    let ws = Arc::new(Mutex::new(workspace));
    let agent = TabSession::new("t-1", ws.clone());
    agent
        .with(|s| s.add_library_pack(PackKind::Widgets, &ns("acme")))
        .and_then(|r| r)
        .unwrap_or_else(|e| panic!("le deck épingle acme : {}", e.one_line()));
    Deck { ws, agent, _data: data }
}

impl Deck {
    /// The agent opens `acme`; the questions its gate was asked, and the answer.
    fn open(&self) -> (Vec<TabGate>, TabOpen) {
        let mut asked = Vec::new();
        let opened = self
            .agent
            .open_pack(ACME, None, &mut |gate| {
                asked.push(gate);
                true
            })
            .unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
        (asked, opened)
    }

    fn tabs(&self) -> Vec<String> {
        Workspace::lock(&self.ws).tab_ids()
    }

    /// The deck agent writes `card` to the pack `pack` (or the deck's own).
    fn write(&self, pack: Option<&str>) -> oku_slides_session::Result<()> {
        let pack = pack.map(ns);
        self.agent.with(|s| s.set_widget_by_agent(pack.as_ref(), "card", CARD.into()).map(|_| ())).and_then(|r| r)
    }
}

#[test]
fn deck_agent_open_pack_opens_a_pack_tab_beside_its_deck() {
    let d = deck();
    let (asked, opened) = d.open();
    assert_that!(opened).is_equal_to(TabOpen::Opened { tab: "t-2".to_owned(), shown: true });
    assert_that!(asked).is_equal_to(vec![TabGate::NewTab]);
    assert_that!(d.tabs()).is_equal_to(vec!["t-1".to_owned(), "t-2".to_owned()]);
}

#[test]
fn deck_agent_set_widget_without_namespace_still_writes_the_deck_pack() {
    let d = deck();
    d.open();
    assert_that!(d.write(None).is_ok()).is_true();
}

#[test]
fn deck_agent_set_widget_on_a_pack_open_in_a_tab_is_refused() {
    let d = deck();
    d.open();
    let refused = d.write(Some("acme"));
    assert_that!(refused.err().map(|e| e.kind())).is_equal_to(Some(SessionErrorKind::PackOpenInTab));
}

#[test]
fn deck_agent_set_widget_after_the_pack_tab_closes_is_allowed() {
    let d = deck();
    d.open();
    let close: WorkspaceRequest = serde_json::from_value(json!({ "cmd": "close", "tab": "t-2", "discard": true }))
        .unwrap_or_else(|e| panic!("requête invalide : {e}"));
    Workspace::apply(&d.ws, close).unwrap_or_else(|e| panic!("fermeture : {}", e.one_line()));
    assert_that!(d.write(Some("acme")).is_ok()).is_true();
}
