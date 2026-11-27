use speculoos::prelude::*;

use super::*;

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

/// t-1760: the first kept image whose placeholder is missing, found in one scan.
#[test]
fn first_uncited_finds_gap() {
    let l = limits(5, 1 << 20);
    let mut images = Images::new(&l, 1 << 20, "tok");
    for (key, side) in [("a", 2), ("b", 3), ("c", 4)] {
        let bytes = png(side, side);
        images.cite(key, "", || Ok(Cow::Borrowed(bytes.as_slice())));
    }
    assert_that!(images.first_uncited("![](oku-image-tok-0) ![](oku-image-tok-2)")).is_equal_to(Some(1));
    assert_that!(images.first_uncited("(oku-image-tok-2)(oku-image-tok-1)(oku-image-tok-0)")).is_none();
    assert_that!(images.first_uncited("(oku-image-tok-1)(oku-image-tok-10)(oku-image-tok-2")).is_equal_to(Some(0));
    assert_that!(images.first_uncited("(oku-image-tok-0)(oku-image-tok-1)(oku-image-tok-2x)")).is_equal_to(Some(2));
}

#[test]
fn images_cite_kept_once_and_placeholder_by_index() {
    let l = limits(5, 1 << 20);
    let mut images = Images::new(&l, 1 << 20, "tok");
    let bytes = png(2, 2);
    let first = images.cite("a", "x|y", || Ok(Cow::Borrowed(bytes.as_slice())));
    let again = images.cite("a", "z", || Err(Skip::Unreadable));
    assert_that!(first).is_equal_to(Some("![x\\|y](oku-image-tok-0)".to_owned()));
    assert_that!(again).is_equal_to(Some("![z](oku-image-tok-0)".to_owned()));
    let (notices, cut) = images.notices();
    assert_that!(images.into_kept().len()).is_equal_to(1);
    assert_that!(notices).is_equal_to(String::new());
    assert_that!(cut).is_none();
}

#[test]
fn images_cite_past_max_images_not_read() {
    let l = limits(1, 1 << 20);
    let mut images = Images::new(&l, 1 << 20, "tok");
    let bytes = png(2, 2);
    images.cite("a", "", || Ok(Cow::Borrowed(bytes.as_slice())));
    let second = images.cite("b", "", || panic!("read past max_images"));
    assert_that!(second).is_none();
    let (notices, cut) = images.notices();
    assert_that!(notices).is_equal_to("… 1 images not imported (limit 1)\n".to_owned());
    assert_that!(cut).is_equal_to(Some(Cut { what: CutKind::Images, count: 1 }));
}

#[test]
fn images_cite_past_image_bytes_listed() {
    let bytes = png(2, 2);
    let l = limits(5, count_u64(bytes.len()) + 1);
    let mut images = Images::new(&l, 1 << 20, "tok");
    images.cite("a", "", || Ok(Cow::Borrowed(bytes.as_slice())));
    let other = png(3, 3);
    assert_that!(images.cite("b", "", || Ok(Cow::Borrowed(other.as_slice())))).is_none();
    let (notices, _) = images.notices();
    assert_that!(images.into_kept().len()).is_equal_to(1);
    assert_that!(notices.as_str()).contains("(past 25 bytes of images)");
}

/// D4 (t-1760): the same bytes under two keys are one image, linked from both placeholders;
/// the copy takes neither an image nor a byte of the limits.
#[test]
fn images_cite_same_bytes_under_two_keys_linked_to_one_image() {
    let bytes = png(2, 2);
    let l = limits(2, count_u64(bytes.len()) + 1);
    let mut images = Images::new(&l, 1 << 20, "tok");
    let first = images.cite("a", "", || Ok(Cow::Borrowed(bytes.as_slice())));
    let copy = images.cite("b", "copy", || Ok(Cow::Borrowed(bytes.as_slice())));
    assert_that!(first).is_equal_to(Some("![](oku-image-tok-0)".to_owned()));
    assert_that!(copy).is_equal_to(Some("![copy](oku-image-tok-0)".to_owned()));
    assert_that!(images.cite("b", "", || panic!("read twice"))).is_equal_to(Some("![](oku-image-tok-0)".to_owned()));
    let (notices, cut) = images.notices();
    assert_that!(notices).is_equal_to(String::new());
    assert_that!(cut).is_none();
    let other = png(3, 3);
    assert_that!(images.cite("c", "", || Ok(Cow::Borrowed(other.as_slice())))).is_none();
    assert_that!(images.into_kept().len()).is_equal_to(1);
}

#[test]
fn images_rollback_forgets_a_linked_copy() {
    let l = limits(5, 1 << 20);
    let mut images = Images::new(&l, 1 << 20, "tok");
    let bytes = png(2, 2);
    images.cite("a", "", || Ok(Cow::Borrowed(bytes.as_slice())));
    let mark = images.mark();
    images.cite("b", "", || Ok(Cow::Borrowed(bytes.as_slice())));
    images.rollback(mark);
    images.skip("b", Skip::Budget);
    let (notices, _) = images.notices();
    assert_that!(notices).is_equal_to("… 1 images not imported (text budget spent)\n".to_owned());
}

#[test]
fn images_rollback_forgets_images_of_a_dropped_block() {
    let l = limits(5, 1 << 20);
    let mut images = Images::new(&l, 1 << 20, "tok");
    let bytes = png(2, 2);
    let mark = images.mark();
    images.cite("a", "", || Ok(Cow::Borrowed(bytes.as_slice())));
    images.rollback(mark);
    images.skip("a", Skip::Budget);
    let (notices, _) = images.notices();
    assert_that!(images.into_kept().is_empty()).is_true();
    assert_that!(notices).is_equal_to("… 1 images not imported (text budget spent)\n".to_owned());
}

/// M3 (t-1760): a copy of an image demoted for the budget is listed with it, never dropped
/// unstated.
#[test]
fn images_demote_lists_the_copies_it_drops() {
    let l = limits(5, 1 << 20);
    let mut images = Images::new(&l, 1 << 20, "tok");
    let (kept, demoted) = (png(2, 2), png(3, 3));
    images.cite("a", "", || Ok(Cow::Borrowed(kept.as_slice())));
    images.cite("b", "", || Ok(Cow::Borrowed(demoted.as_slice())));
    images.cite("c", "", || Ok(Cow::Borrowed(demoted.as_slice())));
    images.demote(1);
    let (notices, cut) = images.notices();
    assert_that!(notices).is_equal_to("… 2 images not imported (text budget spent)\n".to_owned());
    assert_that!(cut.map(|c| c.count)).is_equal_to(Some(2));
    assert_that!(images.into_kept().len()).is_equal_to(1);
}
