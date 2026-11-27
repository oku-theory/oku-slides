//! t-1178: pasting into tab B an image copied in tab A was refused (`UnknownAsset`): its
//! bytes stayed in A's bundle. `PasteElement` brings them to B, which keeps them in its
//! bundle only because the paste names them.

use std::sync::Mutex;

use oku_slides_core::Element;
use oku_slides_session::{Result, SessionErrorKind, Workspace, WorkspaceOutcome, WorkspaceRequest};
use serde_json::json;
use speculoos::prelude::*;

use oku_slides_session::agent::Upload;

use crate::common::{data_dir, fixture_path};
use crate::workspace_support::{on, op, png, send, workspace};

/// Deux onglets sur roadmap.json ; dans le premier, `e-image` cite une image du bundle.
/// Les onglets et l'image copiée.
fn two_tabs(ws: &Mutex<Workspace>) -> (String, String, Element) {
    let asset = oku_slides_core::asset_id(&png());
    let window = send(ws, json!({ "cmd": "view" }))["windows"][0]["label"].as_str().map(str::to_owned);
    let window = window.unwrap_or_else(|| panic!("aucune fenêtre"));
    send(ws, json!({ "cmd": "new_tab", "window": window }));
    let tabs: Vec<String> = Workspace::lock(ws).view().windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    let (a, b) = (tabs[0].clone(), tabs[1].clone());
    for tab in [&a, &b] {
        on(ws, tab, |s| s.open(&fixture_path())).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
    }
    let element = on(ws, &a, |s| {
        let pasted = s.attach(Upload::new(png(), ""), &[]).unwrap_or_else(|e| panic!("collage : {}", e.one_line()));
        let set = op(json!({
            "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" },
            "param": "asset", "value": pasted.id()
        }));
        s.apply("image", vec![set], None).unwrap_or_else(|e| panic!("apply : {}", e.one_line()));
        let slide = s.deck().slides.iter().find(|x| x.id == "s-libre").cloned();
        let slide = slide.unwrap_or_else(|| panic!("s-libre absente"));
        let element = slide.locate("e-image").map(|l| l.element.clone());
        assert_that!(pasted.id().to_owned()).is_equal_to(&asset);
        element.unwrap_or_else(|| panic!("e-image absente"))
    });
    (a, b, element)
}

/// The paste into `to` of a copy of `element` under a new id, copied in `from`.
fn paste(ws: &Mutex<Workspace>, from: &str, to: &str, element: &Element) -> Result<WorkspaceOutcome> {
    let mut copy = element.clone();
    copy.id = "e-image-copie".into();
    let slot = json!({ "owner": { "slide": "s-libre" }, "param": "elements" });
    let ops = [json!({ "op": "add_element", "slot": slot, "element": copy })];
    let req = json!({ "cmd": "paste_element", "from": from, "to": to, "label": "coller", "ops": ops });
    let req: WorkspaceRequest = serde_json::from_value(req).unwrap_or_else(|e| panic!("invalid request: {e}"));
    Workspace::apply(ws, req)
}

#[test]
fn paste_from_another_tab_brings_the_image_bytes() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, b, element) = two_tabs(&ws);
    paste(&ws, &a, &b, &element).unwrap_or_else(|e| panic!("paste refused: {}", e.one_line()));
    let bytes = on(&ws, &b, |s| s.element_assets(&element));
    assert_that!(bytes).is_equal_to(vec![png()]);
}

#[test]
fn paste_from_a_closed_tab_is_refused_as_an_unknown_asset() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, b, element) = two_tabs(&ws);
    send(&ws, json!({ "cmd": "close", "tab": a, "discard": true }));
    let e = paste(&ws, &a, &b, &element).err().map(|e| e.kind());
    assert_that!(e).is_equal_to(Some(SessionErrorKind::UnknownAsset));
}

#[test]
fn paste_from_a_dormant_tab_brings_nothing() {
    let data = data_dir();
    let (a, b, element) = {
        let ws = workspace(&data);
        let (a, b, element) = two_tabs(&ws);
        Workspace::settle(&ws);
        (a, b, element)
    };
    // Read back: the tabs sleep; reading bytes does not wake `a`.
    let ws = workspace(&data);
    let asleep = paste(&ws, &a, &b, &element).err().map(|e| e.kind());
    // Woken, the same tab brings its image: sleep held everything back.
    on(&ws, &a, |_| ());
    let awake = paste(&ws, &a, &b, &element).map(|_| ()).map_err(|e| e.one_line());
    assert_that!((asleep, awake)).is_equal_to((Some(SessionErrorKind::UnknownAsset), Ok(())));
}

#[test]
fn paste_into_an_unknown_tab_is_refused() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, _, element) = two_tabs(&ws);
    let e = paste(&ws, &a, "t-99", &element).err().map(|e| e.kind());
    assert_that!(e).is_equal_to(Some(SessionErrorKind::UnknownTab));
}
