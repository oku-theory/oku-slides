//! t-1182 : l'élément copié n'existait que dans la fenêtre où il l'avait été ;
//! coller dans une autre fenêtre ne collait que son texte. L'hôte le garde
//! (`SetClip`, relu par `Clip`) et le diffuse ; le collage apporte ses images
//! comme entre deux onglets (`CarryAssets`, t-1178).

use std::sync::Mutex;

use oku_slides_core::Element;
use oku_slides_session::Workspace;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{data_dir, fixture_path};
use crate::workspace_support::{on, op, png, send, workspace};

/// L'onglet de `w-1` sur roadmap.json, `e-image` citant une image du bundle, et un
/// onglet de `w-2` sur le même fichier, sans cette image. Les deux onglets et l'image.
fn two_windows(ws: &Mutex<Workspace>) -> (String, String, Element) {
    let a = send(ws, json!({ "cmd": "view" }))["windows"][0]["active"].as_str().map(str::to_owned);
    let a = a.unwrap_or_else(|| panic!("aucun onglet"));
    let view = send(ws, json!({ "cmd": "new_window" }));
    let b = view["windows"].as_array().and_then(|w| w.iter().find(|w| w["label"] != "w-1"));
    let b = b.and_then(|w| w["active"].as_str()).map(str::to_owned).unwrap_or_else(|| panic!("pas de w-2"));
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
        slide.locate("e-image").map(|l| l.element.clone()).unwrap_or_else(|| panic!("e-image absente"))
    });
    (a, b, element)
}

/// L'élément copié dans `tab`, tel que la fenêtre `origin` l'envoie.
fn clip(tab: &str, element: &Element, origin: &str) -> serde_json::Value {
    json!({ "tab": tab, "text": "base/image", "element": element, "origin": origin })
}

#[test]
fn clip_without_a_copy_is_null() {
    let data = data_dir();
    let ws = workspace(&data);
    assert_that!(send(&ws, json!({ "cmd": "clip" }))).is_equal_to(json!({ "clip": null, "seq": 0 }));
}

#[test]
fn set_clip_replies_with_the_clip_and_its_number() {
    let data = data_dir();
    let ws = workspace(&data);
    let element = json!({ "id": "e-1", "kind": "base/text" });
    let copied = json!({ "tab": "t-1", "text": "Bonjour", "element": element, "origin": "w-1" });
    let reply = send(&ws, json!({ "cmd": "set_clip", "clip": copied }));
    assert_that!(reply).is_equal_to(json!({ "clip": copied, "seq": 1 }));
}

#[test]
fn clip_after_two_set_clips_returns_the_last_with_its_number() {
    let data = data_dir();
    let ws = workspace(&data);
    let first = json!({ "tab": "t-1", "text": "Un", "element": { "id": "e-1", "kind": "base/text" }, "origin": "w-1" });
    let last =
        json!({ "tab": "t-2", "text": "Deux", "element": { "id": "e-2", "kind": "base/text" }, "origin": "w-2" });
    send(&ws, json!({ "cmd": "set_clip", "clip": first }));
    send(&ws, json!({ "cmd": "set_clip", "clip": last }));
    assert_that!(send(&ws, json!({ "cmd": "clip" }))).is_equal_to(json!({ "clip": last, "seq": 2 }));
}

#[test]
fn clip_after_resume_is_null() {
    let data = data_dir();
    {
        let ws = workspace(&data);
        let copied =
            json!({ "tab": "t-1", "text": "Un", "element": { "id": "e-1", "kind": "base/text" }, "origin": "w-1" });
        send(&ws, json!({ "cmd": "set_clip", "clip": copied }));
        Workspace::settle(&ws);
    }
    // Jamais enregistré : une reprise repart sans élément copié.
    let ws = workspace(&data);
    assert_that!(send(&ws, json!({ "cmd": "clip" }))).is_equal_to(json!({ "clip": null, "seq": 0 }));
}

#[test]
fn paste_into_another_window_brings_the_image_bytes() {
    let data = data_dir();
    let ws = workspace(&data);
    let (a, b, element) = two_windows(&ws);
    send(&ws, json!({ "cmd": "set_clip", "clip": clip(&a, &element, "w-1") }));
    // La fenêtre w-2 relit l'élément copié, puis colle dans son onglet.
    let copied = send(&ws, json!({ "cmd": "clip" }))["clip"].clone();
    let carried =
        send(&ws, json!({ "cmd": "carry_assets", "from": copied["tab"], "to": b, "element": copied["element"] }));
    assert_that!(carried).is_equal_to(json!({ "carried": 1 }));
    // L'élément collé est celui que l'hôte a gardé : son asset a survécu à l'aller-retour.
    let mut copy: Element =
        serde_json::from_value(copied["element"].clone()).unwrap_or_else(|e| panic!("élément : {e}"));
    copy.id = "e-image-copie".into();
    let slot = json!({ "owner": { "slide": "s-libre" }, "param": "elements" });
    let paste = op(json!({ "op": "add_element", "slot": slot, "element": copy.clone() }));
    on(&ws, &b, |s| s.apply("coller", vec![paste], None))
        .unwrap_or_else(|e| panic!("collage refusé : {}", e.one_line()));
    assert_that!(on(&ws, &b, |s| s.element_assets(&copy))).is_equal_to(vec![png()]);
}
