//! t-1295: an imported deck's images drew as fallback frames after reopening.
//!
//! This guards the whole reopening, as in the app; it passed before the fix too. The
//! cause, host and worker disagreeing on sent assets after a worker restart, is pinned in
//! the render worker's unit tests (`client_tests`, `link_fake_tests`, `serve_tests`).
//!
//! Binary without harness, like the app: run with `--render-worker`, it is the render
//! process; otherwise it is the test, rendering in that subprocess.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

#[path = "../common/hifi_import.rs"]
#[expect(dead_code, reason = "shared helpers: this binary uses only fixture_pptx")]
mod hifi_import;

use std::path::Path;
use std::sync::Mutex;

use speculoos::prelude::*;

use oku_slides_platform::RenderLaunch;
use oku_slides_session::import::ImportTarget;
use oku_slides_session::{Fonts, Platform, Request, Session, Settings, Workspace, WorkspaceRequest};

/// The fill of the fallback frame (`luma(230)`, `main_source` of the render crate).
const MISSING_FILL: &str = "#e6e6e6";

/// A render budget generous enough for a loaded suite (see `issue_0327`, t-577).
const TIMEOUT_MS: u64 = 60_000;

const TEST: &str = "resume_imported_deck_draws_its_images";

fn main() {
    oku_slides_render_worker::exit_if_worker();
    // Nextest discovery protocol: without a harness, the single test is announced.
    if std::env::args().any(|a| a == "--list") {
        if !std::env::args().any(|a| a == "--ignored") {
            println!("{TEST}: test");
        }
        return;
    }
    for hifi in [true, false] {
        for saved in [false, true] {
            resume_imported_deck_draws_its_images(hifi, saved);
        }
    }
    println!("issue_1295_imported_images_survive_reopening: ok");
}

/// The workspace resumed from `data`, rendering in a subprocess as the app does.
fn workspace(data: &Path) -> Mutex<Workspace> {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = TIMEOUT_MS;
    settings.render.probe_budget_ms = TIMEOUT_MS;
    let program = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
    let platform = Platform::with_settings(settings)
        .unwrap_or_else(|e| panic!("platform: {e}"))
        .with_render_launch(RenderLaunch::Process { program });
    let ws = Workspace::resume(Fonts::default(), &platform, &data.join("resume"))
        .unwrap_or_else(|e| panic!("resume: {}", e.one_line()));
    Mutex::new(ws)
}

/// `f` on the session of `tab`.
fn on<R>(ws: &Mutex<Workspace>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> R {
    Workspace::with_session(ws, tab, f).0.unwrap_or_else(|e| panic!("tab {tab}: {}", e.one_line()))
}

/// The `<image` tags and the missing-image frames of every slide of `tab`, rendered.
fn drawn_images(ws: &Mutex<Workspace>, tab: &str) -> (usize, usize) {
    on(ws, tab, |s| {
        let ids: Vec<String> = s.deck().slides.iter().map(|x| x.id.clone()).collect();
        ids.iter().fold((0, 0), |(images, frames), id| {
            let svg = s.svg(id).unwrap_or_else(|e| panic!("{id}: {}", e.one_line()));
            (images + svg.matches("<image").count(), frames + svg.matches(MISSING_FILL).count())
        })
    })
}

/// Imports `fixtures/import/kitchen` into a new tab of `w-1`: its id.
fn import(ws: &Mutex<Workspace>, data: &Path, hifi: bool) -> String {
    let path = hifi_import::fixture_pptx(data, "kitchen");
    let target = if hifi {
        ImportTarget::Hifi { witness: None }
    } else {
        ImportTarget::Retrofit { layouts: Vec::new(), theme: None }
    };
    let req = WorkspaceRequest::ImportTab { window: "w-1".to_owned(), path, target, tune: false };
    let before = Workspace::lock(ws).tab_ids();
    Workspace::apply(ws, req).unwrap_or_else(|e| panic!("import refused: {}", e.one_line()));
    let added: Vec<String> = Workspace::lock(ws).tab_ids().into_iter().filter(|t| !before.contains(t)).collect();
    assert_that!(added).has_length(1);
    added[0].clone()
}

fn resume_imported_deck_draws_its_images(hifi: bool, saved: bool) {
    let data = tempfile::Builder::new().prefix("oku-slides-1295-").tempdir().unwrap_or_else(|e| panic!("{e}"));
    let ws = workspace(data.path());
    let tab = import(&ws, data.path(), hifi);
    if saved {
        let path = Some(data.path().join("decks").join("kitchen.oku-slides.pdf"));
        Workspace::request(&ws, &tab, Request::Save { path, overwrite: false })
            .0
            .unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    }
    let before = drawn_images(&ws, &tab);
    let case = format!("hifi {hifi}, saved {saved}");
    assert_that!(before.0).named(&format!("{case}: images drawn on import")).is_greater_than(0);
    assert_that!(before.1).named(&format!("{case}: fallback frames on import")).is_equal_to(0);
    Workspace::settle(&ws);
    drop(ws);

    let again = workspace(data.path());
    let after = drawn_images(&again, &tab);
    assert_that!(after).named(&format!("{case}: (images, fallback frames) after reopening")).is_equal_to(before);
}
