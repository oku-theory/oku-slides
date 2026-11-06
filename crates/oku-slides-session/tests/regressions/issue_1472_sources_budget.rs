//! t-1472: in-app additions could grow a deck's sources past `[sources] max_sources_mib`,
//! so the saved file no longer reopened. A pasted image past the budget is now refused.

use std::io::Cursor;

use oku_slides_session::{SessionErrorKind, Settings};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{fixture_path, req, session_with_settings};

/// A grey PNG of about 1.2 MiB that deflate cannot shrink.
fn heavy_png() -> Vec<u8> {
    let (w, h) = (1100_u32, 1100_u32);
    let mut state = 0x9e37_79b9_u32;
    let pixels: Vec<u8> = (0..w * h)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()[0]
        })
        .collect();
    let mut out = Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, w, h);
    e.set_color(png::ColorType::Grayscale);
    let mut writer = e.write_header().unwrap_or_else(|e| panic!("png : {e}"));
    writer.write_image_data(&pixels).unwrap_or_else(|e| panic!("png : {e}"));
    drop(writer);
    out.into_inner()
}

#[test]
fn issue_1472_pasted_image_past_budget_is_refused() {
    let data = crate::common::data_dir();
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.sources.max_sources_mib = 1;
    let mut s = session_with_settings(&data, settings);
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
    let pasted = s.paste_image(heavy_png(), &[]).unwrap_or_else(|e| panic!("collage : {}", e.one_line()));
    let before = s.deck().clone();
    let apply = req(json!({ "cmd": "apply", "label": "image", "ops": [{
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" },
        "param": "asset", "value": pasted.id
    }]}));
    let e = s.handle(apply).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::SourcesOverBudget);
    assert_that!(s.deck()).is_equal_to(&before);
}
