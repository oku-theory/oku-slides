use speculoos::prelude::*;

use std::net::IpAddr;

use super::{reachable_from_network, stays_inside, token_url};
use crate::guard::Token;

#[test]
fn stays_inside_accepts_plain_relative_paths() {
    for rel in ["index.html", "assets/app.js", "assets/fonts/a.woff2", "dir/"] {
        assert_that!(stays_inside(rel)).named(rel).is_true();
    }
}

#[test]
fn stays_inside_refuses_paths_that_climb_or_jump() {
    for rel in ["..", "../secret.txt", "a/../../secret.txt", "./index.html", "/etc/passwd"] {
        assert_that!(stays_inside(rel)).named(rel).is_false();
    }
}

#[cfg(windows)]
#[test]
fn stays_inside_refuses_windows_climbs_and_prefixes() {
    for rel in [r"..\secret.txt", r"a\..\..\secret.txt", "C:/Windows/win.ini", r"C:\Windows", r"\\server\share\x"] {
        assert_that!(stays_inside(rel)).named(rel).is_false();
    }
}

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap_or_else(|e| panic!("{text} : {e}"))
}

#[test]
fn loopback_addresses_are_not_reachable_from_the_network() {
    for addr in ["127.0.0.1", "127.0.0.2", "::1", "::ffff:127.0.0.1"] {
        assert_that!(reachable_from_network(ip(addr))).named(addr).is_false();
    }
}

#[test]
fn wildcard_and_network_addresses_are_reachable_from_the_network() {
    for addr in ["0.0.0.0", "::", "192.168.1.10", "10.0.0.2", "fe80::1", "::ffff:192.168.1.10"] {
        assert_that!(reachable_from_network(ip(addr))).named(addr).is_true();
    }
}

#[rstest::rstest]
#[case::network_address("192.168.1.20", "http://192.168.1.20:4870/?token=")]
#[case::wildcard_v4("0.0.0.0", "http://localhost:4870/?token=")]
#[case::wildcard_v6("::", "http://localhost:4870/?token=")]
#[case::ipv6_address("fe80::1", "http://[fe80::1]:4870/?token=")]
fn token_url_names_a_host_the_guard_accepts(#[case] host: &str, #[case] prefix: &str) {
    let token = Token::generate(4870).unwrap_or_else(|e| panic!("jeton : {e}"));
    assert_that!(token_url(host, 4870, &token)).is_equal_to(format!("{prefix}{}", token.secret()));
}

/// A server's shared state on a fresh workspace under `data`, and the runtime its agent stack uses.
fn shared_on(data: &std::path::Path) -> (super::Shared, tokio::runtime::Runtime) {
    use std::sync::{Arc, Mutex};
    let mut settings = oku_slides_session::Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = super::Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let ws = super::Workspace::new(super::Fonts::default(), &platform, None).unwrap_or_else(|e| panic!("{e}"));
    let workspace = Arc::new(Mutex::new(ws));
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap_or_else(|e| panic!("{e}"));
    let sessions: Arc<dyn oku_slides_session::agent::Sessions> = workspace.clone();
    let stt = super::Stt::new(&platform);
    let stack = super::Stack::start(sessions, &platform, Some(stt.clone()), rt.handle().clone());
    let shared = super::Shared {
        workspace,
        stack,
        stt,
        clients: crate::sse::Clients::default(),
        platform,
        ui: std::path::PathBuf::new(),
        host: "127.0.0.1".to_owned(),
        token: None,
        one_at_a_time: Arc::new(tokio::sync::Mutex::new(())),
    };
    (shared, rt)
}

/// Imports `fixtures/import/alto-wealth.pptx` in hi-fi over the blank tab: the imported deck's tab.
fn import_alto(s: &super::Shared) -> String {
    let pptx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/alto-wealth.pptx");
    let req = serde_json::json!({ "cmd": "import_tab", "window": "w-1", "path": pptx, "mode": "hifi" });
    let (status, body) = s.api("/api/workspace", req.to_string().as_bytes());
    assert_that!(status).named(&body).is_equal_to(super::StatusCode::OK);
    super::Workspace::lock(&s.workspace).tab_ids().into_iter().next().unwrap_or_else(|| panic!("a tab"))
}

/// Opens the deck-only layouts pack of `deck` in its deck-pack tab: that tab.
fn open_deck_pack(s: &super::Shared, deck: &str) -> String {
    let pack = super::Workspace::with_session(&s.workspace, deck, |s| {
        s.deck().layouts.get(&oku_slides_core::Namespace::deck_layouts()).map(|p| p.id)
    })
    .0
    .ok()
    .flatten()
    .unwrap_or_else(|| panic!("the import's deck pack"));
    let req = serde_json::json!({ "cmd": "open_pack", "tab": deck, "pack": pack });
    let (status, body) = s.api("/api/workspace", req.to_string().as_bytes());
    assert_that!(status).named(&body).is_equal_to(super::StatusCode::OK);
    let tabs = super::Workspace::lock(&s.workspace).tab_ids();
    tabs.into_iter().find(|t| t != deck).unwrap_or_else(|| panic!("a deck-pack tab"))
}

#[test]
fn api_forward_runs_after_change_on_the_owner() {
    let data = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let (s, rt) = shared_on(data.path());
    let deck = import_alto(&s);
    let tab = open_deck_pack(&s, &deck);
    let rename = serde_json::json!({ "op": "set_deck_param", "key": "title", "value": "Modèles" });
    let edit = serde_json::json!({ "tab": tab, "req": { "cmd": "apply", "label": "Nom", "ops": [rename] } });
    assert_that!(s.api("/api", edit.to_string().as_bytes()).0).is_equal_to(super::StatusCode::OK);
    // `api` itself, minus the agent: the request, then `drain_owners` handing each owner to `after_change`.
    let save = serde_json::from_value(serde_json::json!({ "cmd": "save_pack", "as_new": false })).unwrap();
    let (result, _) = super::Workspace::request(&s.workspace, &tab, save);
    assert_that!(result.is_ok()).is_true();
    let mut owners = Vec::new();
    super::drain_owners(&s.workspace, |owner| owners.push(owner.to_owned()));
    super::drain_owners(&s.workspace, |owner| owners.push(owner.to_owned()));
    assert_that!(owners).is_equal_to(vec![deck]);
    rt.block_on(s.stack.shutdown());
}

#[test]
fn close_of_an_owner_whose_links_are_all_tabs_is_409() {
    let data = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let (s, rt) = shared_on(data.path());
    let deck = import_alto(&s);
    let tab = open_deck_pack(&s, &deck);
    let close = serde_json::json!({ "cmd": "close", "tab": deck });
    let (status, _) = s.api("/api/workspace", close.to_string().as_bytes());
    assert_that!(status).is_equal_to(super::StatusCode::CONFLICT);
    assert_that!(super::Workspace::lock(&s.workspace).tab_ids()).is_equal_to(vec![deck, tab]);
    rt.block_on(s.stack.shutdown());
}
