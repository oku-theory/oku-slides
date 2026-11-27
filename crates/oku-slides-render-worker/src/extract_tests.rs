use oku_slides_core::knowledge::{
    Cut, CutKind, DocumentType, ExtractLimits, ExtractRefusal, Extracted, ExtractedImage, MAX_TITLE_CHARS,
};
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::RenderErrorKind;
use speculoos::prelude::*;

use super::*;
use crate::RenderWorker;
use crate::frame::{FrameLimits, read_frame, write_frame};
use crate::protocol::{ImageMeta, Request, Response};
use crate::test_documents::{TOKEN, docx_two_images, limits, png, read};

fn settings(max_frame_mib: u64) -> RenderSettings {
    RenderSettings {
        timeout_ms: 30_000,
        probe_budget_ms: 30_000,
        memory_mib: 4096,
        max_frame_mib,
        max_widget_kib: 256,
        max_widgets: 64,
        max_guide_chars: 4000,
        data_timeout_ms: 120_000,
        data_memory_mib: 4096,
    }
}

fn in_process() -> RenderWorker {
    RenderWorker::new(RenderLaunch::InProcess, settings(64))
}

/// A PNG of `len` bytes (its header, then zeros; cut short below the header's length).
fn image(len: usize) -> ExtractedImage {
    let mut bytes = png(1, 1);
    bytes.resize(len, 0);
    ExtractedImage { mime: "image/png".to_owned(), bytes }
}

fn extracted(markdown: &str, images: Vec<ExtractedImage>, cuts: Vec<Cut>) -> Extracted {
    Extracted { title: Some("T".to_owned()), markdown: markdown.to_owned(), images, cuts }
}

/// The images of a reply built by `reply_of`, read back from its header and payload.
fn reply_images(reply: &(Response, Vec<u8>)) -> (usize, Vec<Cut>) {
    match &reply.0 {
        Response::Extracted { images, cut, .. } => (images.len(), cut.clone()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn extract_roundtrip_markdown_and_images() {
    let doc = docx_two_images();
    let direct = oku_slides_extract::extract(&doc, DocumentType::Docx, &limits(), &read(), TOKEN).unwrap().unwrap();
    let out = in_process().extract_document(&doc, DocumentType::Docx, &limits(), &read(), TOKEN).unwrap().unwrap();
    assert_that!(out.images.iter().map(|i| i.bytes.clone()).collect::<Vec<_>>())
        .is_equal_to(vec![png(2, 2), png(3, 1)]);
    assert_that!(out.markdown.as_str()).contains(format!("](oku-image-{TOKEN}-1)").as_str());
    assert_that!(out).is_equal_to(direct);
}

#[test]
fn extract_refusal_crosses_the_wire() {
    let doc = b"not a zip".as_slice();
    let direct = oku_slides_extract::extract(doc, DocumentType::Docx, &limits(), &read(), TOKEN).unwrap();
    let out = in_process().extract_document(doc, DocumentType::Docx, &limits(), &read(), TOKEN).unwrap();
    assert_that!(out.clone().err()).is_some();
    assert_that!(out).is_equal_to(direct);
}

#[test]
fn extract_refusal_of_a_kind_mismatch_crosses_the_wire() {
    let out = in_process().extract_document(b"\xff\xfe\x00", DocumentType::Text, &limits(), &read(), TOKEN).unwrap();
    assert_that!(out).is_equal_to(Err(ExtractRefusal::TypeMismatch));
}

/// 16 MiB of `"` would be 32 MiB escaped in a JSON header; in the payload it passes a
/// 32 MiB frame, whose header holds 4 MiB.
#[test]
fn extract_markdown_with_json_escaped_chars_in_payload() {
    const MAX: usize = 32 << 20;
    let markdown = "\"".repeat(16 << 20);
    let reply = reply_of(extracted(&markdown, vec![image(40)], Vec::new()), u64::MAX, MAX);
    let wide = ExtractLimits { max_text_bytes: 16 << 20, ..limits() };
    let req = Request::ExtractDocument {
        kind: DocumentType::Markdown,
        limits: wide.clone(),
        read: read(),
        token: TOKEN.into(),
    };
    let mut pipe = Vec::new();
    write_frame(&mut pipe, &reply.0, &reply.1, FrameLimits::reply_to(&req, MAX)).unwrap();
    let frame = read_frame(&mut pipe.as_slice(), FrameLimits::reply_to(&req, MAX)).unwrap().unwrap();
    let Response::Extracted { title, markdown_len, images, cut } = serde_json::from_slice(&frame.header).unwrap()
    else {
        panic!("not an extraction")
    };
    let out = split(title, markdown_len, images, cut, frame.payload, &wide).unwrap();
    assert_that!(out.markdown == markdown).is_true();
    assert_that!(out.images).is_equal_to(vec![image(40)]);
}

#[test]
fn extract_images_past_frame_cut_stated() {
    let reply = reply_of(extracted("0123456789", vec![image(40), image(40), image(40)], Vec::new()), u64::MAX, 100);
    assert_that!(reply.1.len()).is_equal_to(90);
    assert_that!(reply_images(&reply)).is_equal_to((2, vec![Cut { what: CutKind::Images, count: 1 }]));
}

#[test]
fn extract_images_past_image_bytes_cut_stated_with_the_extractor_cut() {
    let cuts = vec![Cut { what: CutKind::Rows, count: 4 }, Cut { what: CutKind::Images, count: 2 }];
    let reply = reply_of(extracted("md", vec![image(40), image(40), image(1)], cuts), 79, usize::MAX);
    let expected = vec![Cut { what: CutKind::Rows, count: 4 }, Cut { what: CutKind::Images, count: 4 }];
    assert_that!(reply_images(&reply)).is_equal_to((1, expected));
}

/// t-1760: the extractor keeps images within `max_image_bytes` itself, so `reply_of` never
/// stops on that bound for what the extractor kept: only the frame cap can drop images.
#[test]
fn reply_of_keeps_every_image_the_extractor_kept() {
    let doc = docx_two_images();
    let exact = ExtractLimits { max_image_bytes: count_u64(png(2, 2).len() + png(3, 1).len()), ..limits() };
    let out = oku_slides_extract::extract(&doc, DocumentType::Docx, &exact, &read(), TOKEN).unwrap().unwrap();
    let reply = reply_of(out, exact.max_image_bytes, usize::MAX);
    assert_that!(reply_images(&reply)).is_equal_to((2, Vec::new()));
}

#[test]
fn extract_markdown_past_frame_refused_too_large() {
    let reply = reply_of(extracted("0123456789", Vec::new(), Vec::new()), u64::MAX, 9);
    assert_that!(matches!(reply.0, Response::ExtractRefused { refusal: ExtractRefusal::TooLarge })).is_true();
    assert_that!(reply.1).is_empty();
}

fn metas(lens: &[usize]) -> Vec<ImageMeta> {
    lens.iter().map(|&len| ImageMeta { mime: "image/png".to_owned(), len }).collect()
}

#[test]
fn extract_reply_lengths_short_of_payload_is_error() {
    let err = split(None, 2, metas(&[1]), Vec::new(), b"abcd".to_vec(), &limits()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn extract_reply_lengths_overflowing_is_error() {
    let err = split(None, usize::MAX, metas(&[1]), Vec::new(), b"ab".to_vec(), &limits()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn extract_reply_markdown_not_utf8_is_error() {
    let err = split(None, 2, Vec::new(), Vec::new(), vec![0xff, 0xfe], &limits()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn extract_reply_past_the_image_limits_is_error() {
    let mut tight = limits();
    tight.max_images = 1;
    let err = split(None, 0, metas(&[1, 1]), Vec::new(), b"ab".to_vec(), &tight).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
    tight.max_images = 2;
    tight.max_image_bytes = 1;
    let err = split(None, 0, metas(&[1, 1]), Vec::new(), b"ab".to_vec(), &tight).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

/// The parts of `payload` (`markdown`, then `images`) as `split` reads them back.
fn split_parts(markdown: &str, images: &[ExtractedImage], limits: &ExtractLimits) -> Result<Extracted, RenderError> {
    let mut payload = markdown.as_bytes().to_vec();
    let metas = images
        .iter()
        .map(|i| {
            payload.extend_from_slice(&i.bytes);
            ImageMeta { mime: i.mime.clone(), len: i.bytes.len() }
        })
        .collect();
    split(None, markdown.len(), metas, Vec::new(), payload, limits)
}

#[test]
fn extract_reply_split_images_hold_only_their_bytes() {
    let images = vec![image(1000), image(2000), image(3000)];
    let out = split_parts("some markdown", &images, &limits()).unwrap();
    assert_that!(out.images.iter().map(|i| i.bytes.capacity()).collect::<Vec<_>>()).is_equal_to(vec![1000, 2000, 3000]);
    assert_that!(out.markdown.capacity()).is_equal_to("some markdown".len());
    assert_that!(out.images).is_equal_to(images);
}

#[test]
fn extract_reply_mime_outside_rasters_is_error() {
    let svg = ExtractedImage { mime: "image/svg+xml".to_owned(), ..image(40) };
    let err = split_parts("md", &[svg], &limits()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn extract_reply_mime_not_the_sniffed_one_is_error() {
    let jpeg_claimed = ExtractedImage { mime: "image/jpeg".to_owned(), ..image(40) };
    let err = split_parts("md", &[jpeg_claimed], &limits()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn extract_reply_markdown_past_text_bound_is_error() {
    let tight = ExtractLimits { max_text_bytes: 4, ..limits() };
    assert_that!(split_parts("1234", &[], &tight).unwrap().markdown).is_equal_to("1234".to_owned());
    let err = split_parts("12345", &[], &tight).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

/// A reply of `markdown`, `images` and `cuts`, titled `title`, as `split` reads it back.
fn split_reply(
    title: Option<String>,
    markdown: &str,
    images: &[ExtractedImage],
    cuts: Vec<Cut>,
) -> Result<Extracted, RenderError> {
    let mut payload = markdown.as_bytes().to_vec();
    let metas = images
        .iter()
        .map(|i| {
            payload.extend_from_slice(&i.bytes);
            ImageMeta { mime: i.mime.clone(), len: i.bytes.len() }
        })
        .collect();
    split(title, markdown.len(), metas, cuts, payload, &limits())
}

#[test]
fn split_refuses_image_past_pixel_limit() {
    let bomb = ExtractedImage { mime: "image/png".to_owned(), bytes: png(30_000, 30_000) };
    let err = split_reply(None, "md", &[bomb], Vec::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn split_refuses_image_without_readable_header() {
    let short = ExtractedImage { mime: "image/png".to_owned(), bytes: png(2, 2)[..18].to_vec() };
    let err = split_reply(None, "md", &[short], Vec::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn split_refuses_long_title() {
    let at_most = "t".repeat(MAX_TITLE_CHARS);
    assert_that!(split_reply(Some(at_most.clone()), "md", &[], Vec::new())).is_ok();
    let err = split_reply(Some(format!("{at_most}é")), "md", &[], Vec::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn split_refuses_too_many_cuts() {
    let twice = vec![Cut { what: CutKind::Rows, count: 1 }, Cut { what: CutKind::Rows, count: 2 }];
    let err = split_reply(None, "md", &[], twice).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

/// M6 (t-1760): more cuts than there are kinds are refused before any pairwise check.
#[test]
fn split_refuses_more_cuts_than_kinds() {
    let many = vec![Cut { what: CutKind::Text, count: 1 }; 10_000];
    let err = split_reply(None, "md", &[], many).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

#[test]
fn split_refuses_markdown_with_bidi_control() {
    let err = split_reply(None, "safe \u{202E}txt.exe", &[], Vec::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
    let err = split_reply(None, "line\r\nend", &[], Vec::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

/// A fake worker (shell script), answering `Hello` then the extraction with raw bytes.
#[cfg(unix)]
mod fake {
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::protocol::PROTOCOL_VERSION;

    pub(super) struct Fake {
        _dir: tempfile::TempDir,
        pub(super) worker: RenderWorker,
    }

    /// Reads `Hello` and answers it, then answers `reply` once the next request starts
    /// arriving (the host reads a reply under that request's limits), then runs `then`.
    pub(super) fn fake(reply: &[u8], then: &str) -> Fake {
        let dir = tempfile::Builder::new().prefix("oku-slides-fake-extract-").tempdir().unwrap();
        let mut hello = Vec::new();
        let req = Request::Hello { version: PROTOCOL_VERSION, canary: None, bases: None };
        write_frame(&mut hello, &req, &[], FrameLimits::uniform(usize::MAX)).unwrap();
        let mut hi = Vec::new();
        let hi_reply = Response::Hello { version: PROTOCOL_VERSION, canary: None };
        write_frame(&mut hi, &hi_reply, &[], FrameLimits::uniform(usize::MAX)).unwrap();
        let hi_path = dir.path().join("hello.bin");
        std::fs::write(&hi_path, hi).unwrap();
        let reply_path = dir.path().join("reply.bin");
        std::fs::write(&reply_path, reply).unwrap();
        let program = dir.path().join("worker.sh");
        let body = format!(
            "#!/bin/sh\nhead -c {} >/dev/null\ncat '{}'\nhead -c 1 >/dev/null\ncat '{}'\n{then}\n",
            hello.len(),
            hi_path.display(),
            reply_path.display()
        );
        std::fs::write(&program, body).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let worker = RenderWorker::new(RenderLaunch::Process { program }, settings(64));
        worker.link().unsandboxed = true;
        Fake { _dir: dir, worker }
    }

    pub(super) fn extracted_frame(markdown_len: usize, payload: &[u8]) -> Vec<u8> {
        let reply = Response::Extracted { title: None, markdown_len, images: Vec::new(), cut: Vec::new() };
        let mut frame = Vec::new();
        write_frame(&mut frame, &reply, payload, FrameLimits::uniform(usize::MAX)).unwrap();
        frame
    }
}

#[cfg(unix)]
#[test]
fn extract_truncated_reply_is_error() {
    let mut frame = fake::extracted_frame(6, b"abcdef");
    frame.truncate(frame.len() - 3);
    // The worker dies once the request arrives, mid-reply.
    let f = fake::fake(&frame, "exit 0");
    let err = f.worker.extract_document(b"doc", DocumentType::Text, &limits(), &read(), TOKEN).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::WorkerLost);
}

#[cfg(unix)]
#[test]
fn extract_reply_lengths_past_payload_is_error() {
    let f = fake::fake(&fake::extracted_frame(10, b"abc"), "cat >/dev/null");
    let err = f.worker.extract_document(b"doc", DocumentType::Text, &limits(), &read(), TOKEN).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}

/// End to end (t-1760): a worker replying a 30000 × 30000 PNG header is refused before any
/// pixel is trusted, as is Markdown carrying a bidi override.
#[cfg(unix)]
#[test]
fn extract_reply_image_past_pixels_or_unclean_markdown_is_error() {
    let bomb = png(30_000, 30_000);
    let reply = Response::Extracted {
        title: None,
        markdown_len: 2,
        images: vec![ImageMeta { mime: "image/png".to_owned(), len: bomb.len() }],
        cut: Vec::new(),
    };
    let mut frame = Vec::new();
    write_frame(&mut frame, &reply, &[b"md".as_slice(), &bomb].concat(), FrameLimits::uniform(usize::MAX)).unwrap();
    let f = fake::fake(&frame, "cat >/dev/null");
    let err = f.worker.extract_document(b"doc", DocumentType::Text, &limits(), &read(), TOKEN).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);

    let markdown = "a\u{202E}b".as_bytes();
    let f = fake::fake(&fake::extracted_frame(markdown.len(), markdown), "cat >/dev/null");
    let err = f.worker.extract_document(b"doc", DocumentType::Text, &limits(), &read(), TOKEN).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Frame);
}
