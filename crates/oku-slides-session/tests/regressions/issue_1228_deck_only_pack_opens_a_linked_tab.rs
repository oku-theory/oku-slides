//! t-1228: a deck-only pack opens in a tab linked to its deck, whose saves reach the deck.
//!
//! Before t-1228, opening an imported deck's own layouts pack entered a workshop
//! inside the deck tab. It now opens a deck-pack tab right after the deck, and a
//! save there is forwarded to the deck, which pins the new version.

use std::sync::mpsc::channel;
use std::time::Duration;

use oku_slides_core::{Namespace, Op};
use oku_slides_session::{Request, TabDocument, Workspace};
use speculoos::prelude::*;

use crate::common::{Data, data_dir};
use crate::workspace_support::{import_hifi, on, open_pack, workspace};

fn scenario(data: &Data) {
    let ws = workspace(data);
    let deck = import_hifi(&ws, data);
    let pinned = |ws: &std::sync::Mutex<Workspace>| {
        on(ws, &deck, |s| s.deck().layouts.get(&Namespace::deck_layouts()).cloned())
            .unwrap_or_else(|| panic!("the import's deck pack"))
    };
    let before = pinned(&ws);
    let tab = open_pack(&ws, &deck, before.id);
    let document =
        Workspace::lock(&ws).view().windows[0].tabs.iter().find(|t| t.id == tab).map(|t| t.info.document.clone());
    assert_that!(document).is_equal_to(Some(TabDocument::DeckPack { deck_tab: deck.clone(), pack: before.id }));
    let rename: Op =
        serde_json::from_value(serde_json::json!({ "op": "set_deck_param", "key": "title", "value": "Modèles" }))
            .unwrap_or_else(|e| panic!("{e}"));
    on(&ws, &tab, |s| s.apply("Nom", vec![rename], None)).unwrap_or_else(|e| panic!("rename: {}", e.one_line()));
    let (saved, _) = Workspace::request(&ws, &tab, Request::SavePack { as_new: false });
    saved.unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    let after = pinned(&ws);
    assert_that!((after.id, after.version != before.version)).is_equal_to((before.id, true));
    assert_that!(Workspace::take_touched(&ws)).is_equal_to(vec![deck]);
}

#[test]
fn issue_1228_deck_only_pack_save_in_its_tab_reaches_the_deck() {
    let (done, finished) = channel();
    std::thread::spawn(move || {
        scenario(&data_dir());
        done.send(()).unwrap_or_else(|e| panic!("{e}"));
    });
    finished.recv_timeout(Duration::from_secs(120)).unwrap_or_else(|e| panic!("timed out: {e}"));
}
