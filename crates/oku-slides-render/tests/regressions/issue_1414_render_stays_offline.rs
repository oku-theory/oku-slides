//! t-1414: a received pack's widget must not reach outside the in-memory Typst world.
//!
//! Each widget body reads a file that really exists on disk (text, image, plugin), by its
//! absolute path or by a `..` path climbing to it. A world that read the disk would render
//! it; the closed world fails with Typst's "not found" diagnostic naming the path instead. A
//! `..` path never reaches the world: Typst refuses it first, as escaping the project root.

use std::collections::BTreeMap;
use std::path::Path;

use oku_slides_core::{DECK_VERSION, Deck, kinds::essentiel};
use oku_slides_render::{PackSet, RenderErrorKind};
use rstest::{fixture, rstest};
use serde_json::json;
use speculoos::prelude::*;
use tempfile::TempDir;

use crate::common::{pack, renderer_with};

/// Typst's message for a file the world does not serve (`FileError::NotFound`).
const NOT_FOUND: &str = "not found";

/// Typst's message for a path that climbs above the root, refused before the world.
const ESCAPES_ROOT: &str = "would escape the project root";

/// Text of the file on disk; no diagnostic may quote it.
const SECRET: &str = "oku-1414-secret-on-disk";

/// Smallest valid WebAssembly module: magic number and version 1, no section.
const EMPTY_WASM: &[u8] = b"\0asm\x01\0\0\0";

/// A one-pixel SVG image, valid for Typst's `image`.
const PIXEL_SVG: &str =
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1"/></svg>"#;

/// A temp dir holding `secret.txt`, `pixel.svg` and `empty.wasm`, real files a disk world would serve.
#[fixture]
fn disk() -> TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(dir.path().join(name), bytes).unwrap_or_else(|e| panic!("write {name}: {e}"));
    };
    write("secret.txt", SECRET.as_bytes());
    write("pixel.svg", PIXEL_SVG.as_bytes());
    write("empty.wasm", EMPTY_WASM);
    dir
}

/// Which way a widget body tries to reach a file on disk.
#[derive(Debug, Clone, Copy)]
enum Reach {
    /// `read` of the text file by its absolute path.
    AbsoluteRead,
    /// `image` of the SVG file by its absolute path.
    AbsoluteImage,
    /// `plugin` of the wasm module by its absolute path.
    AbsolutePlugin,
    /// `read` of the text file by a `..` path climbing from the widget to it.
    RelativeEscape,
}

impl Reach {
    /// The file this reach targets, under `dir`.
    fn target(self, dir: &Path) -> String {
        let name = match self {
            Reach::AbsoluteRead | Reach::RelativeEscape => "secret.txt",
            Reach::AbsoluteImage => "pixel.svg",
            Reach::AbsolutePlugin => "empty.wasm",
        };
        dir.join(name).to_string_lossy().into_owned()
    }

    /// The Typst code (code mode) reaching `target`, and the path Typst sees.
    fn code(self, target: &str) -> (String, String) {
        let path = match self {
            Reach::RelativeEscape => format!("{}{}", "../".repeat(32), target.trim_start_matches('/')),
            _ => target.to_owned(),
        };
        let code = match self {
            Reach::AbsoluteRead | Reach::RelativeEscape => format!("read({path:?})"),
            Reach::AbsoluteImage => format!("image({path:?})"),
            Reach::AbsolutePlugin => format!("let p = plugin({path:?}); [#p]"),
        };
        (code, path)
    }
}

/// A pack widget `fuite` whose body evaluates `escape` (Typst code mode) and shows the result.
fn leaking_widget(escape: &str) -> String {
    format!(
        "#import \"/oku-slides/lib.typ\": widget\n\
         #let widget = widget(\"fuite\", label: \"Fuite\", params: (:), (args, ctx) => {{\n  {escape}\n}})\n"
    )
}

/// Renders a one-slide deck whose single block is the `fuite` widget evaluating `escape`.
fn render_leak(escape: &str) -> Vec<String> {
    let source = leaking_widget(escape);
    let set = PackSet::new(vec![pack("fuite", &[("fuite", &source)])]).unwrap_or_else(|e| panic!("{e}"));
    let mut r = renderer_with(&set);
    let deck = json!({
        "version": DECK_VERSION, "params": { "title": "T" },
        "slides": [{ "id": "s", "kind": essentiel::CONTENT,
            "params": { "blocks": [{ "id": "f", "kind": "test/fuite", "params": {} }] } }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("invalid deck: {e}"));
    let Err(error) = r.render(&deck, &BTreeMap::new()) else {
        panic!("`{escape}` rendered: the world served something outside it")
    };
    assert_that!(error.kind()).is_equal_to(RenderErrorKind::Compile);
    error.diagnostics().iter().map(|d| d.message.clone()).collect()
}

#[rstest]
#[case::absolute_read(Reach::AbsoluteRead, NOT_FOUND)]
#[case::absolute_image(Reach::AbsoluteImage, NOT_FOUND)]
#[case::absolute_plugin(Reach::AbsolutePlugin, NOT_FOUND)]
#[case::relative_escape(Reach::RelativeEscape, ESCAPES_ROOT)]
fn render_widget_reading_disk_file_fails_on_its_path(disk: TempDir, #[case] reach: Reach, #[case] expected: &str) {
    let (code, path) = reach.code(&reach.target(disk.path()));

    let messages = render_leak(&code);

    let named = &format!("{messages:?}");
    assert_that!(messages.iter().any(|m| m.contains(expected))).named(named).is_true();
    if expected == NOT_FOUND {
        assert_that!(messages.iter().any(|m| m.contains(&path))).named(named).is_true();
    }
    assert_that!(messages.iter().any(|m| m.contains(SECRET))).named(named).is_false();
}

/// No package resolves: a world with a package downloader or cache would import this one.
///
/// It cannot prove the absence of a download on its own (the package may be missing offline);
/// `just render-offline-deps` guards the worker's dependency tree against any HTTP client.
#[rstest]
fn render_widget_importing_package_fails_not_found() {
    let messages = render_leak(r#"import "@preview/cetz:0.3.4": *"#);

    assert_that!(messages.iter().any(|m| m.contains(NOT_FOUND))).named(&format!("{messages:?}")).is_true();
}

/// `read` takes no URL: Typst sees a relative path, which no world can serve from the web.
///
/// It documents the language, not the world: even a disk world would fail it.
#[rstest]
fn render_widget_reading_url_fails_not_found() {
    let messages = render_leak(r#"read("https://example.com")"#);

    assert_that!(messages.iter().any(|m| m.contains(NOT_FOUND))).named(&format!("{messages:?}")).is_true();
}
