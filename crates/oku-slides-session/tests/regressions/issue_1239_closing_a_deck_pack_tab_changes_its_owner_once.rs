//! t-1239: closing an edited deck-pack tab ran its owner's after-change twice.
//!
//! The close flushed the tab to its owner, which named the owner in `touched`,
//! and also answered `DeckChanged` for it, so the host ran the owner's
//! after-change once per path. The close now answers `Applied`: the touched
//! drain alone names the owner.

use std::sync::mpsc::channel;
use std::time::Duration;

use oku_slides_core::{Namespace, Op};
use oku_slides_session::Workspace;
use speculoos::prelude::*;

use crate::common::{Data, data_dir};
use crate::workspace_support::{import_hifi, on, open_pack, send, workspace};

fn scenario(data: &Data) {
    let ws = workspace(data);
    let deck = import_hifi(&ws, data);
    let pack = on(&ws, &deck, |s| s.deck().layouts.get(&Namespace::deck_layouts()).map(|pin| pin.id))
        .unwrap_or_else(|| panic!("the import's deck pack"));
    let tab = open_pack(&ws, &deck, pack);
    let rename: Op =
        serde_json::from_value(serde_json::json!({ "op": "set_deck_param", "key": "title", "value": "Modèles" }))
            .unwrap_or_else(|e| panic!("{e}"));
    on(&ws, &tab, |s| s.apply("Nom", vec![rename], None)).unwrap_or_else(|e| panic!("rename: {}", e.one_line()));
    drop(Workspace::take_touched(&ws));
    // `send` panics on any outcome but `Applied`.
    send(&ws, serde_json::json!({ "cmd": "close", "tab": tab }));
    assert_that!(Workspace::take_touched(&ws)).is_equal_to(vec![deck]);
}

#[test]
fn issue_1239_closing_a_deck_pack_tab_changes_its_owner_once() {
    let (done, finished) = channel();
    std::thread::spawn(move || {
        scenario(&data_dir());
        done.send(()).unwrap_or_else(|e| panic!("{e}"));
    });
    finished.recv_timeout(Duration::from_secs(120)).unwrap_or_else(|e| panic!("timed out: {e}"));
}
