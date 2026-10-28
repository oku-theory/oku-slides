//! t-1224: a deck-mode widget write built on a dormant pack tab's unfrozen draft.
//!
//! A pack tab saves its draft `v1`, then sleeps without its lease (a restart). The
//! deck accepts `v1` and writes `v2` on top of it. The woken tab's next save must not
//! replace `v1`: that would leave `v2`'s parent out of the library. Every public path
//! that makes a deck pin a version freezes it first (`refresh_pack_updates` then
//! `adopt_pins`): that is the safety this test guards.

use std::sync::Mutex;
use std::sync::mpsc::channel;
use std::time::Duration;

use oku_slides_core::{PackKind, PackVersion};
use oku_slides_session::{PackLibrary, PackManifest, Request, Session, Storage, Workspace};
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{Data, add_user_pack, data_dir, no_user_themes, ns};
use crate::workspace_support::{on, send, workspace};

const QUOTE_V1: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()),
  (args, ctx) => field(ctx, "body", strong(args.body)))
"#;

const QUOTE_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()),
  (args, ctx) => field(ctx, "body", underline(args.body)))
"#;

const QUOTE_V3: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()),
  (args, ctx) => field(ctx, "body", highlight(args.body)))
"#;

/// The widget library of the data folder `data`.
fn packs(data: &Data) -> PackLibrary {
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    PackLibrary::new(Some(data.join("widgets")), None, no_user_themes(), storage)
}

/// The workspace on `data`, `t-1` a deck tab pinning `acme`, and `t-2` its pack tab with a saved draft `v1`.
///
/// Settled and dropped: `t-2` is left dormant, without its lease. The saved `v1`.
fn deck_with_a_dormant_pack_tab(data: &Data, acme: Uuid) -> PackVersion {
    let ws = workspace(data);
    on(&ws, "t-1", |s| s.add_library_pack(PackKind::Widgets, &ns("acme")))
        .unwrap_or_else(|e| panic!("épinglage : {}", e.one_line()));
    send(&ws, json!({ "cmd": "open_pack", "tab": "t-1", "pack": acme }));
    on(&ws, "t-2", |s| save_widget(s, QUOTE_V1));
    Workspace::settle(&ws);
    packs(data).current(acme).unwrap_or_else(|| panic!("version courante"))
}

/// Writes `source` as the `quote` widget of the pack tab `s`, then saves the pack.
///
/// Through `handle`, which persists the tab's state as the app does: a dormant tab keeps its draft.
fn save_widget(s: &mut Session, source: &str) {
    let set = Request::SetWidget { pack: Some(ns("acme")), name: "quote".into(), source: source.into() };
    s.handle(set).unwrap_or_else(|e| panic!("widget : {}", e.one_line()));
    s.handle(Request::SavePack { as_new: false }).unwrap_or_else(|e| panic!("enregistrement : {}", e.one_line()));
}

/// The parent recorded in the manifest of `version` of `acme`; `None` if the library lost it or it has none.
fn parent_of(library: &PackLibrary, acme: Uuid, version: &PackVersion) -> Option<PackVersion> {
    let files = library.version(acme, version)?;
    PackManifest::from_files(&files).unwrap_or_else(|e| panic!("manifeste : {e}")).parent
}

fn scenario(data: &Data) {
    let acme = add_user_pack(data, "acme");
    let v0 = packs(data).current(acme).unwrap_or_else(|| panic!("version courante"));
    let v1 = deck_with_a_dormant_pack_tab(data, acme);
    assert_that!(v1).is_not_equal_to(v0.clone());
    let ws: Mutex<Workspace> = workspace(data);
    // The deck accepts `v1`, then writes `v2` on top of it.
    on(&ws, "t-1", |s| s.pack_use_mine(PackKind::Widgets, &ns("acme")))
        .unwrap_or_else(|e| panic!("mise à jour : {}", e.one_line()));
    on(&ws, "t-1", |s| {
        s.handle(Request::SetWidget { pack: Some(ns("acme")), name: "quote".into(), source: QUOTE_V2.into() })
    })
    .unwrap_or_else(|e| panic!("widget du deck : {}", e.one_line()));
    let v2 = packs(data).current(acme).unwrap_or_else(|| panic!("version courante"));
    // The dormant pack tab wakes and saves on its own base.
    on(&ws, "t-2", |s| {
        s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("réveil : {}", e.one_line()));
        save_widget(s, QUOTE_V3);
    });
    let v3 = packs(data).current(acme).unwrap_or_else(|| panic!("version courante"));
    let library = packs(data);
    assert_that!(library.version(acme, &v1).is_some()).is_true();
    assert_that!(v2).is_not_equal_to(v1.clone());
    assert_that!(parent_of(&library, acme, &v2)).is_equal_to(Some(v1.clone()));
    // A sibling of the deck's write, not a replacement of `v1`.
    assert_that!(parent_of(&library, acme, &v3)).is_equal_to(Some(v1));
}

#[test]
fn issue_1224_woken_pack_tab_save_keeps_the_parent_of_a_deck_pack_write() {
    let (done, finished) = channel();
    std::thread::spawn(move || {
        scenario(&data_dir());
        done.send(()).unwrap_or_else(|e| panic!("{e}"));
    });
    finished.recv_timeout(Duration::from_secs(120)).unwrap_or_else(|e| panic!("délai dépassé : {e}"));
}
