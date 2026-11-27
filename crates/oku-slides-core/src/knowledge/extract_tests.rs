use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{Cut, CutKind, DocumentType, ExtractLimits, ExtractRefusal};

/// Every refusal, for the locale tests; a new variant fails the match in `refusal_at` until added.
const ALL_REFUSALS: [ExtractRefusal; 7] = [
    ExtractRefusal::NoTextLayer,
    ExtractRefusal::Legacy,
    ExtractRefusal::Encrypted,
    ExtractRefusal::Unsupported,
    ExtractRefusal::Damaged,
    ExtractRefusal::TypeMismatch,
    ExtractRefusal::TooLarge,
];

/// The index of `refusal` in [`ALL_REFUSALS`]: exhaustive, so a new variant does not compile.
fn refusal_at(refusal: ExtractRefusal) -> usize {
    match refusal {
        ExtractRefusal::NoTextLayer => 0,
        ExtractRefusal::Legacy => 1,
        ExtractRefusal::Encrypted => 2,
        ExtractRefusal::Unsupported => 3,
        ExtractRefusal::Damaged => 4,
        ExtractRefusal::TypeMismatch => 5,
        ExtractRefusal::TooLarge => 6,
    }
}

/// t-1760: the test list holds every refusal once, in order.
#[test]
fn all_refusals_lists_every_refusal_once() {
    for (i, refusal) in ALL_REFUSALS.into_iter().enumerate() {
        assert_that!(refusal_at(refusal)).is_equal_to(i);
    }
}

/// Each refusal says its English text through its key, and French differs from it.
#[test]
fn extract_refusal_every_key_in_en_and_fr() {
    let en = Localizer::new(Some("en"));
    let fr = Localizer::new(Some("fr"));
    for refusal in ALL_REFUSALS {
        assert_that!(en.tr(refusal.key())).named(refusal.key()).is_equal_to(refusal.to_string());
        assert_that!(fr.tr(refusal.key())).named(refusal.key()).is_not_equal_to(refusal.to_string());
    }
}

#[test]
fn extract_refusal_keys_in_all_locales() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for refusal in ALL_REFUSALS {
            assert!(l.has(refusal.key()), "{locale}: {} missing", refusal.key());
        }
    }
}

#[test]
fn extract_refusal_serializes_as_snake_case() {
    let json = serde_json::to_string(&ExtractRefusal::TypeMismatch).unwrap();
    assert_that!(json).is_equal_to("\"type_mismatch\"".to_owned());
}

#[test]
fn document_type_label_names_the_format() {
    assert_that!((DocumentType::Xlsx.label(), DocumentType::Markdown.label())).is_equal_to(("XLSX", "Markdown"));
}

#[test]
fn extract_limits_and_cut_round_trip_through_json() {
    let limits = ExtractLimits {
        max_text_bytes: 10,
        sheet_rows: 2000,
        sheet_columns: 50,
        max_images: 200,
        max_image_pixels: 1 << 26,
        max_image_bytes: 128 << 20,
    };
    let back: ExtractLimits = serde_json::from_str(&serde_json::to_string(&limits).unwrap()).unwrap();
    assert_that!(back).is_equal_to(limits);
    let cut = Cut { what: CutKind::Rows, count: 3 };
    let back: Cut = serde_json::from_str(&serde_json::to_string(&cut).unwrap()).unwrap();
    assert_that!(back).is_equal_to(cut);
}

/// t-1760: `CutKind::ALL` lists every kind once; a new kind fails this match until added.
#[test]
fn cut_kind_all_lists_every_kind_once() {
    let at = |kind: CutKind| match kind {
        CutKind::Rows => 0,
        CutKind::Columns => 1,
        CutKind::Sheets => 2,
        CutKind::Pages => 3,
        CutKind::Text => 4,
        CutKind::Images => 5,
        CutKind::Depth => 6,
    };
    for (i, kind) in CutKind::ALL.into_iter().enumerate() {
        assert_that!(at(kind)).is_equal_to(i);
    }
}
