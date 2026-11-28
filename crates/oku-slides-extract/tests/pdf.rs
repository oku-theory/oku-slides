//! PDFs: their text page by page, their images within limits, and the ones refused (no text
//! layer, encrypted, damaged) or cut (pages, hostile images).

#![expect(clippy::panic, reason = "test helpers fail loudly on a broken fixture")]

mod common;

use std::path::Path;

use common::{
    PageSpec, extract, limits, pdf_encrypted, pdf_form_cycle, pdf_hand_written, pdf_kids_cycle, pdf_objstm_bomb,
    pdf_pages, pdf_parent_cycle, pdf_prev_loop, pdf_shared_form, pdf_shared_subtree, pdf_titled, pdf_xref_stream_bomb,
    read,
};
use oku_slides_core::knowledge::{CutKind, DocumentType, ExtractRefusal};
use oku_slides_core::knowledge::{Extracted, MAX_TITLE_CHARS};
use oku_slides_import::ReadLimits;
use speculoos::prelude::*;

const TOKEN: &str = "0123456789abcdef";

fn extracted(bytes: &[u8]) -> Extracted {
    extract(bytes, DocumentType::Pdf, &limits(), &read(), TOKEN).unwrap_or_else(|e| panic!("extract: {e:?}"))
}

fn refusal(bytes: &[u8]) -> Option<ExtractRefusal> {
    extract(bytes, DocumentType::Pdf, &limits(), &read(), TOKEN).err()
}

fn cut(out: &Extracted, what: CutKind) -> Option<u64> {
    out.cuts.iter().find(|c| c.what == what).map(|c| c.count)
}

#[test]
fn pdf_text_pages_as_page_sections() {
    let out = extracted(&pdf_pages(&[PageSpec::Text("First page words"), PageSpec::Text("# Second *page*")]));
    let md = out.markdown.as_str();
    assert_that!(md).contains("## Page 1\n\nFirst page words");
    assert_that!(md).contains("## Page 2\n\n\\# Second \\*page\\*");
    assert_that!(md.find("## Page 1")).is_less_than(md.find("## Page 2"));
    assert_that!(out.cuts.is_empty()).is_true();
    assert_that!(out.images.is_empty()).is_true();
}

#[test]
fn pdf_image_only_refused_no_text_layer() {
    let bytes = pdf_pages(&[PageSpec::Jpeg, PageSpec::Jpeg]);
    assert_that!(refusal(&bytes)).is_equal_to(Some(ExtractRefusal::NoTextLayer));
}

#[test]
fn pdf_mixed_pages_states_blank_page() {
    let out = extracted(&pdf_pages(&[PageSpec::Text("Alpha"), PageSpec::Jpeg, PageSpec::Text("Gamma")]));
    let md = out.markdown.as_str();
    assert_that!(md).contains("## Page 1\n\nAlpha");
    assert_that!(md).contains("## Page 2\n\n*(no text on this page)*");
    assert_that!(md).contains("## Page 3\n\nGamma");
    assert_that!(md).contains(format!("(oku-image-{TOKEN}-0)").as_str());
}

#[test]
fn pdf_dct_image_kept_as_jpeg() {
    let out = extracted(&pdf_pages(&[PageSpec::Text("Photo"), PageSpec::Jpeg]));
    assert_that!(out.images.len()).is_equal_to(1);
    assert_that!(out.images[0].mime.as_str()).is_equal_to("image/jpeg");
    assert_that!(out.images[0].bytes).is_equal_to(common::jpeg(4, 3));
    assert_that!(out.markdown.as_str()).contains(format!("](oku-image-{TOKEN}-0)").as_str());
}

#[test]
fn pdf_flate_rgb_image_as_png() {
    let out = extracted(&pdf_pages(&[PageSpec::Text("Chart"), PageSpec::FlateRgb { w: 5, h: 4 }]));
    assert_that!(out.images.len()).is_equal_to(1);
    let image = &out.images[0];
    assert_that!(image.mime.as_str()).is_equal_to("image/png");
    let decoder = png::Decoder::new(std::io::Cursor::new(image.bytes.as_slice()));
    let mut reader = decoder.read_info().unwrap_or_else(|e| panic!("png: {e}"));
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_else(|| panic!("png size"))];
    let info = reader.next_frame(&mut pixels).unwrap_or_else(|e| panic!("png frame: {e}"));
    assert_that!((info.width, info.height)).is_equal_to((5, 4));
    assert_that!(info.color_type).is_equal_to(png::ColorType::Rgb);
    let expected: Vec<u8> = (0..60).map(common::pixel).collect();
    assert_that!(pixels[..info.buffer_size()].to_vec()).is_equal_to(expected);
}

#[test]
fn pdf_flate_bomb_listed_not_imported() {
    let bytes = pdf_pages(&[PageSpec::Text("Bomb ahead"), PageSpec::FlateBomb]);
    assert_that!(bytes.len()).is_less_than(1 << 20);
    let out = extracted(&bytes);
    assert_that!(out.images.is_empty()).is_true();
    assert_that!(cut(&out, CutKind::Images)).is_equal_to(Some(1));
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported");
    assert_that!(out.markdown.len()).is_less_than(4096);
}

#[test]
fn pdf_header_dimensions_overflow_not_imported() {
    let huge = 1_i64 << 40;
    let bytes = pdf_pages(&[PageSpec::Text("Huge"), PageSpec::FlateRgb { w: huge, h: huge }]);
    let out = extracted(&bytes);
    assert_that!(out.images.is_empty()).is_true();
    assert_that!(cut(&out, CutKind::Images)).is_equal_to(Some(1));
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (over");
}

#[test]
fn pdf_kids_cycle_damaged() {
    assert_that!(refusal(&pdf_kids_cycle())).is_equal_to(Some(ExtractRefusal::Damaged));
}

#[test]
fn pdf_shared_subtree_damaged() {
    assert_that!(refusal(&pdf_shared_subtree())).is_equal_to(Some(ExtractRefusal::Damaged));
}

#[test]
fn pdf_parent_cycle_damaged() {
    assert_that!(refusal(&pdf_parent_cycle())).is_equal_to(Some(ExtractRefusal::Damaged));
}

#[test]
fn pdf_xref_prev_loop_read_once() {
    let out = extracted(&pdf_prev_loop());
    assert_that!(out.markdown.matches("Prev loop").count()).is_equal_to(1);
}

#[test]
fn pdf_object_stream_bomb_at_load_bounded() {
    let bytes = pdf_objstm_bomb();
    assert_that!(bytes.len()).is_less_than(1 << 20);
    let out = extracted(&bytes);
    assert_that!(out.markdown.as_str()).contains("Beside a bomb");
}

#[test]
fn pdf_xref_stream_bomb_at_load_too_large() {
    let bytes = pdf_xref_stream_bomb();
    assert_that!(bytes.len()).is_less_than(1 << 20);
    assert_that!(refusal(&bytes)).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn pdf_text_only_in_form_extracted() {
    let out = extracted(&pdf_pages(&[PageSpec::Forms { depth: 1, text: "Drawn by a form" }]));
    assert_that!(out.markdown.as_str()).contains("## Page 1\n\nDrawn by a form");
}

#[test]
fn pdf_text_in_nested_form_extracted() {
    let out = extracted(&pdf_pages(&[PageSpec::Forms { depth: 3, text: "Three forms deep" }]));
    assert_that!(out.markdown.as_str()).contains("Three forms deep");
    assert_that!(out.markdown.as_str()).does_not_contain("forms not read");
}

#[test]
fn pdf_form_cycle_read_once_stated() {
    let out = extracted(&pdf_form_cycle());
    assert_that!(out.markdown.matches("Looping form").count()).is_equal_to(1);
    assert_that!(out.markdown.as_str()).contains("1 embedded forms not read");
}

#[test]
fn pdf_forms_too_deep_refused_unsupported() {
    let bytes = pdf_pages(&[PageSpec::Forms { depth: 10, text: "Too deep" }]);
    assert_that!(refusal(&bytes)).is_equal_to(Some(ExtractRefusal::Unsupported));
    let out = extracted(&pdf_pages(&[PageSpec::Text("Shallow"), PageSpec::Forms { depth: 10, text: "Too deep" }]));
    assert_that!(out.markdown.as_str()).does_not_contain("Too deep");
    assert_that!(out.markdown.as_str()).contains("1 embedded forms not read");
}

#[test]
fn pdf_inline_image_stated() {
    let out = extracted(&pdf_pages(&[PageSpec::InlineImage("Has an inline image")]));
    assert_that!(out.markdown.as_str()).contains("Has an inline image");
    assert_that!(out.markdown.as_str()).contains("1 inline images");
}

#[test]
fn pdf_content_over_text_limit_without_text_too_large() {
    let read = ReadLimits { max_part_bytes: 1 << 20, ..read() };
    let bytes = pdf_pages(&[PageSpec::BigContent { bytes: 1 << 20 }]);
    let err = extract(&bytes, DocumentType::Pdf, &limits(), &read, TOKEN).err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn pdf_page_cap_without_text_too_large() {
    let mut pages = vec![PageSpec::Blank; 2000];
    pages.extend([PageSpec::Text("Past the cap"); 3]);
    assert_that!(refusal(&pdf_pages(&pages))).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn pdf_encrypted_refused_encrypted() {
    assert_that!(refusal(&pdf_encrypted())).is_equal_to(Some(ExtractRefusal::Encrypted));
}

#[test]
fn pdf_garbage_damaged() {
    assert_that!(refusal(b"%PDF-1.7\n\x00\xff garbage with no objects at all"))
        .is_equal_to(Some(ExtractRefusal::Damaged));
    let mut cut_short = pdf_pages(&[PageSpec::Text("Cut short")]);
    cut_short.truncate(40);
    assert_that!(refusal(&cut_short)).is_equal_to(Some(ExtractRefusal::Damaged));
}

#[test]
fn pdf_past_page_cap_cut_stated() {
    let pages = vec![PageSpec::Text("p"); 2003];
    let out = extracted(&pdf_pages(&pages));
    assert_that!(out.markdown.as_str()).contains("## Page 2000\n");
    assert_that!(out.markdown.as_str()).does_not_contain("## Page 2001");
    assert_that!(cut(&out, CutKind::Pages)).is_equal_to(Some(3));
    assert_that!(out.markdown.as_str()).contains("… 3 pages not imported (limit 2000)");
}

#[test]
fn pdf_broken_contents_refused_damaged() {
    let bytes = pdf_pages(&[PageSpec::BrokenContents, PageSpec::BrokenContents]);
    assert_that!(refusal(&bytes)).is_equal_to(Some(ExtractRefusal::Damaged));
}

#[test]
fn pdf_form_over_budget_without_text_too_large() {
    let read = ReadLimits { max_part_bytes: 1 << 20, ..read() };
    let bytes = pdf_pages(&[PageSpec::BigForm { bytes: 1 << 20 }]);
    let err = extract(&bytes, DocumentType::Pdf, &limits(), &read, TOKEN).err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn pdf_amplifying_to_unicode_capped_stated() {
    let read = ReadLimits { max_part_bytes: 1 << 20, ..read() };
    let bytes = pdf_pages(&[PageSpec::Amplified { bytes: 1000 }]);
    let out = extract(&bytes, DocumentType::Pdf, &limits(), &read, TOKEN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(out.markdown.matches('A').count()).is_less_than_or_equal_to((1 << 20) / 8);
    assert_that!(out.markdown.matches('A').count()).is_greater_than(1000);
    assert_that!(out.markdown.as_str()).contains("text of 1 pages past the read limits");
}

#[test]
fn pdf_missing_xobject_stated_as_not_resolving() {
    let out = extracted(&pdf_pages(&[PageSpec::BrokenXObject("Draws nothing")]));
    assert_that!(out.markdown.as_str()).contains("Draws nothing");
    assert_that!(out.markdown.as_str()).contains("1 drawn objects that do not resolve");
}

#[test]
fn pdf_missing_font_stated_as_unreadable() {
    let out = extracted(&pdf_pages(&[PageSpec::BrokenFont("Readable words")]));
    assert_that!(out.markdown.as_str()).contains("Readable words");
    assert_that!(out.markdown.as_str()).does_not_contain("Lost words");
    assert_that!(out.markdown.as_str()).contains("text of 1 unreadable pages");
}

#[test]
fn pdf_title_raw_capped() {
    let raw = "Q3\u{202E} Report\u{7}\n\tPlan";
    let out = extracted(&pdf_titled(&[PageSpec::Text("Body")], raw));
    assert_that!(out.title).is_equal_to(Some(raw.to_owned()));
    let long = "x".repeat(MAX_TITLE_CHARS + 300);
    let out = extracted(&pdf_titled(&[PageSpec::Text("Body")], &long));
    assert_that!(out.title.map(|t| t.chars().count())).is_equal_to(Some(MAX_TITLE_CHARS));
}

#[test]
fn alto_wealth_pdf_has_known_word() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import/alto-wealth.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert_that!(bytes.starts_with(b"version https://git-lfs")).named("an LFS pointer, not the PDF").is_false();
    let out = extracted(&bytes);
    assert_that!(out.markdown.as_str()).contains("## Page 1\n");
    assert_that!(out.markdown.as_str()).contains("Alto");
}

/// t-1760: lines placed by `Td`, `TD` or `Tm` inside one text object stay separate lines,
/// never glued into one word.
#[test]
fn pdf_td_lines_in_one_block_are_separate() {
    let content = "BT /F1 12 Tf 72 720 Td (First line) Tj 0 -14 Td (Second line) Tj 0 -14 TD (Third line) Tj ET";
    let out = extracted(&pdf_pages(&[PageSpec::Raw(content)]));
    assert_that!(out.markdown.as_str()).contains("First line\nSecond line\nThird line");
}

#[test]
fn pdf_tm_lines_in_one_block_are_separate_and_same_line_spaced() {
    let content = "BT /F1 12 Tf 1 0 0 1 72 720 Tm (Top) Tj 1 0 0 1 72 700 Tm (Below) Tj \
                   1 0 0 1 200 700 Tm (right) Tj 120 0 Td (further) Tj ET";
    let out = extracted(&pdf_pages(&[PageSpec::Raw(content)]));
    assert_that!(out.markdown.as_str()).contains("Top\nBelow right further");
}

/// t-1760: a form shared by every page is decoded once, not once per page: read whole
/// within a total read budget of a few decodes.
#[test]
fn pdf_shared_form_on_many_pages_decoded_once() {
    let read = ReadLimits { max_total_bytes: 1 << 20, ..read() };
    let bytes = pdf_shared_form(40, 128 << 10, "Shared form");
    let out = extract(&bytes, DocumentType::Pdf, &limits(), &read, TOKEN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(out.markdown.matches("Shared form").count()).is_equal_to(40);
    assert_that!(out.cuts.is_empty()).is_true();
}

/// t-1760: once the total read budget is spent, the pages left are cut and stated.
#[test]
fn pdf_stops_walking_when_read_total_spent() {
    let read = ReadLimits { max_total_bytes: 1 << 20, ..read() };
    let mut pages = vec![PageSpec::Text("First page")];
    pages.extend([PageSpec::BigContent { bytes: 256 << 10 }; 10]);
    let out =
        extract(&pdf_pages(&pages), DocumentType::Pdf, &limits(), &read, TOKEN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(out.markdown.as_str()).contains("First page");
    let left = cut(&out, CutKind::Pages).unwrap_or_else(|| panic!("no pages cut: {}", out.markdown));
    assert_that!(left).is_greater_than_or_equal_to(6);
    assert_that!(out.markdown.as_str()).contains(format!("… {left} pages not imported (past the read limits)"));
}

/// t-1760: once the text budget is spent, the pages left are not walked but cut and stated.
#[test]
fn pdf_stops_walking_when_budget_spent() {
    let limits = oku_slides_core::knowledge::ExtractLimits { max_text_bytes: 400, ..limits() };
    let pages = vec![PageSpec::Text("A line of text on this page"); 50];
    let out =
        extract(&pdf_pages(&pages), DocumentType::Pdf, &limits, &read(), TOKEN).unwrap_or_else(|e| panic!("{e:?}"));
    let left = cut(&out, CutKind::Pages).unwrap_or_else(|| panic!("no pages cut: {}", out.markdown));
    assert_that!(left).is_greater_than(30);
    assert_that!(out.markdown.as_str()).contains(format!("… {left} pages not imported (text budget spent)"));
}

/// N2 (t-1760): fonts decode within the document's total read budget: many distinct fonts
/// with large `ToUnicode` maps stop the walk with a stated cut instead of growing memory.
#[test]
fn pdf_many_large_fonts_stop_at_read_total_stated() {
    let read = ReadLimits { max_part_bytes: 8 << 20, max_total_bytes: 1 << 20, ..read() };
    let pages = vec![PageSpec::BigFont { bytes: 256 << 10 }; 20];
    let out =
        extract(&pdf_pages(&pages), DocumentType::Pdf, &limits(), &read, TOKEN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(out.markdown.as_str()).contains("Big font words");
    let left = cut(&out, CutKind::Pages).unwrap_or_else(|| panic!("no pages cut: {}", out.markdown));
    assert_that!(left).is_greater_than_or_equal_to(15);
    assert_that!(out.markdown.as_str()).contains(format!("… {left} pages not imported (past the read limits)"));
}

/// The asset id of the PDF `ui/e2e/460-knowledge-tab.cjs` attaches, pinned there too.
const E2E_PDF_ID: &str = "sha256:a67ffa35c1576965fc27e699bfa36e41f7ec6b6fcf31c7e585ac2fe0a23105de";

/// t-1761: the PDF the knowledge tab e2e attaches, written like `ui/e2e/lib/pdf.cjs` writes
/// it, extracts to its one line of text.
#[test]
fn pdf_hand_written_page_extracts_its_text() {
    let bytes = pdf_hand_written("Knowledge e2e marker");
    // The e2e pins the same hash: both sides write the same bytes.
    assert_that!(oku_slides_core::asset_id(&bytes).as_str()).is_equal_to(E2E_PDF_ID);
    let out = extracted(&bytes);
    assert_that!(out.markdown.as_str()).contains("## Page 1\n\nKnowledge e2e marker");
    assert_that!(out.cuts.is_empty()).is_true();
}
