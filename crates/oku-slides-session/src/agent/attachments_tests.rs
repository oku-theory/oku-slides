use std::io::Cursor;

use oku_slides_core::Op;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::SessionErrorKind;
use crate::test_support::session;

/// Un PNG valide de `w` × `h` pixels gris, de contenu propre à `seed`.
pub(crate) fn png(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, w, h);
    e.set_color(png::ColorType::Grayscale);
    let mut writer = e.write_header().unwrap();
    let pixels: Vec<u8> = (0..w * h).map(|i| (i as u8).wrapping_add(seed)).collect();
    writer.write_image_data(&pixels).unwrap();
    drop(writer);
    out.into_inner()
}

#[test]
fn paste_image_png_returns_its_asset_id_type_and_size() {
    let mut s = session();
    let bytes = png(3, 2, 0);
    let r = s.paste_image(bytes.clone(), &[]).unwrap();
    assert_that!((r.id, r.mime.as_str(), r.width, r.height)).is_equal_to((
        oku_slides_core::asset_id(&bytes),
        "image/png",
        3,
        2,
    ));
}

#[test]
fn paste_image_twice_keeps_one_copy() {
    let mut s = session();
    let bytes = png(2, 2, 1);
    let a = s.paste_image(bytes.clone(), &[]).unwrap();
    let b = s.paste_image(bytes, &[]).unwrap();
    assert_that!(a).is_equal_to(b);
}

#[test]
fn pasted_image_gives_back_the_bytes() {
    let mut s = session();
    let bytes = png(2, 2, 2);
    let r = s.paste_image(bytes.clone(), &[]).unwrap();
    let (_, kept) = s.pasted_image(&r.id).unwrap();
    assert_that!(kept.to_vec()).is_equal_to(bytes);
}

#[rstest]
#[case::svg(b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(), "session-paste-unsupported")]
#[case::text(b"bonjour".to_vec(), "session-paste-unsupported")]
#[case::bmp(b"BM\x36\0\0\0\0\0\0\0".to_vec(), "session-paste-unsupported")]
#[case::truncated_png(b"\x89PNG\r\n\x1a\n\0\0".to_vec(), "session-paste-unreadable")]
fn paste_image_refused_bytes_say_why(#[case] bytes: Vec<u8>, #[case] key: &str) {
    let mut s = session();
    let e = s.paste_image(bytes, &[]).unwrap_err();
    assert_that!((e.kind(), e.paste_reason_key())).is_equal_to((SessionErrorKind::Paste, Some(key)));
}

#[test]
fn paste_image_over_the_byte_limit_is_too_large() {
    let (_data, settings) = paste_settings(|p| p.max_image_mib = 0);
    let mut s = crate::test_support::session_with(settings);
    let e = s.paste_image(png(2, 2, 3), &[]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-paste-too-large"));
}

#[test]
fn paste_image_over_the_pixel_limit_has_too_many_pixels() {
    let (_data, settings) = paste_settings(|p| p.max_image_pixels = 3);
    let mut s = crate::test_support::session_with(settings);
    let e = s.paste_image(png(2, 2, 4), &[]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-paste-too-many-pixels"));
}

#[test]
fn paste_image_past_the_kept_limit_forgets_the_oldest_outside_keep() {
    let (_data, settings) = paste_settings(|p| p.max_kept_mib = 0);
    let mut s = crate::test_support::session_with(settings);
    let old = s.paste_image(png(2, 2, 5), &[]).unwrap();
    let kept = s.paste_image(png(2, 2, 6), &[]).unwrap();
    s.paste_image(png(2, 2, 7), std::slice::from_ref(&kept.id)).unwrap();
    assert_that!((s.pasted_image(&old.id).is_none(), s.pasted_image(&kept.id).is_some())).is_equal_to((true, true));
}

#[test]
fn paste_error_is_said_in_french_without_user_data() {
    let mut s = session();
    let e = s.paste_image(b"bonjour".to_vec(), &[]).unwrap_err();
    let fr = oku_slides_i18n::Localize::localize(&e, &oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(fr.contains("bonjour")).is_false();
    assert_that!(fr.as_str()).is_not_equal_to("session-paste-unsupported");
}

/// Des réglages de test dont `[paste]` est retouché par `edit`, et le dossier qu'ils visent (à garder).
fn paste_settings(
    edit: impl FnOnce(&mut oku_slides_platform::config::PasteSettings),
) -> (tempfile::TempDir, crate::Settings) {
    let data = crate::test_support::fresh_dir("oku-slides-paste-");
    let mut settings = crate::test_support::settings_on(data.path());
    edit(&mut settings.paste);
    (data, settings)
}

/// L'op qui donne `asset` à l'image `e-image` de `s-libre` (roadmap.json).
fn set_image(asset: &str) -> Op {
    serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "asset", "value": asset
    }))
    .unwrap()
}

#[test]
fn apply_citing_a_pasted_image_adds_it_to_the_bundle() {
    let mut s = session();
    let r = s.paste_image(png(4, 4, 8), &[]).unwrap();
    s.apply("image", vec![set_image(&r.id)], None).unwrap();
    assert_that!(s.doc.sources.assets.contains_key(&r.id)).is_true();
}

#[test]
fn paste_image_alone_adds_nothing_to_the_bundle() {
    let mut s = session();
    let r = s.paste_image(png(4, 4, 9), &[]).unwrap();
    assert_that!(s.doc.sources.assets.contains_key(&r.id)).is_false();
}

#[test]
fn apply_citing_an_unknown_asset_is_refused() {
    let mut s = session();
    let before: Vec<String> = s.doc.sources.assets.keys().cloned().collect();
    let e = s.apply("image", vec![set_image("sha256:1234")], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
    let after: Vec<String> = s.doc.sources.assets.keys().cloned().collect();
    assert_that!(after).is_equal_to(before);
}

#[test]
fn apply_keeps_an_asset_already_missing_before_the_op() {
    let mut s = session();
    let fit = serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "fit", "value": "contain"
    }))
    .unwrap();
    assert_that!(s.apply("fit", vec![fit], None)).is_ok();
}
