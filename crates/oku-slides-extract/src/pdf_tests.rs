use lopdf::{Dictionary, Stream, dictionary};
use speculoos::prelude::*;

use super::*;

fn flate(w: i64, h: i64, space: &str, raw: Vec<u8>) -> Stream {
    let mut s = Stream::new(Dictionary::new(), raw);
    s.compress().unwrap();
    assert!(s.dict.has(b"Filter"), "test data too random to compress");
    let mut dict = dictionary! {
        "Subtype" => "Image",
        "Width" => w,
        "Height" => h,
        "ColorSpace" => space,
        "BitsPerComponent" => 8,
        "Filter" => "FlateDecode",
    };
    dict.set("Length", i64::try_from(s.content.len()).unwrap());
    Stream { dict, ..s }
}

/// A total read budget that never runs out.
fn all() -> Cell<usize> {
    Cell::new(usize::MAX)
}

fn empty() -> Document {
    Document::with_version("1.5")
}

#[test]
fn load_refusal_maps_header_limit_and_password() {
    let header = Error::Parse(ParseError::InvalidFileHeader);
    let limit = Error::Decompress(DecompressError::MemoryLimitExceeded { limit: 1 });
    assert_that!(load_refusal(&header)).is_equal_to(ExtractRefusal::TypeMismatch);
    assert_that!(load_refusal(&limit)).is_equal_to(ExtractRefusal::TooLarge);
    assert_that!(load_refusal(&Error::InvalidPassword)).is_equal_to(ExtractRefusal::Encrypted);
    assert_that!(load_refusal(&Error::MissingXrefEntry)).is_equal_to(ExtractRefusal::Damaged);
}

#[test]
fn flate_png_gray_image_encoded() {
    let png = flate_png(&empty(), &flate(16, 16, "DeviceGray", vec![7; 256]), 1 << 20, 1000, &all()).unwrap();
    assert_that!(png.starts_with(b"\x89PNG")).is_true();
}

#[test]
fn flate_png_indirect_entries_resolved() {
    let mut pdf = empty();
    let mut image = flate(16, 16, "DeviceGray", vec![7; 256]);
    let (w, h, space) = (pdf.add_object(16), pdf.add_object(16), pdf.add_object(Object::Name(b"DeviceGray".to_vec())));
    image.dict.set("Width", w);
    image.dict.set("Height", h);
    image.dict.set("ColorSpace", space);
    let png = flate_png(&pdf, &image, 1 << 20, 1000, &all()).unwrap();
    assert_that!(png.starts_with(b"\x89PNG")).is_true();
}

#[test]
fn flate_png_short_data_unreadable() {
    let short = flate(20, 20, "DeviceRGB", vec![0; 600]);
    assert_that!(flate_png(&empty(), &short, 1 << 20, 1000, &all())).is_equal_to(Err(Skip::Unreadable));
}

#[test]
fn flate_png_bomb_read_limit() {
    let bomb = flate(16, 16, "DeviceGray", vec![0; 1 << 20]);
    assert_that!(flate_png(&empty(), &bomb, 1 << 20, 1000, &all())).is_equal_to(Err(Skip::ReadLimit));
}

#[test]
fn flate_png_past_part_read_limit() {
    let image = flate(100, 100, "DeviceRGB", vec![0; 30_000]);
    assert_that!(flate_png(&empty(), &image, 1000, 1 << 20, &all())).is_equal_to(Err(Skip::ReadLimit));
}

#[test]
fn flate_png_predictor_or_cmyk_format() {
    let mut predicted = flate(8, 8, "DeviceRGB", vec![0; 192]);
    predicted.dict.set("DecodeParms", dictionary! { "Predictor" => 15 });
    assert_that!(flate_png(&empty(), &predicted, 1 << 20, 100, &all()))
        .is_equal_to(Err(Skip::Format("PDF image with a predictor".to_owned())));
    let cmyk = flate(8, 8, "DeviceCMYK", vec![0; 256]);
    assert_that!(flate_png(&empty(), &cmyk, 1 << 20, 100, &all()))
        .is_equal_to(Err(Skip::Format("PDF DeviceCMYK image".to_owned())));
}

#[test]
fn image_bytes_other_filter_listed_by_name() {
    let jpx = Stream::new(dictionary! { "Filter" => "JPXDecode" }, vec![0; 4]);
    assert_that!(image_bytes(&empty(), &jpx, 1 << 20, 100, &all()).err())
        .is_equal_to(Some(Skip::Format("PDF JPXDecode".to_owned())));
}

/// t-1760: a Flate image decodes within the document's total left, which it spends.
#[test]
fn flate_png_past_total_left_read_limit_and_spends_it() {
    let image = || flate(16, 16, "DeviceGray", vec![7; 256]);
    let left = Cell::new(300);
    assert_that!(flate_png(&empty(), &image(), 1 << 20, 1000, &left).is_ok()).is_true();
    assert_that!(left.get()).is_equal_to(44);
    assert_that!(flate_png(&empty(), &image(), 1 << 20, 1000, &left)).is_equal_to(Err(Skip::ReadLimit));
}

/// D7 (t-1760): an image whose pixels its samples alone do not give is skipped, with why.
#[test]
fn image_bytes_altered_pixels_skipped_with_reason() {
    let cases = [
        ("ImageMask", Object::Boolean(true), "PDF image mask"),
        ("SMask", Object::Reference((9, 0)), "PDF image with a soft mask"),
        ("Mask", Object::Array(vec![0.into(), 0.into()]), "PDF image with a mask"),
        ("Decode", Object::Array(vec![1.into(), 0.into()]), "PDF image with a decode array"),
    ];
    for (key, value, why) in cases {
        let mut image = flate(16, 16, "DeviceGray", vec![7; 256]);
        image.dict.set(key, value);
        assert_that!(image_bytes(&empty(), &image, 1 << 20, 1000, &all()).err())
            .is_equal_to(Some(Skip::Format(why.to_owned())));
    }
}

/// D7 (t-1760): a JPEG kept only in RGB or gray; a CMYK one would show wrong colors.
#[test]
fn image_bytes_jpeg_kept_in_rgb_only() {
    let jpeg =
        |space: &str| Stream::new(dictionary! { "Filter" => "DCTDecode", "ColorSpace" => space }, vec![0xFF, 0xD8]);
    assert_that!(image_bytes(&empty(), &jpeg("DeviceRGB"), 1 << 20, 100, &all()).is_ok()).is_true();
    assert_that!(image_bytes(&empty(), &jpeg("DeviceCMYK"), 1 << 20, 100, &all()).err())
        .is_equal_to(Some(Skip::Format("PDF DeviceCMYK image".to_owned())));
}

#[test]
fn counts_no_text_refusal_by_cause() {
    let counts = |over, unreadable, forms| Counts { unreadable, over, forms, ..Counts::default() };
    assert_that!(counts(0, 0, 0).no_text(0)).is_equal_to(ExtractRefusal::NoTextLayer);
    assert_that!(counts(0, 0, 0).no_text(3)).is_equal_to(ExtractRefusal::TooLarge);
    assert_that!(counts(1, 1, 1).no_text(0)).is_equal_to(ExtractRefusal::TooLarge);
    assert_that!(counts(0, 1, 1).no_text(0)).is_equal_to(ExtractRefusal::Damaged);
    assert_that!(counts(0, 0, 1).no_text(0)).is_equal_to(ExtractRefusal::Unsupported);
}

/// A `DCTDecode` stream in color space `[/ICCBased s]`, `s` a profile of `n` components.
fn icc_jpeg(pdf: &mut Document, n: i64) -> Stream {
    let profile = pdf.add_object(Stream::new(dictionary! { "N" => n }, vec![0; 4]));
    let space = Object::Array(vec![Object::Name(b"ICCBased".to_vec()), Object::Reference(profile)]);
    Stream::new(dictionary! { "Filter" => "DCTDecode", "ColorSpace" => space }, vec![0xFF, 0xD8])
}

/// N1 (t-1760): a JPEG tagged with an RGB or gray ICC profile is kept as it is.
#[test]
fn image_bytes_jpeg_icc_rgb_or_gray_kept() {
    let mut pdf = empty();
    for n in [1, 3] {
        let jpeg = icc_jpeg(&mut pdf, n);
        assert_that!(image_bytes(&pdf, &jpeg, 1 << 20, 100, &all()).is_ok()).is_true();
    }
}

/// N1 (t-1760): a JPEG tagged with a CMYK ICC profile is skipped, with why.
#[test]
fn image_bytes_jpeg_icc_cmyk_skipped() {
    let mut pdf = empty();
    let jpeg = icc_jpeg(&mut pdf, 4);
    assert_that!(image_bytes(&pdf, &jpeg, 1 << 20, 100, &all()).err())
        .is_equal_to(Some(Skip::Format("PDF ICC image of 4 components".to_owned())));
}

/// M2 (t-1760): an identity decode array leaves the samples as they are: the image is kept.
#[test]
fn image_bytes_identity_decode_kept() {
    let mut image = flate(16, 16, "DeviceGray", vec![7; 256]);
    image.dict.set("Decode", Object::Array(vec![0.into(), 1.into()]));
    assert_that!(image_bytes(&empty(), &image, 1 << 20, 1000, &all()).is_ok()).is_true();
    let mut rgb = Stream::new(dictionary! { "Filter" => "DCTDecode", "ColorSpace" => "DeviceRGB" }, vec![0xFF, 0xD8]);
    rgb.dict.set("Decode", Object::Array(vec![0.into(), 1.0.into(), 0.into(), 1.into(), 0.into(), 1.into()]));
    assert_that!(image_bytes(&empty(), &rgb, 1 << 20, 100, &all()).is_ok()).is_true();
}
