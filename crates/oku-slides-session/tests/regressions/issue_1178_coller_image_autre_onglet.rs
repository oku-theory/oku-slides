//! t-1178 : coller dans l'onglet B une image copiée dans l'onglet A était refusé
//! (`UnknownAsset`) : ses octets restaient dans le bundle de A. `CarryAssets` les
//! apporte à B, qui ne les range dans son bundle que quand le collage les cite.

use std::io::Cursor;
use std::sync::Mutex;

use oku_slides_core::{Element, Op};
use oku_slides_session::{
    Fonts, Platform, Session, SessionErrorKind, Settings, Workspace, WorkspaceOutcome, WorkspaceRequest,
};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{Data, data_dir, fixture_path};

fn workspace(data: &Data) -> Mutex<Workspace> {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    let ws = Workspace::resume(Fonts::default(), &platform, data.join("resume"))
        .unwrap_or_else(|e| panic!("reprise impossible : {}", e.one_line()));
    Mutex::new(ws)
}

fn send(ws: &Mutex<Workspace>, v: serde_json::Value) -> serde_json::Value {
    let req: WorkspaceRequest = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"));
    match Workspace::apply(ws, req).unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line())) {
        WorkspaceOutcome::Applied(json) => serde_json::from_str(&json).unwrap_or_else(|e| panic!("réponse : {e}")),
        other => panic!("réponse inattendue : {other:?}"),
    }
}

fn on<R>(ws: &Mutex<Workspace>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> R {
    Workspace::with_session(ws, tab, f).0.unwrap_or_else(|e| panic!("onglet {tab} : {}", e.one_line()))
}

/// Un PNG gris de 4 × 4 pixels.
fn png() -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, 4, 4);
    e.set_color(png::ColorType::Grayscale);
    let mut writer = e.write_header().unwrap_or_else(|e| panic!("png : {e}"));
    writer.write_image_data(&[7; 16]).unwrap_or_else(|e| panic!("png : {e}"));
    drop(writer);
    out.into_inner()
}

fn op(v: serde_json::Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op invalide : {e}"))
}

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
        let pasted = s.paste_image(png(), &[]).unwrap_or_else(|e| panic!("collage : {}", e.one_line()));
        let set = op(json!({
            "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" },
            "param": "asset", "value": pasted.id
        }));
        s.apply("image", vec![set], None).unwrap_or_else(|e| panic!("apply : {}", e.one_line()));
        let slide = s.deck().slides.iter().find(|x| x.id == "s-libre").cloned();
        let slide = slide.unwrap_or_else(|| panic!("s-libre absente"));
        let element = slide.locate("e-image").map(|l| l.element.clone());
        assert_that!(pasted.id).is_equal_to(&asset);
        element.unwrap_or_else(|| panic!("e-image absente"))
    });
    (a, b, element)
}

/// L'op du collage dans B : une copie de `element` sous un nouvel id.
fn paste(element: &Element) -> Op {
    let mut copy = element.clone();
    copy.id = "e-image-copie".into();
    let slot = json!({ "owner": { "slide": "s-libre" }, "param": "elements" });
    op(json!({ "op": "add_element", "slot": slot, "element": copy }))
}

#[test]
fn paste_from_another_tab_brings_the_image_bytes() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, b, element) = two_tabs(&ws);
    let carried = send(&ws, json!({ "cmd": "carry_assets", "from": a, "to": b, "element": element }));
    assert_that!(carried).is_equal_to(json!({ "carried": 1 }));
    on(&ws, &b, |s| s.apply("coller", vec![paste(&element)], None))
        .unwrap_or_else(|e| panic!("collage refusé : {}", e.one_line()));
    let bytes = on(&ws, &b, |s| s.element_assets(&element));
    assert_that!(bytes).is_equal_to(vec![png()]);
}

#[test]
fn paste_from_a_closed_tab_is_refused_as_an_unknown_asset() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, b, element) = two_tabs(&ws);
    send(&ws, json!({ "cmd": "close", "tab": a, "discard": true }));
    let carried = send(&ws, json!({ "cmd": "carry_assets", "from": a, "to": b, "element": element }));
    assert_that!(carried).is_equal_to(json!({ "carried": 0 }));
    let e = on(&ws, &b, |s| s.apply("coller", vec![paste(&element)], None)).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
}

#[test]
fn carry_from_a_dormant_tab_brings_nothing() {
    let data = data_dir();
    let (a, b, element) = {
        let ws = workspace(&data);
        let (a, b, element) = two_tabs(&ws);
        Workspace::settle(&ws);
        (a, b, element)
    };
    // Relu : les onglets dorment ; lire des octets ne réveille pas `a`.
    let ws = workspace(&data);
    let asleep = send(&ws, json!({ "cmd": "carry_assets", "from": a, "to": b, "element": element }));
    // Réveillé, le même onglet apporte bien son image : c'est le sommeil qui a tout retenu.
    on(&ws, &a, |_| ());
    let awake = send(&ws, json!({ "cmd": "carry_assets", "from": a, "to": b, "element": element }));
    assert_that!((asleep, awake)).is_equal_to((json!({ "carried": 0 }), json!({ "carried": 1 })));
}

#[test]
fn carry_to_an_unknown_tab_is_refused() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, _, element) = two_tabs(&ws);
    let req = serde_json::from_value(json!({ "cmd": "carry_assets", "from": a, "to": "t-99", "element": element }));
    let req: WorkspaceRequest = req.unwrap_or_else(|e| panic!("requête invalide : {e}"));
    let e = Workspace::apply(&ws, req).err().map(|e| e.kind());
    assert_that!(e).is_equal_to(Some(SessionErrorKind::UnknownTab));
}
