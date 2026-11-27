use speculoos::prelude::*;

use super::*;
use oku_slides_core::knowledge::CutKind;

fn limits(max_images: usize, max_image_bytes: u64) -> ExtractLimits {
    ExtractLimits {
        max_text_bytes: 1 << 20,
        sheet_rows: 10,
        sheet_columns: 10,
        max_images,
        max_image_pixels: 1 << 20,
        max_image_bytes,
    }
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b
}

fn none(_: &str, _: &str) -> Option<String> {
    None
}

#[test]
fn inline_render_emphasis_wraps_text_without_its_spaces() {
    let mut i = Inline::new(100);
    i.text("Rose by ", false, false, None);
    i.text("twelve ", true, false, None);
    i.text("percent", false, true, None);
    assert_that!(i.render(false, &mut none)).is_equal_to("Rose by **twelve** *percent*".to_owned());
}

#[test]
fn inline_render_line_starts_escaped_and_breaks_kept() {
    let mut i = Inline::new(100);
    i.text("# not a heading", false, false, None);
    i.line_break();
    i.text("- not an item", false, false, None);
    assert_that!(i.render(false, &mut none)).is_equal_to("\\# not a heading\n\\- not an item".to_owned());
    assert_that!(i.render(true, &mut none)).is_equal_to("# not a heading - not an item".to_owned());
}

#[test]
fn inline_link_unsafe_scheme_becomes_text() {
    let mut i = Inline::new(100);
    let web = i.link("https://e.org/a b");
    let js = i.link("javascript:alert(1)");
    i.text("site", false, false, Some(web));
    i.text(" and ", false, false, None);
    i.text("click", false, false, Some(js));
    assert_that!(i.render(false, &mut none)).is_equal_to("[site](https://e.org/a%20b) and click".to_owned());
}

#[test]
fn inline_text_past_cap_counted_not_kept() {
    let mut i = Inline::new(5);
    i.text("abc", false, false, None);
    i.text("defgh", false, false, None);
    assert_that!(i.plain()).is_equal_to("abc".to_owned());
    assert_that!(i.dropped()).is_equal_to(5);
    assert_that!(i.read_bytes()).is_equal_to(8);
    assert_that!(i.is_blank()).is_false();
}

/// t-1760: a run past the cap ends what the paragraph keeps; a later shorter run would leave
/// a hole mid-paragraph.
#[test]
fn inline_after_refused_run_drops_the_rest() {
    let mut i = Inline::new(5);
    i.text("abc", false, false, None);
    i.text("defgh", false, false, None);
    i.text("i", false, false, None);
    assert_that!(i.plain()).is_equal_to("abc".to_owned());
    assert_that!(i.dropped()).is_equal_to(6);
}

#[test]
fn writer_block_items_of_one_kind_joined_by_a_line_end() {
    let mut w = Writer::new(100);
    assert_that!(w.block("1. a", Join::Item { ordered: true, level: 0 })).is_true();
    assert_that!(w.block("1. b", Join::Item { ordered: true, level: 0 })).is_true();
    assert_that!(w.block("- c", Join::Item { ordered: false, level: 0 })).is_true();
    assert_that!(w.block("text", Join::Block)).is_true();
    assert_that!(w.finish().0).is_equal_to("1. a\n1. b\n\n- c\n\ntext\n".to_owned());
}

#[test]
fn writer_block_past_budget_refused_then_every_later_one() {
    let mut w = Writer::new(10);
    assert_that!(w.block("12345678", Join::Block)).is_true();
    assert_that!(w.block("abc", Join::Block)).is_false();
    assert_that!(w.block("x", Join::Block)).is_false();
    assert_that!(w.room()).is_equal_to(0);
}

#[test]
fn doc_paragraph_past_budget_counted_and_its_image_listed() {
    let l = ExtractLimits { max_text_bytes: 8, ..limits(5, 1 << 20) };
    let mut doc = Doc::new(&l, 1 << 20, "tok");
    let bytes = png(2, 2);
    let fetch = |_: &str| -> Result<Cow<'_, [u8]>, Skip> { Ok(Cow::Borrowed(bytes.as_slice())) };
    let mut first = Inline::new(doc.room());
    first.text("short", false, false, None);
    doc.paragraph(&first, Kind::Para, &fetch);
    let mut second = Inline::new(doc.room());
    second.text("x", false, false, None);
    second.image("a", "pic");
    doc.paragraph(&second, Kind::Para, &fetch);
    let out = doc
        .finish(
            Cut { what: CutKind::Depth, count: 0 },
            Bound::TooDeep,
            &[],
            &ExtractLimits { max_text_bytes: 1000, ..l },
        )
        .unwrap();
    assert_that!(out.images.is_empty()).is_true();
    assert_that!(out.markdown.as_str()).starts_with("short\n");
    assert_that!(out.markdown.as_str()).contains("(text budget spent)");
    assert_that!(out.cuts.iter().any(|c| c.what == CutKind::Text && c.count == 1)).is_true();
}

#[test]
fn doc_paragraph_heading_and_items_marked() {
    let l = limits(0, 0);
    let mut doc = Doc::new(&l, 0, "tok");
    let fetch = |_: &str| -> Result<Cow<'_, [u8]>, Skip> { Err(Skip::Unreadable) };
    let line = |t: &str| {
        let mut i = Inline::new(100);
        i.text(t, false, false, None);
        i
    };
    doc.paragraph(&line("Top #1"), Kind::Heading(2), &fetch);
    doc.paragraph(&line("one"), Kind::Item { ordered: true, level: 0 }, &fetch);
    doc.paragraph(&line("sub"), Kind::Item { ordered: true, level: 1 }, &fetch);
    let out = doc.finish(Cut { what: CutKind::Depth, count: 0 }, Bound::TooDeep, &[], &l).unwrap();
    assert_that!(out.markdown).is_equal_to("## Top \\#1\n\n1. one\n    1. sub\n".to_owned());
}

#[test]
fn inline_render_number_split_by_unkept_link_escaped_at_line_start() {
    let mut i = Inline::new(100);
    let js = i.link("javascript:x");
    i.text("1", false, false, None);
    i.text(". x", false, false, Some(js));
    assert_that!(i.render(false, &mut none)).is_equal_to("1\\. x".to_owned());
}

#[test]
fn inline_render_mark_after_break_and_spaces_escaped() {
    let mut i = Inline::new(100);
    i.text("a", false, false, None);
    i.line_break();
    i.text("  ", false, false, None);
    i.text("+ b", true, false, None);
    assert_that!(i.render(false, &mut none)).is_equal_to("a\n**+ b**".to_owned());
}

#[test]
fn doc_paragraph_nested_item_after_paragraph_written_at_level_zero() {
    let l = limits(0, 0);
    let mut doc = Doc::new(&l, 0, "tok");
    let fetch = |_: &str| -> Result<Cow<'_, [u8]>, Skip> { Err(Skip::Unreadable) };
    let line = |t: &str| {
        let mut i = Inline::new(100);
        i.text(t, false, false, None);
        i
    };
    doc.paragraph(&line("intro"), Kind::Para, &fetch);
    doc.paragraph(&line("deep"), Kind::Item { ordered: false, level: 3 }, &fetch);
    doc.paragraph(&line("deeper"), Kind::Item { ordered: false, level: 5 }, &fetch);
    let out = doc.finish(Cut { what: CutKind::Depth, count: 0 }, Bound::TooDeep, &[], &l).unwrap();
    assert_that!(out.markdown).is_equal_to("intro\n\n- deep\n    - deeper\n".to_owned());
}

#[test]
fn doc_paragraph_after_budget_spent_image_not_fetched() {
    let l = ExtractLimits { max_text_bytes: 4, ..limits(5, 1 << 20) };
    let mut doc = Doc::new(&l, 1 << 20, "tok");
    let fetch = |_: &str| -> Result<Cow<'_, [u8]>, Skip> { panic!("fetched past the budget") };
    let mut first = Inline::new(doc.room());
    first.text("abcd", false, false, None);
    doc.paragraph(&first, Kind::Para, &fetch);
    let mut second = Inline::new(doc.room());
    second.image("a", "pic");
    doc.paragraph(&second, Kind::Para, &fetch);
    let out = doc
        .finish(
            Cut { what: CutKind::Depth, count: 0 },
            Bound::TooDeep,
            &[],
            &ExtractLimits { max_text_bytes: 1000, ..l },
        )
        .unwrap();
    assert_that!(out.markdown.as_str()).contains("… 1 images not imported (text budget spent)");
}

#[test]
fn doc_finish_depth_bound_stated() {
    let l = limits(0, 0);
    let doc = Doc::new(&l, 0, "tok");
    let out = doc.finish(Cut { what: CutKind::Depth, count: 3 }, Bound::TooDeep, &[], &l).unwrap();
    assert_that!(out.markdown).is_equal_to("… 3 nested elements not imported (too deep)\n".to_owned());
    assert_that!(out.cuts).is_equal_to(vec![Cut { what: CutKind::Depth, count: 3 }]);
}
