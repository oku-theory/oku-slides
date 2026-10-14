# PowerPoint import — lot 4: reference rendering and visual gap — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `oku-slides import deck.pptx --mode hifi [--reference auto|none]` measures each imported slide against a reference rendering of the presentation made by an independent witness, writes a visual gap per slide and the worst regions (the faulty items) to the report, and reports text fields rendered past their page in both modes.

**Architecture:** A new crate `oku-slides-reference` holds the witness behind a `ReferenceRenderer` trait: `Office2Pdf` (the `office2pdf` crate, pure Rust on Typst 0.15) renders the `.pptx` to PDF with the app's and the deck's fonts, and `hayro` rasterizes its pages. The witness reads an untrusted file without bounds of its own, so it runs inside the render worker (new `Reference`, `ReferencePng`, `ReleaseReference` requests; `RenderWorker::reference` → `RemoteReference`). The pure import crate gains `metrics` (OKLab ΔE of two borrowed RGBA images, regions from the source items) and report types (`ReferenceReport`, `ReferenceCause`, `SlideReport::score`, `LossKind::Overflow`). The CLI orchestrates: render the deck, crop each Oku page to the source page at 960 px, ask the worker for the witness pages, score, and read overflows from the render `Layout`.

**Tech Stack:** Rust 2024; `office2pdf` 0.8.1 (`default-features = false`), `hayro` 0.7.1 (already in the lockfile through `typst-render`), `png`; `oku-slides-render-worker` (bounded worker protocol), `oku-slides-render` (`Layout`, `Compiled::png`), `oku-slides-i18n` (Fluent, 20 locales); tests with `speculoos`, `rstest`, nextest.

**Spec:** `docs/superpowers/specs/2026-10-09-import-pptx-design.md` — "Reference rendering and metrics", "Import report", "Entry points", "Tests", "Lots", and the section "Decisions from the lot 4 plan (2026-10-14)" that Task 7 adds. Earlier plans: `docs/superpowers/plans/2026-10-09-import-pptx-lot1.md`, `…-10-10-import-pptx-lot2.md`, `…-10-11-import-pptx-lot3.md`; their validated decisions still apply.

**Subtasks:** Task 1 → t-873, Task 2 → t-874, Task 3 → t-875, Task 4 → t-876, Task 5 → t-877, Task 6 → t-878, Task 7 → t-879, Task 8 → t-880, Task 9 → t-881, Task 10 → t-882, Task 11 → t-883 (parent t-692). Backlog from the plan review: t-884 to t-890.

**Drafting:** every task was implemented, built and tested in a draft worktree before being written down; the code in this plan is the code of the draft commits. The real render of alto, `libreoffice.pptx` and `kitchen` was measured and compared page by page (`pdftoppm`) while drafting; Task 5 is the recalibration that this comparison required. The plan was then reviewed by Opus with a real render and by the 12-agent review team; their findings became Tasks 8 to 11 (see "Plan review"), appended without rewriting the earlier tasks: each diff applies on the state the previous task left.

## Decisions validated on 2026-10-13/14 (they override the spec)

1. **Witness: `office2pdf`, not LibreOffice.** No embeddable PPTX renderer is a faithful reference and the user ruled out depending on a local `soffice`. A spike compared `office2pdf` 0.8.1, LibreOffice 26.8 and Oku on alto: office2pdf gets the p. 8 bullet indent right (27 pt, `marL`/`indent`) and keeps the p. 7 callout inside its box (PowerPoint's « every » line break, t-863), where LibreOffice shrinks `normAutofit` text. It sits behind a `ReferenceRenderer` trait; LibreOffice and PowerPoint witnesses are backlog work.
2. **Fonts.** The witness gets the app's fonts and the deck's embedded fonts first (the gap measures layout, not font substitution); office2pdf keeps the system fonts as its own fallback. Hidden slides are rendered (the import keeps them, visible).
3. **Isolation.** The witness runs in a render worker of its own (not the one holding the Oku render), under `[import] reference_timeout_ms` (30 s) and `memory_mib`, 20 slides per request (`slide_range`, Tasks 9–10: one whole-deck request reached 3.27 GB at 240 slides); no pack is blamed. A failing or mismatched reference, or a slide that cannot be scored, gives a `ReferenceCause` (`failed`, `timeout`, `memory`, `pages`) in the report and a log line, never an error, and no slide keeps a score.
4. **Metric** (`metrics.rs`, pure). Slide score: mean OKLab ΔE ×100 over the source page at 960 px, after a Gaussian blur, σ = 2 px (Task 8; σ = 1 let titles aligned within 1 px rank above the real gaps). Region score (Task 5): pixels owned by the topmost item (at most 256 regions, Task 8), region realigned by the best integer shift up to 3 px per axis (reported), score = mean gap of the worst 16 px tile (half-overlapping), no score under 100 owned pixels or for a negative frame; 3 worst regions kept.
5. **Threshold and tests.** A slide is flagged when its **worst region** exceeds `[import] score_threshold = 10.0` (Task 10): the slide mean does not separate a faithful slide from a damaged one (the broken table of `libreoffice.pptx` p. 1 scores 0.6). Measured on alto: titles aligned within 1 px 7.7–8.3 (noise floor), real gaps 8.7–22.5, witness errors up to 46. CLI tests run the witness live (pinned at `=0.8.1`; no versioned reference PDF), with per-fixture bounds on the slide mean (measurement + 25 %) and designation checks: alto p. 8 two bullet lists and p. 7 callout among the 3 worst regions, `libreoffice.pptx` p. 1 table first; plus the timeout path and an end-to-end retrofit overflow.
6. **Retrofit.** No visual gap (another theme): `--reference` is refused in retrofit (`cli-import-reference-hifi-only`). In both modes, a non-empty text field (`FieldBox`) rendered past its page edge becomes an `Overflow` loss of its slide; shapes and images that bleed on purpose do not count. t-816 stays open for the fix.
7. **Crate placement.** `oku-slides-import` stays pure (no file, no process, no render): it scores borrowed images. The CLI renders and crops. `hifi::source_rect` gives the source page on the Oku page so the CLI does not redo the page fit.
8. **Documents in English** (AGENTS.md, since t-833): the `ARCHITECTURE.md` files, the spec additions and this plan are in English; code comments and commit messages stay French.

Clarifications made along the way:

- `office2pdf` pulls a second `zip` (0.6) and `quick-xml` 0.37 (via `umya-spreadsheet`, its XLSX reader, never called): `deny.toml` extends the existing quick-xml advisory ignore with that reason.
- The alto titles and footers come out first among the worst regions on most slides: the witness draws letter-spaced titles 2–5 px wider than Oku (which matches HarfBuzz within 1 px) and LibreOffice, and shrinks `normAutofit` text without `fontScale` (the p. 2 title, every footer). Witness limits, documented; an autofit-aware ranking is backlog work before lot 5.
- The witness gets the `.pptx` without its `ppt/fonts/` parts (Task 9): office2pdf writes embedded fonts to the system temp dir and a killed worker would leave them there; the fonts already travel in memory.
- office2pdf brings its DOCX and XLSX readers (~48 crates) into the render worker and the desktop app: accepted, since lot 5 needs the witness in the app (user decision).
- `oku-slides import --mode hifi` prints a second line: the witness, the mean gap and the number of slides whose worst region is above the threshold, or why the reference is missing.

## Global Constraints

- Workspace: Rust 2024; `cargo clippy --workspace --all-targets -- -D warnings`; no `unwrap`/`expect` outside tests; `#[expect(..., reason = "...")]` rather than `#[allow]`; rustfmt width 120; `just check` (fmt, clippy, tsc, LFS fonts, agents-lint) passes.
- `oku-slides-import` stays free of any dependency on the session, the render, the CLI and of any I/O (crate AGENTS.md); `metrics` takes images from the caller.
- All Typst work runs in the bounded render worker (root AGENTS.md, t-327): the witness too.
- Messages: every user-facing string goes through Fluent; new keys in the 20 locales (`ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`), same variables and key order everywhere; no hard-coded `1` in a plural branch; no space between a number and its CJK counter (ja, ko, zh); report keys in `document.ftl` (`import-`), CLI keys in `cli.ftl` (`cli-`).
- Files ≤ 500 lines; tests side by side (`foo_tests.rs`, `#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`); test names `<unit>_<scenario>_<result>`.
- Cargo: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target` (shared target, user's decision) and `touch crates/*/src/lib.rs` in the same command as every build or test; a failure in a test the task does not touch is a stale artifact until a fresh run proves otherwise.
- Every run of the CLI or the app that writes data: `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` set to a fresh temporary dir (the `oku_slides()` test helper does it).
- Commits: conventional, in French, `type(scope) : description (t-692)`; message in a file, `git commit -F <file>` alone; `git add` in its own command; no attribution line.

## Draft branch

The draft commits stay on branch **`692-draft/import-pptx-lot4`** until this plan is merged (its worktree is removed; the commits stay reachable from any worktree of the repository). Base: `53454df2`. Task → commit: Task 1 `98506223`, Task 2 `dc32a579`, Task 3 `e72b2749`, Task 4 `40a05b9d`, Task 5 `e33bc28`, Task 6 `4982645` (with `ar/cli.ftl` from `dc0dd55`, a fix of the draft), Task 7 `f86a6c84` (after the merge of `main`, `6d7037a`; its diffs are taken against `main`), Task 8 `eaeb700`, Task 9 `211ded7`, Task 10 `a5546575`, Task 11 `f96e0ef2`. Extraction commands run **in the implementer's worktree** (`git checkout <c> -- <file> …`), never with `git -C` on the draft. An implementer may extract a task's files from its commit — only the commit's own files (`git diff --name-only <c>~1 <c>`), never a directory; a file changed on `main` since `53454df2`, or by a review fix of an earlier task, is applied with `git show <c> -- <file> | git apply --3way`.

## Structure

```
crates/oku-slides-reference/            new crate (Task 1)
  src/lib.rs                            ReferenceRenderer trait, re-exports
  src/office2pdf.rs                     Office2Pdf witness
  src/pdf.rs                            ReferencePdf, MAX_RASTER_SIDE, hayro rasterization
  src/raster.rs                         Raster (RGBA), crop, PNG in and out
  src/error.rs                          ReferenceError, ReferenceErrorKind
crates/oku-slides-render-worker/src/
  protocol.rs, frame.rs, serve.rs       Reference, ReferencePng, ReleaseReference (Task 2)
  reference.rs                          RenderWorker::reference, RemoteReference
  client.rs                             put_fonts, Deferred::ReleaseReference
crates/oku-slides-import/src/
  metrics.rs                            score, regions, Image, Region, SlideScore, RegionScore (Tasks 3, 5)
  report.rs, report/reference.rs        ReferenceReport, ReferenceCause, SlideReport.score, Overflow (Task 4)
  report/fonts.rs                       font_notice, moved out of report.rs
  hifi/mod.rs, hifi/page.rs             source_rect (Task 6)
crates/oku-slides-platform/src/         [import] score_threshold (Task 6)
crates/oku-slides-cli/src/
  import.rs, main.rs, pack_render.rs    --reference, write_rendered, Rendered::png, DeckWidgets::worker (Task 6)
  import_reference.rs                   witness, crop, score per slide
  import_overflow.rs                    Overflow losses from the render Layout
crates/oku-slides-cli/tests/import_reference.rs   live witness tests (Task 6)
crates/oku-slides-i18n/src/locales/*/   document.ftl (Task 4), cli.ftl (Task 6)
docs, ARCHITECTURE.md, AGENTS.md        Task 7
```

## Order and dependencies

Tasks 1 → 2 (the worker uses the witness) → 3 → 4 (the report uses `SlideScore`) → 5 (rewrites the region score of Task 3) → 6 (uses everything) → 7 (documents) → 8 (import review fixes) → 9 (witness and worker review fixes) → 10 (CLI review fixes, uses 8 and 9) → 11 (documents after the review). Each diff applies on the state left by the previous task.

## Plan review

Opus reviewed the plan with a real render of alto and `libreoffice.pptx` (scores per slide, witness pages rasterized and compared by eye, a numpy reimplementation of the metric), and the 12-agent review team (8 language-agnostic reviewers and 4 Rust lenses) read the plan and the draft. Consolidated: 3 critical, 9 major, 14 minor, 6 deferred. What changed, and where:

- **Critical**: a negative `a:ext cx` panicked `metrics::score` (Task 8, with a regression test); `cargo deny check licenses` failed on libfuzzer-sys (Task 9); the extraction commands wrote into the draft worktree (fixed in this text).
- **Major**: the witness's memory (own worker, 20-slide spans, fonts handed over: Tasks 9–10); the region ranking and the slide mean as signals (σ = 2, worst-region threshold: Tasks 8, 10); silent fallbacks in scoring (Task 10); missing or weak tests (Tasks 8, 10); the temp-font leak (Task 9); `Raster::new` panicking and the unpinned witness (Task 9); unbounded host-side scoring (Task 8).
- **Minor**, fixed: duplicate dev-dependency, dead public API, threshold validation, `#[non_exhaustive]`, the `Overflow` doc, the dense tile grid, doc first sentences, README, the t-816 line, the ko CJK spacing.
- **Deferred** (backlog tasks): an autofit-aware ranking before lot 5; LibreOffice and PowerPoint witnesses; the `[pages]` message plural; the coupling of Typst upgrades to office2pdf; moving `import_reference`/`import_overflow` to a shared crate in lot 5; a neutral `color` module; `cargo audit` already red on `main` (no `.cargo/audit.toml`; new rustls advisories).

---

### Task 1: Reference witness crate: `office2pdf` to PDF, `hayro` rasterization

This task creates the crate `oku-slides-reference`: a reference witness that renders an imported `.pptx` to PDF with `office2pdf` (pure Rust, on Typst 0.15, behind the `ReferenceRenderer` trait) and rasterizes the PDF pages with `hayro`. Later tasks compare these rasters with Oku's own render to measure the visual gap. The witness keeps hidden slides (the import turns them visible, one page per slide) and prefers the fonts the caller passes over system fonts. It also adds the two workspace dependencies and extends the `deny.toml` advisory reasons for `quick-xml` 0.37 (pulled by `office2pdf`).

**Files:**
- Create: `crates/oku-slides-reference/Cargo.toml` (crate manifest, workspace deps only)
- Create: `crates/oku-slides-reference/src/lib.rs` (crate docs, re-exports, `ReferenceRenderer` trait)
- Create: `crates/oku-slides-reference/src/error.rs` (`ReferenceError`, `ReferenceErrorKind`, private Snafu `Inner`)
- Create: `crates/oku-slides-reference/src/office2pdf.rs` (the `Office2Pdf` witness)
- Create: `crates/oku-slides-reference/src/pdf.rs` (`ReferencePdf`, `MAX_RASTER_SIDE`, hayro rasterization)
- Create: `crates/oku-slides-reference/src/raster.rs` (`Raster`: RGBA image, crop, PNG in and out)
- Create: `crates/oku-slides-reference/ARCHITECTURE.md` (final English content, see the note below)
- Modify: `Cargo.toml` (workspace member dependency `oku-slides-reference`, plus `office2pdf` and `hayro`)
- Modify: `Cargo.lock` (generated, see Step 3)
- Modify: `deny.toml` (quick-xml advisory comment and reasons)
- Test: `crates/oku-slides-reference/src/office2pdf_tests.rs`, `crates/oku-slides-reference/src/pdf_tests.rs`, `crates/oku-slides-reference/src/raster_tests.rs`

**Interfaces:**
- Consumes: nothing new (the tests read `fixtures/import/libreoffice.pptx` and the fonts `assets/fonts/{inter-400,inter-700,carlito-400,liberation-sans-400}.ttf`, which already exist; workspace deps `png`, `snafu`, `speculoos`, `zip` already exist).
- Produces (all re-exported from the crate root):
  - `pub trait ReferenceRenderer: Debug + Send + Sync { fn witness(&self) -> &'static str; fn render(&self, pptx: &[u8], fonts: &[Vec<u8>]) -> Result<ReferencePdf, ReferenceError>; }`
  - `pub struct Office2Pdf` (`Debug, Clone, Copy, Default`), `witness()` returns `"office2pdf 0.8.1"`.
  - `pub struct ReferencePdf` with `new(bytes: Vec<u8>) -> Result<Self, ReferenceError>`, `page_count(&self) -> usize`, `page_size(&self, index: usize) -> Result<(f32, f32), ReferenceError>` (pt), `raster(&self, index: usize, width: u32) -> Result<Raster, ReferenceError>` (white background, opaque).
  - `pub const MAX_RASTER_SIDE: u32 = 4096`.
  - `pub struct Raster` (`Clone, PartialEq, Eq`) with `new(width: u32, height: u32, rgba: Vec<u8>)` (panics if the length is not `width * height * 4`), `width()`, `height()`, `rgba() -> &[u8]`, `crop(x, y, width, height) -> Raster` (bounded to the image), `from_png(&[u8]) -> Result<Raster, ReferenceError>`, `to_png(&self) -> Result<Vec<u8>, ReferenceError>`.
  - `pub struct ReferenceError` (opaque) with `kind() -> ReferenceErrorKind`; `pub enum ReferenceErrorKind { Render, Pdf, NoPage, Size, Png }`.

- [ ] **Step 1: Write the failing tests**

Create the three test files, and the manifests they need (the workspace `Cargo.toml` and the crate `Cargo.toml` are given in full in Step 3; add them now so the crate exists and the run fails on the missing sources). The hidden-slide test `render_hidden_slide_keeps_its_page` was checked while drafting to fail with `include_hidden_slides: false` (2 pages instead of 3), so it does guard that option.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/office2pdf_tests.rs
@@ -0,0 +1,65 @@
+use std::io::{Cursor, Read, Write};
+use std::path::Path;
+
+use speculoos::prelude::*;
+use zip::write::SimpleFileOptions;
+
+use super::*;
+use crate::ReferenceErrorKind;
+
+fn fixture(name: &str) -> Vec<u8> {
+    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import").join(name);
+    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
+}
+
+/// Les polices de l'app employées par les fixtures (Inter, Carlito, Liberation Sans).
+fn fonts() -> Vec<Vec<u8>> {
+    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
+    ["inter-400.ttf", "inter-700.ttf", "carlito-400.ttf", "liberation-sans-400.ttf"]
+        .iter()
+        .map(|f| std::fs::read(dir.join(f)).unwrap_or_else(|e| panic!("{f}: {e}")))
+        .collect()
+}
+
+/// `pptx` avec la slide `part` masquée (`show="0"`).
+fn hide(pptx: &[u8], part: &str) -> Vec<u8> {
+    let mut src = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
+    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
+    for i in 0..src.len() {
+        let mut f = src.by_index(i).unwrap();
+        let name = f.name().to_owned();
+        let mut bytes = Vec::new();
+        f.read_to_end(&mut bytes).unwrap();
+        if name == part {
+            let xml = String::from_utf8(bytes).unwrap().replacen("<p:sld ", "<p:sld show=\"0\" ", 1);
+            bytes = xml.into_bytes();
+        }
+        out.start_file(name, SimpleFileOptions::default()).unwrap();
+        out.write_all(&bytes).unwrap();
+    }
+    out.finish().unwrap().into_inner()
+}
+
+#[test]
+fn render_libreoffice_fixture_gives_one_page_per_slide() {
+    let pdf = Office2Pdf.render(&fixture("libreoffice.pptx"), &fonts()).unwrap();
+    assert_that!(pdf.page_count()).is_equal_to(3);
+}
+
+#[test]
+fn render_hidden_slide_keeps_its_page() {
+    let pptx = hide(&fixture("libreoffice.pptx"), "ppt/slides/slide2.xml");
+    let pdf = Office2Pdf.render(&pptx, &fonts()).unwrap();
+    assert_that!(pdf.page_count()).is_equal_to(3);
+}
+
+#[test]
+fn render_garbage_is_a_render_error() {
+    let err = Office2Pdf.render(b"not a zip", &[]).unwrap_err();
+    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
+}
+
+#[test]
+fn witness_names_the_crate_version() {
+    assert_that!(Office2Pdf.witness()).is_equal_to("office2pdf 0.8.1");
+}
```

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/pdf_tests.rs
@@ -0,0 +1,57 @@
+use std::path::Path;
+
+use speculoos::prelude::*;
+
+use super::*;
+use crate::{Office2Pdf, ReferenceErrorKind, ReferenceRenderer};
+
+/// Le PDF du témoin pour `libreoffice.pptx` (16:9, trois slides).
+fn libreoffice() -> ReferencePdf {
+    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
+    let pptx = std::fs::read(root.join("fixtures/import/libreoffice.pptx")).unwrap();
+    let font = std::fs::read(root.join("assets/fonts/carlito-400.ttf")).unwrap();
+    Office2Pdf.render(&pptx, &[font]).unwrap()
+}
+
+#[test]
+fn raster_keeps_the_page_ratio() {
+    let r = libreoffice().raster(0, 960).unwrap();
+    assert_that!((r.width(), r.height())).is_equal_to((960, 540));
+}
+
+#[test]
+fn raster_draws_the_page_content() {
+    let r = libreoffice().raster(0, 960).unwrap();
+    let not_white = r.rgba().as_chunks::<4>().0.iter().filter(|p| p[..3] != [255, 255, 255]).count();
+    assert_that!(not_white).is_greater_than(1000);
+}
+
+#[test]
+fn raster_is_opaque() {
+    let r = libreoffice().raster(1, 480).unwrap();
+    assert_that!(r.rgba().as_chunks::<4>().0.iter().all(|p| p[3] == 255)).is_true();
+}
+
+#[test]
+fn raster_beyond_last_page_is_no_page() {
+    let err = libreoffice().raster(3, 960).unwrap_err();
+    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::NoPage);
+}
+
+#[test]
+fn raster_too_wide_is_a_size_error() {
+    let err = libreoffice().raster(0, MAX_RASTER_SIDE + 1).unwrap_err();
+    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Size);
+}
+
+#[test]
+fn page_size_reads_the_points() {
+    let (w, h) = libreoffice().page_size(0).unwrap();
+    assert_that!((w.round(), h.round())).is_equal_to((960.0, 540.0));
+}
+
+#[test]
+fn new_without_pdf_is_a_pdf_error() {
+    let err = ReferencePdf::new(b"%PDF-garbage".to_vec()).unwrap_err();
+    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Pdf);
+}
```

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/raster_tests.rs
@@ -0,0 +1,33 @@
+use speculoos::prelude::*;
+
+use super::*;
+use crate::ReferenceErrorKind;
+
+/// Une image 3 × 2 : chaque pixel porte son rang dans sa composante rouge.
+fn ramp() -> Raster {
+    Raster::new(3, 2, (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect())
+}
+
+#[test]
+fn png_round_trip_keeps_pixels() {
+    let back = Raster::from_png(&ramp().to_png().unwrap()).unwrap();
+    assert_that!(back).is_equal_to(ramp());
+}
+
+#[test]
+fn crop_takes_the_rectangle() {
+    let c = ramp().crop(1, 1, 2, 1);
+    assert_that!((c.width(), c.height(), c.rgba().to_vec())).is_equal_to((2, 1, vec![4, 0, 0, 255, 5, 0, 0, 255]));
+}
+
+#[test]
+fn crop_beyond_the_image_is_bounded() {
+    let c = ramp().crop(2, 1, 10, 10);
+    assert_that!((c.width(), c.height())).is_equal_to((1, 1));
+}
+
+#[test]
+fn from_png_garbage_is_a_png_error() {
+    let err = Raster::from_png(b"nope").unwrap_err();
+    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Png);
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-reference`
Expected: cargo cannot build the package: `src/lib.rs` does not exist (`can't find library oku_slides_reference`). No test runs.

- [ ] **Step 3: Implement**

Workspace `Cargo.toml`: register the crate and add the two dependencies. `office2pdf` is taken without default features (no embedded `typst-kit` fonts: the caller passes its own); `hayro` is already pulled by `typst-render`.

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -24,6 +24,7 @@ oku-slides-mcp = { path = "crates/oku-slides-mcp" }
 oku-slides-agent = { path = "crates/oku-slides-agent" }
 oku-slides-render-worker = { path = "crates/oku-slides-render-worker" }
 oku-slides-import = { path = "crates/oku-slides-import" }
+oku-slides-reference = { path = "crates/oku-slides-reference" }
 # Agent : MCP 1.x (ère legacy ≤ 2025-11-25, celle des clients des agents ; voir écart 6) et ACP
 rust-mcp-sdk = "=1.1.0"
 rust-mcp-axum = "=1.0.2"
@@ -75,6 +76,11 @@ json-patch = "4"
 typst-layout = "0.15.1"
 # Aperçu des widgets recadré sur leur dessin (t-392) ; déjà tiré par typst-render.
 png = "0.18"
+# Témoin du rendu de référence de l'import (t-692, lot 4) : PPTX → PDF en Rust pur, sur Typst 0.15 ;
+# sans les polices embarquées de typst-kit (l'appelant passe les siennes).
+office2pdf = { version = "0.8.1", default-features = false }
+# Rastérisation des pages du PDF de référence ; déjà tiré par typst-render.
+hayro = { version = "0.7.1", default-features = false }
 # Serveur de développement (`oku-slides serve`, t-419) ; déjà tiré par rust-mcp-axum.
 axum = { version = "0.8", default-features = false, features = ["http1", "tokio"] }
 # Corps de réponse SSE en flux (`Frame`, que axum ne ré-exporte pas).
```

Crate manifest.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/Cargo.toml
@@ -0,0 +1,21 @@
+[package]
+name = "oku-slides-reference"
+description = "Rendu de référence d'une présentation importée (témoin office2pdf) et rastérisation de ses pages."
+version.workspace = true
+edition.workspace = true
+rust-version.workspace = true
+authors.workspace = true
+publish.workspace = true
+
+[dependencies]
+office2pdf.workspace = true
+hayro.workspace = true
+png.workspace = true
+snafu.workspace = true
+
+[dev-dependencies]
+speculoos.workspace = true
+zip.workspace = true
+
+[lints]
+workspace = true
```

Errors: opaque `ReferenceError` carrying a private Snafu `Inner`, with a backtrace in every variant; `kind()` is what the caller localizes.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/error.rs
@@ -0,0 +1,60 @@
+//! Erreurs du rendu de référence : opaques, avec leur nature
+//! ([`ReferenceErrorKind`]) pour le message que l'appelant localise.
+
+use snafu::{Backtrace, Snafu};
+
+/// Nature d'une erreur du rendu de référence.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum ReferenceErrorKind {
+    /// Le témoin n'a pas pu rendre la présentation.
+    Render,
+    /// Le PDF rendu est illisible, ou n'a aucune page.
+    Pdf,
+    /// La page demandée n'existe pas.
+    NoPage,
+    /// L'image demandée dépasse [`crate::MAX_RASTER_SIDE`] ou est vide.
+    Size,
+    /// Un PNG illisible ou impossible à écrire.
+    Png,
+}
+
+/// Une erreur du rendu de référence.
+#[derive(Debug, Snafu)]
+#[snafu(display("{inner}"))]
+pub struct ReferenceError {
+    inner: Inner,
+}
+
+impl ReferenceError {
+    /// Sa nature.
+    pub fn kind(&self) -> ReferenceErrorKind {
+        match self.inner {
+            Inner::Render { .. } => ReferenceErrorKind::Render,
+            Inner::Pdf { .. } => ReferenceErrorKind::Pdf,
+            Inner::NoPage { .. } => ReferenceErrorKind::NoPage,
+            Inner::Size { .. } => ReferenceErrorKind::Size,
+            Inner::Png { .. } => ReferenceErrorKind::Png,
+        }
+    }
+}
+
+impl From<Inner> for ReferenceError {
+    fn from(inner: Inner) -> Self {
+        Self { inner }
+    }
+}
+
+#[derive(Debug, Snafu)]
+#[snafu(visibility(pub(crate)))]
+pub(crate) enum Inner {
+    #[snafu(display("the witness could not render the presentation: {message}"))]
+    Render { message: String, backtrace: Backtrace },
+    #[snafu(display("unreadable reference pdf: {message}"))]
+    Pdf { message: String, backtrace: Backtrace },
+    #[snafu(display("no page {index} in the reference pdf"))]
+    NoPage { index: usize, backtrace: Backtrace },
+    #[snafu(display("raster size out of range: {width} x {height}"))]
+    Size { width: u32, height: u32, backtrace: Backtrace },
+    #[snafu(display("png: {message}"))]
+    Png { message: String, backtrace: Backtrace },
+}
```

Crate root: module list, re-exports and the `ReferenceRenderer` trait.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/lib.rs
@@ -0,0 +1,40 @@
+//! Rendu de référence d'une présentation importée : un témoin indépendant
+//! d'Oku rend le `.pptx` en PDF, dont les pages sont rastérisées pour être
+//! comparées au rendu Oku (écart visuel du hi-fi, `oku_slides_import::metrics`).
+//!
+//! - [`ReferenceRenderer`] : un témoin (`.pptx` + polices → PDF) ;
+//!   [`Office2Pdf`] est le seul livré (crate `office2pdf`, Rust pur, sur Typst).
+//! - [`ReferencePdf`] : le PDF rendu, ses pages, rastérisées par `hayro`
+//!   ([`ReferencePdf::raster`]) à une largeur donnée.
+//! - [`Raster`] : une image RGBA 8 bits opaque ; [`Raster::from_png`],
+//!   [`Raster::to_png`] (le tube du processus de rendu transporte des PNG).
+//! - [`ReferenceError`], [`ReferenceErrorKind`] : opaques ; jamais fatals pour
+//!   l'import, qui dit au rapport que la référence manque.
+//!
+//! Le témoin lit un fichier non fiable sans borne de temps ni de mémoire :
+//! l'appelant le fait tourner dans le processus de rendu
+//! (`oku-slides-render-worker`), qui le borne.
+
+mod error;
+mod office2pdf;
+mod pdf;
+mod raster;
+
+pub use error::{ReferenceError, ReferenceErrorKind};
+pub use office2pdf::Office2Pdf;
+pub use pdf::{MAX_RASTER_SIDE, ReferencePdf};
+pub use raster::Raster;
+
+/// Un témoin : rend une présentation en PDF, une page par slide.
+///
+/// Implement this trait to add a reference witness (LibreOffice, PowerPoint…).
+/// Le PDF rendu doit garder les slides masquées (l'import les reprend
+/// visibles) et utiliser les polices fournies en priorité : l'écart mesure la
+/// mise en page, pas la substitution des polices.
+pub trait ReferenceRenderer: std::fmt::Debug + Send + Sync {
+    /// Nom et version du témoin, tel que le rapport l'écrit (`office2pdf 0.8.1`).
+    fn witness(&self) -> &'static str;
+
+    /// Le PDF de `pptx`, rendu avec `fonts` (octets de fichiers de police).
+    fn render(&self, pptx: &[u8], fonts: &[Vec<u8>]) -> Result<ReferencePdf, ReferenceError>;
+}
```

The `office2pdf` witness: `include_hidden_slides: true` and the provided fonts passed as `font_bytes`.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/office2pdf.rs
@@ -0,0 +1,31 @@
+//! Le témoin `office2pdf` : la présentation en PDF par le crate du même nom
+//! (Rust pur, sur Typst 0.15), sans LibreOffice.
+//!
+//! Les polices fournies passent avant toutes les autres ; celles du système
+//! restent un repli (le crate les cherche toujours, hors WASM). Les slides
+//! masquées sont rendues : l'import les reprend visibles, une page par slide.
+
+use crate::error::RenderSnafu;
+use crate::{ReferenceError, ReferencePdf, ReferenceRenderer};
+use office2pdf::config::{ConvertOptions, Format};
+
+/// Le témoin `office2pdf`.
+#[derive(Debug, Clone, Copy, Default)]
+pub struct Office2Pdf;
+
+impl ReferenceRenderer for Office2Pdf {
+    fn witness(&self) -> &'static str {
+        "office2pdf 0.8.1"
+    }
+
+    fn render(&self, pptx: &[u8], fonts: &[Vec<u8>]) -> Result<ReferencePdf, ReferenceError> {
+        let options = ConvertOptions { include_hidden_slides: true, font_bytes: fonts.to_vec(), ..Default::default() };
+        let converted = office2pdf::convert_bytes(pptx, Format::Pptx, &options)
+            .map_err(|e| ReferenceError::from(RenderSnafu { message: e.to_string() }.build()))?;
+        ReferencePdf::new(converted.pdf)
+    }
+}
+
+#[cfg(test)]
+#[path = "office2pdf_tests.rs"]
+mod office2pdf_tests;
```

The reference PDF and its rasterization with `hayro` (white opaque background, side capped at `MAX_RASTER_SIDE`).

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/pdf.rs
@@ -0,0 +1,73 @@
+//! Le PDF d'un témoin et la rastérisation de ses pages par `hayro`.
+
+use std::sync::Arc;
+
+use hayro::hayro_interpret::InterpreterSettings;
+use hayro::hayro_syntax::Pdf;
+use hayro::vello_cpu::color::palette::css::WHITE;
+use hayro::{RenderCache, RenderSettings};
+use snafu::ensure;
+
+use crate::error::{NoPageSnafu, PdfSnafu, SizeSnafu};
+use crate::{Raster, ReferenceError};
+
+/// Côté maximal, en pixels, d'une page rastérisée.
+pub const MAX_RASTER_SIDE: u32 = 4096;
+
+/// Le PDF rendu par un témoin, lu une fois.
+pub struct ReferencePdf {
+    pdf: Pdf,
+}
+
+impl std::fmt::Debug for ReferencePdf {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        f.debug_struct("ReferencePdf").field("pages", &self.page_count()).finish()
+    }
+}
+
+impl ReferencePdf {
+    /// Lit `bytes` ; un PDF illisible ou sans page est une erreur.
+    pub fn new(bytes: Vec<u8>) -> Result<Self, ReferenceError> {
+        let pdf = Pdf::new(Arc::new(bytes))
+            .map_err(|e| ReferenceError::from(PdfSnafu { message: format!("{e:?}") }.build()))?;
+        ensure!(!pdf.pages().is_empty(), PdfSnafu { message: "no page" });
+        Ok(Self { pdf })
+    }
+
+    /// Nombre de pages.
+    pub fn page_count(&self) -> usize {
+        self.pdf.pages().len()
+    }
+
+    /// Taille de la page `index` (à partir de 0), en pt : largeur, hauteur.
+    pub fn page_size(&self, index: usize) -> Result<(f32, f32), ReferenceError> {
+        let page = self.pdf.pages().get(index).ok_or_else(|| NoPageSnafu { index }.build())?;
+        Ok(page.render_dimensions())
+    }
+
+    /// La page `index` sur fond blanc, à `width` pixels de large (la hauteur
+    /// suit le ratio de la page, arrondie).
+    pub fn raster(&self, index: usize, width: u32) -> Result<Raster, ReferenceError> {
+        let page = self.pdf.pages().get(index).ok_or_else(|| NoPageSnafu { index }.build())?;
+        let (w, h) = page.render_dimensions();
+        let scale = width as f32 / w;
+        let height = (h * scale).round();
+        let fits = |side: f32| side.is_finite() && side >= 1.0 && side <= MAX_RASTER_SIDE as f32;
+        ensure!(fits(width as f32) && fits(height), SizeSnafu { width, height: height.max(0.0) as u32 });
+        let height = height as u32;
+        let settings = RenderSettings {
+            x_scale: scale,
+            y_scale: scale,
+            width: u16::try_from(width).ok(),
+            height: u16::try_from(height).ok(),
+            bg_color: WHITE,
+        };
+        let pixmap = hayro::render(page, &RenderCache::new(), &InterpreterSettings::default(), &settings);
+        // Fond blanc opaque : les composantes prémultipliées sont les composantes droites.
+        Ok(Raster::new(width, height, pixmap.data_as_u8_slice().to_vec()))
+    }
+}
+
+#[cfg(test)]
+#[path = "pdf_tests.rs"]
+mod pdf_tests;
```

The RGBA image and its PNG conversions.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/src/raster.rs
@@ -0,0 +1,101 @@
+//! Une image RGBA 8 bits, opaque, et son passage en PNG.
+
+use snafu::ensure;
+
+use crate::error::{PngSnafu, SizeSnafu};
+use crate::{MAX_RASTER_SIDE, ReferenceError};
+
+/// Une image RGBA 8 bits, ligne à ligne depuis le haut.
+#[derive(Clone, PartialEq, Eq)]
+pub struct Raster {
+    width: u32,
+    height: u32,
+    rgba: Vec<u8>,
+}
+
+impl std::fmt::Debug for Raster {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        f.debug_struct("Raster").field("width", &self.width).field("height", &self.height).finish_non_exhaustive()
+    }
+}
+
+impl Raster {
+    /// Une image de `width` × `height` pixels ; `rgba` en a exactement 4 octets par pixel.
+    ///
+    /// # Panics
+    ///
+    /// Si `rgba` n'a pas la taille de l'image (un bogue de l'appelant).
+    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
+        assert_eq!(rgba.len(), width as usize * height as usize * 4, "taille RGBA");
+        Self { width, height, rgba }
+    }
+
+    /// Largeur, en pixels.
+    pub fn width(&self) -> u32 {
+        self.width
+    }
+
+    /// Hauteur, en pixels.
+    pub fn height(&self) -> u32 {
+        self.height
+    }
+
+    /// Les octets RGBA.
+    pub fn rgba(&self) -> &[u8] {
+        &self.rgba
+    }
+
+    /// Le rectangle `x, y, width, height` de l'image, borné à l'image.
+    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
+        let (x, y) = (x.min(self.width), y.min(self.height));
+        let (width, height) = (width.min(self.width - x), height.min(self.height - y));
+        let row = self.width as usize * 4;
+        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
+        for r in y..y + height {
+            let start = r as usize * row + x as usize * 4;
+            rgba.extend_from_slice(&self.rgba[start..start + width as usize * 4]);
+        }
+        Self { width, height, rgba }
+    }
+
+    /// Lit un PNG (RGB ou RGBA 8 bits, couleurs indexées ou gris développés),
+    /// d'au plus [`MAX_RASTER_SIDE`] pixels de côté.
+    pub fn from_png(bytes: &[u8]) -> Result<Self, ReferenceError> {
+        let png_err = |e: png::DecodingError| ReferenceError::from(PngSnafu { message: e.to_string() }.build());
+        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
+        decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
+        let mut reader = decoder.read_info().map_err(png_err)?;
+        let (width, height) = (reader.info().width, reader.info().height);
+        ensure!(
+            (1..=MAX_RASTER_SIDE).contains(&width) && (1..=MAX_RASTER_SIDE).contains(&height),
+            SizeSnafu { width, height }
+        );
+        let mut buf = vec![0; reader.output_buffer_size().unwrap_or_default()];
+        let info = reader.next_frame(&mut buf).map_err(png_err)?;
+        buf.truncate(info.buffer_size());
+        let rgba = match info.color_type {
+            png::ColorType::Rgba => buf,
+            png::ColorType::GrayscaleAlpha => buf.as_chunks::<2>().0.iter().flat_map(|&[g, a]| [g, g, g, a]).collect(),
+            other => return Err(PngSnafu { message: format!("unexpected color type {other:?}") }.build().into()),
+        };
+        ensure!(rgba.len() == width as usize * height as usize * 4, PngSnafu { message: "truncated image" });
+        Ok(Self { width, height, rgba })
+    }
+
+    /// L'image en PNG RGBA 8 bits.
+    pub fn to_png(&self) -> Result<Vec<u8>, ReferenceError> {
+        let png_err = |e: png::EncodingError| ReferenceError::from(PngSnafu { message: e.to_string() }.build());
+        let mut out = Vec::new();
+        let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
+        encoder.set_color(png::ColorType::Rgba);
+        encoder.set_depth(png::BitDepth::Eight);
+        let mut writer = encoder.write_header().map_err(png_err)?;
+        writer.write_image_data(&self.rgba).map_err(png_err)?;
+        writer.finish().map_err(png_err)?;
+        Ok(out)
+    }
+}
+
+#[cfg(test)]
+#[path = "raster_tests.rs"]
+mod raster_tests;
```

`deny.toml`: the `office2pdf` dependency tree brings `quick-xml` 0.37 (via `umya-spreadsheet`, XLSX only), covered by the same two advisories already ignored for 0.38.

```diff
--- a/deny.toml
+++ b/deny.toml
@@ -4,12 +4,14 @@
 all-features = true
 
 [advisories]
-# RUSTSEC-2026-0194/0195 (quick-xml 0.38, via typst-library → hayagriva →
-# citationberg) : lecture de styles CSL de bibliographie. Oku Slides ne donne jamais
-# de fichier de bibliographie à Typst ; à retirer quand Typst suivra.
+# RUSTSEC-2026-0194/0195 (quick-xml) : 0.38, via typst-library → hayagriva →
+# citationberg, lecture de styles CSL de bibliographie (Oku Slides ne donne jamais de
+# fichier de bibliographie à Typst) ; 0.37, via office2pdf → umya-spreadsheet, lecture
+# des classeurs XLSX (le témoin de l'import ne reçoit que des PPTX). À retirer quand
+# Typst et office2pdf suivront.
 ignore = [
-    { id = "RUSTSEC-2026-0194", reason = "quick-xml via les bibliographies de Typst, jamais atteint par Oku Slides" },
-    { id = "RUSTSEC-2026-0195", reason = "quick-xml via les bibliographies de Typst, jamais atteint par Oku Slides" },
+    { id = "RUSTSEC-2026-0194", reason = "quick-xml via les bibliographies de Typst et le XLSX d'office2pdf, jamais atteints par Oku Slides" },
+    { id = "RUSTSEC-2026-0195", reason = "quick-xml via les bibliographies de Typst et le XLSX d'office2pdf, jamais atteints par Oku Slides" },
 ]
 unmaintained = "workspace"
 
```

`Cargo.lock`: do not paste the diff. Let cargo update the lock file (`CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo build -p oku-slides-reference`), then check that `git diff Cargo.lock` only adds the packages named here (and adds a second `aes` 0.8.4 / `cipher` 0.4.4 next to the existing 0.9.2 / 0.5.2, so some existing dependency entries gain a version suffix such as `"cipher 0.5.2"`): `aes`, `aligned`, `aligned-vec`, `arg_enum_proc_macro`, `as-slice`, `av-scenechange`, `av1-grain`, `avif-serialize`, `bitstream-io`, `block-padding`, `built`, `cbc`, `cfb`, `cipher`, `dirs`, `doc-comment`, `docx-rs`, `equator`, `equator-macro`, `fancy-regex`, `fax`, `hmac`, `html_parser`, `imgref`, `inout`, `interpolate_name`, `libfuzzer-sys`, `loop9`, `maybe-rayon`, `md-5`, `no_std_io2`, `noop_proc_macro`, `office2pdf`, `oku-slides-reference`, `pastey`, `profiling`, `profiling-procmacros`, `quick-xml`, `rav1e`, `ravif`, `simd_helpers`, `thousands`, `tiff`, `typst-kit`, `umya-spreadsheet`, `v_frame`, `y4m`, `zip`. Exact versions may differ slightly if the registry moved; `office2pdf` must resolve to 0.8.x and `hayro` to 0.7.x. The lock file must also keep the workspace version of `main` (2.1.0) for the internal crates.

`crates/oku-slides-reference/ARCHITECTURE.md`: in the draft this file was first written in French; here it is created directly with its final English content (taken from `f86a6c84`), and Task 7 does not need to rewrite it for language.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/ARCHITECTURE.md
@@ -0,0 +1,39 @@
+# oku-slides-reference
+
+Reference rendering of an imported presentation: a witness independent of Oku renders the
+`.pptx` to PDF, whose pages are rasterized and compared with the Oku rendering (hi-fi visual
+gap, `oku_slides_import::metrics`, t-692 lot 4).
+
+## API
+
+- `ReferenceRenderer` (trait): `witness()` (name and version written to the report),
+  `render(pptx, fonts) -> ReferencePdf`. `Office2Pdf` is the only witness shipped.
+- `ReferencePdf`: `page_count`, `page_size(i)` (pt), `raster(i, width) -> Raster` through
+  `hayro`, on white, sides at most `MAX_RASTER_SIDE`.
+- `Raster`: opaque 8-bit RGBA; `crop`, `from_png`, `to_png` (the render worker pipe carries PNGs).
+- `ReferenceError` opaque, `kind() -> ReferenceErrorKind` (`Render`, `Pdf`, `NoPage`, `Size`,
+  `Png`): never fatal for the import, whose report says the reference is missing.
+
+## Isolation
+
+The witness reads an untrusted file without bounds of its own: it runs in the render worker
+(`oku-slides-render-worker`, `Reference` and `ReferencePng` requests), bounded in time and memory.
+
+## Known limits (`office2pdf` witness)
+
+- It shrinks `normAutofit` text without `fontScale` (PowerPoint keeps 100 %), draws charts
+  roughly, SmartArt text too small, and the alto titles about 1 % wider than Oku and LibreOffice.
+- It shares Typst's shaping and line breaking with Oku: a PowerPoint line-break gap (t-863)
+  does not show.
+- The fonts given come first; the system fonts stay its fallback (outside WASM).
+- An import behavior copied from office2pdf (Apache-2.0, credited in the file) is no longer
+  measured by it: check it against another witness or against the OOXML.
+- It also pulls the crate's DOCX and XLSX readers (`docx-rs`, `umya-spreadsheet`), never called.
+
+## Extension points
+
+- New witness (LibreOffice, PowerPoint): an implementation of `ReferenceRenderer`.
+
+## Dependencies
+
+`office2pdf` (without `embedded-fonts`), `hayro`, `png`, `snafu`.
```

- [ ] **Step 4: Run the tests and the checks**

Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-reference`
Expected: 15 tests pass (4 in `office2pdf_tests`, 7 in `pdf_tests`, 4 in `raster_tests`), none skipped.

Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-reference --all-targets -- -D warnings`
Expected: no warnings.

Run: `cargo fmt -p oku-slides-reference -- --check`
Expected: no diff.

Run: `cargo deny check advisories`
Expected: passes (the quick-xml advisories are ignored with the updated reasons).

- [ ] **Step 5: Commit**

```bash
git add Cargo.lock Cargo.toml deny.toml crates/oku-slides-reference/ARCHITECTURE.md crates/oku-slides-reference/Cargo.toml crates/oku-slides-reference/src/error.rs crates/oku-slides-reference/src/lib.rs crates/oku-slides-reference/src/office2pdf.rs crates/oku-slides-reference/src/office2pdf_tests.rs crates/oku-slides-reference/src/pdf.rs crates/oku-slides-reference/src/pdf_tests.rs crates/oku-slides-reference/src/raster.rs crates/oku-slides-reference/src/raster_tests.rs
```

Write the message to a file (for example `/tmp/commit-msg-t1.txt`) containing exactly:

```
feat(reference) : témoin office2pdf du rendu de référence et rastérisation hayro (t-692)
```

```bash
git commit -F /tmp/commit-msg-t1.txt
```

**Extracting from the draft:** instead of retyping, take the crate files from the draft commit: `git checkout 98506223 -- crates/oku-slides-reference/Cargo.toml crates/oku-slides-reference/src/error.rs crates/oku-slides-reference/src/lib.rs crates/oku-slides-reference/src/office2pdf.rs crates/oku-slides-reference/src/office2pdf_tests.rs crates/oku-slides-reference/src/pdf.rs crates/oku-slides-reference/src/pdf_tests.rs crates/oku-slides-reference/src/raster.rs crates/oku-slides-reference/src/raster_tests.rs deny.toml` (run it from the implementer's worktree, with the draft worktree path replaced by any checkout that has the commit). `deny.toml` is unchanged on `main` since the draft base `53454df2`, so a plain checkout is fine. `Cargo.toml` and `Cargo.lock` were changed on `main` since `53454df2` (version bump to 2.1.0), so apply them with `git show 98506223 -- Cargo.toml | git apply --3way` and let cargo regenerate `Cargo.lock` rather than applying the draft lock diff (if you do apply it: `git show 98506223 -- Cargo.lock | git apply --3way`, then run `cargo build -p oku-slides-reference`). `crates/oku-slides-reference/ARCHITECTURE.md` is not extracted from `98506223` (French draft): take the English version with `git show f86a6c84:crates/oku-slides-reference/ARCHITECTURE.md > crates/oku-slides-reference/ARCHITECTURE.md`.

### Task 2: Render worker: reference requests and `RemoteReference`

The render worker learns to render an imported `.pptx` with the witness (`Office2Pdf`, from Task 1's `oku_slides_reference` crate) inside the bounded render process, so the untrusted file is read under the render timeout and memory limits. New protocol messages `Reference`, `ReferencePng` and `ReleaseReference` (and `Response::Reference`) let the worker keep the rendered PDFs by id; the client module `reference.rs` exposes `RenderWorker::reference` and `RemoteReference` (PNG of a page on demand, epoch check giving `Stale` after a worker restart, release queued in the deferred mutex on `Drop`). The font upload loop of `open_slot` is factored into `put_fonts` so both paths share it.

**Files:**
- Create: `crates/oku-slides-render-worker/src/reference.rs` (client side: `RenderWorker::reference`, `RemoteReference`)
- Create: `crates/oku-slides-render-worker/src/reference_tests.rs` (tests of the client side against the in-process worker)
- Modify: `crates/oku-slides-render-worker/Cargo.toml` (dependency on `oku-slides-reference`, also in dev-dependencies)
- Modify: `crates/oku-slides-render-worker/src/client.rs` (`Deferred::ReleaseReference`, `put_fonts` factored out of `open_slot`)
- Modify: `crates/oku-slides-render-worker/src/frame.rs` (frame limits for reference PNGs)
- Modify: `crates/oku-slides-render-worker/src/lib.rs` (module `reference`, export `RemoteReference`)
- Modify: `crates/oku-slides-render-worker/src/protocol.rs` (new `Request` and `Response` variants)
- Modify: `crates/oku-slides-render-worker/src/serve.rs` (serve branches, `references` map)
- Modify: `Cargo.lock` (new dependency edge)
- `crates/oku-slides-render-worker/ARCHITECTURE.md`: updated in Task 7 (English).
- Test: `crates/oku-slides-render-worker/src/serve_tests.rs`, `crates/oku-slides-render-worker/src/reference_tests.rs`, `crates/oku-slides-render-worker/src/client_tests.rs`

**Interfaces:**
- Consumes: from Task 1, the crate `oku-slides-reference` (already declared in `[workspace.dependencies]`): `Office2Pdf` (unit struct), the trait `ReferenceRenderer` (`render(&self, pptx: &[u8], fonts: &[Vec<u8>]) -> Result<ReferencePdf, ReferenceError>`, `witness(&self) -> &str`, in scope through the `use`), `ReferencePdf` (`page_count()`, `page_size(i) -> Result<(f32, f32), ReferenceError>`, `raster(page, width) -> Result<Raster, ReferenceError>`), `Raster` (`to_png()`, `from_png()`, `width()`, `height()`), `ReferenceError` (`Display`). The fixtures `fixtures/import/libreoffice.pptx` (3 pages, 960x540 pt) and `assets/fonts/carlito-400.ttf` must exist; the witness string is `office2pdf 0.8.1`.
- Produces: `RenderWorker::reference(&self, pptx: &[u8], fonts: &Fonts) -> Result<RemoteReference, RenderError>`; `RemoteReference` (public, `Debug`, `Send`, `Drop` queues the release) with `witness(&self) -> &str`, `page_count(&self) -> usize`, `page_size(&self, index: usize) -> Option<(f32, f32)>` (pt), `png(&self, index: usize, width: u32) -> Result<Vec<u8>, RenderError>` (`Stale` after a worker restart); re-exported as `oku_slides_render_worker::RemoteReference`. Wire: `Request::Reference { fonts: Vec<String> }` (payload = the `.pptx`), `Request::ReferencePng { reference: u64, page: usize, width: u32 }`, `Request::ReleaseReference { reference: u64 }`, `Response::Reference { reference: u64, witness: String, pages: Vec<(f32, f32)> }`. A failing witness answers `Error Setup` with a message starting `reference: `; an unknown reference answers `Error Stale`. Reference ids share the generation counter (`next_generation`).

- [ ] **Step 1: Write the failing tests**

Server-side tests:

```diff
--- a/crates/oku-slides-render-worker/src/serve_tests.rs
+++ b/crates/oku-slides-render-worker/src/serve_tests.rs
@@ -229,3 +229,41 @@ fn serve_refuses_a_request_header_over_an_eighth_of_the_frame_size() {
     // 1024 octets par trame : l'en-tête (plus de 200) dépasse 1024 / 8.
     assert_that!(serve(input.as_slice(), Vec::new(), 1024)).is_err();
 }
+
+fn libreoffice_pptx() -> Vec<u8> {
+    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/libreoffice.pptx")).unwrap()
+}
+
+#[test]
+fn serve_reference_then_png_returns_a_png() {
+    let mut h = Harness::new();
+    let reference = match h.call(&Request::Reference { fonts: vec![] }, &libreoffice_pptx()).0 {
+        Response::Reference { reference, pages, .. } => {
+            assert_that!(pages.len()).is_equal_to(3);
+            reference
+        }
+        other => panic!("{other:?}"),
+    };
+    let (resp, png) = h.call(&Request::ReferencePng { reference, page: 0, width: 480 }, &[]);
+    assert_that!(matches!(resp, Response::Bytes)).is_true();
+    assert_that!(png[1..4].to_vec()).is_equal_to(b"PNG".to_vec());
+}
+
+#[test]
+fn serve_png_of_released_reference_returns_stale() {
+    let mut h = Harness::new();
+    let Response::Reference { reference, .. } = h.call(&Request::Reference { fonts: vec![] }, &libreoffice_pptx()).0
+    else {
+        panic!("pas de référence")
+    };
+    h.call(&Request::ReleaseReference { reference }, &[]);
+    let (resp, _) = h.call(&Request::ReferencePng { reference, page: 0, width: 480 }, &[]);
+    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Stale })).is_true();
+}
+
+#[test]
+fn serve_reference_with_unknown_font_returns_setup() {
+    let mut h = Harness::new();
+    let (resp, _) = h.call(&Request::Reference { fonts: vec!["inconnue".into()] }, &libreoffice_pptx());
+    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
+}
```

Client-side tests (new file):

```diff
new file mode 100644
--- /dev/null
+++ b/crates/oku-slides-render-worker/src/reference_tests.rs
@@ -0,0 +1,73 @@
+use std::path::Path;
+
+use oku_slides_platform::RenderLaunch;
+use oku_slides_platform::config::RenderSettings;
+use oku_slides_render::{Fonts, RenderErrorKind};
+use speculoos::prelude::*;
+
+use crate::RenderWorker;
+
+fn worker(timeout_ms: u64) -> RenderWorker {
+    RenderWorker::new(
+        RenderLaunch::InProcess,
+        RenderSettings {
+            timeout_ms,
+            probe_budget_ms: timeout_ms,
+            memory_mib: 4096,
+            max_frame_mib: 64,
+            max_widget_kib: 256,
+            max_widgets: 64,
+        },
+    )
+}
+
+fn root() -> &'static Path {
+    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
+}
+
+fn pptx() -> Vec<u8> {
+    std::fs::read(root().join("fixtures/import/libreoffice.pptx")).unwrap()
+}
+
+fn fonts() -> Fonts {
+    Fonts::new([std::fs::read(root().join("assets/fonts/carlito-400.ttf")).unwrap()])
+}
+
+#[test]
+fn reference_gives_the_witness_and_its_pages() {
+    let r = worker(30_000).reference(&pptx(), &fonts()).unwrap();
+    assert_that!((r.witness(), r.page_count())).is_equal_to(("office2pdf 0.8.1", 3));
+}
+
+#[test]
+fn reference_png_has_the_requested_width() {
+    let r = worker(30_000).reference(&pptx(), &fonts()).unwrap();
+    let png = oku_slides_reference::Raster::from_png(&r.png(0, 960).unwrap()).unwrap();
+    assert_that!((png.width(), png.height())).is_equal_to((960, 540));
+}
+
+#[test]
+fn reference_page_size_is_in_points() {
+    let r = worker(30_000).reference(&pptx(), &fonts()).unwrap();
+    let (w, h) = r.page_size(0).unwrap();
+    assert_that!((w.round(), h.round())).is_equal_to((960.0, 540.0));
+}
+
+#[test]
+fn reference_of_garbage_is_an_error_and_the_worker_goes_on() {
+    let w = worker(30_000);
+    assert_that!(w.reference(b"not a pptx", &fonts()).is_err()).is_true();
+    assert_that!(w.reference(&pptx(), &fonts()).is_ok()).is_true();
+}
+
+#[test]
+fn reference_beyond_the_timeout_is_a_timeout() {
+    let err = worker(1).reference(&pptx(), &fonts()).unwrap_err();
+    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
+}
+
+#[test]
+fn remote_reference_is_send() {
+    fn send<T: Send>() {}
+    send::<crate::RemoteReference>();
+}
```

Restart test (uses the existing `force_restart` helper of `client_tests.rs`):

```diff
--- a/crates/oku-slides-render-worker/src/client_tests.rs
+++ b/crates/oku-slides-render-worker/src/client_tests.rs
@@ -121,6 +121,15 @@ fn remote_compiled_of_previous_worker_is_stale_and_its_release_is_dropped() {
     assert_that!(w.link().connected()).is_false();
 }
 
+#[test]
+fn remote_reference_of_previous_worker_is_stale() {
+    let w = worker(3_000);
+    let pptx = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/libreoffice.pptx")).unwrap();
+    let old = w.reference(&pptx, &Fonts::default()).unwrap();
+    force_restart(&w, "boucle-r");
+    assert_that!(old.png(0, 480).unwrap_err().kind()).is_equal_to(RenderErrorKind::Stale);
+}
+
 #[test]
 fn set_widgets_with_faulty_pack_is_disabled_without_call() {
     let w = worker(3_000);
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-render-worker`
Expected: compile errors naming the missing items (`Request::Reference`, `Response::Reference`, `RenderWorker::reference`, `RemoteReference`; `reference_tests` is not declared yet, so the `serve_tests` and `client_tests` errors come first).

- [ ] **Step 3: Implement**

Dependency: the worker crate depends on `oku-slides-reference` (normal and dev dependency, as in the draft: the tests use `Raster`).

```diff
--- a/crates/oku-slides-render-worker/Cargo.toml
+++ b/crates/oku-slides-render-worker/Cargo.toml
@@ -17,6 +17,7 @@ doc = false
 oku-slides-core.workspace = true
 oku-slides-i18n.workspace = true
 oku-slides-platform.workspace = true
+oku-slides-reference.workspace = true
 oku-slides-render.workspace = true
 serde.workspace = true
 # `float_roundtrip` : les coordonnées de la mise en page traversent le tube au bit près.
@@ -34,6 +35,7 @@ libc.workspace = true
 [dev-dependencies]
 speculoos.workspace = true
 rstest.workspace = true
+oku-slides-reference.workspace = true
 
 [lints]
 workspace = true
```

`Cargo.lock`: do not paste the diff. Let cargo update the lock file (`cargo build`), then check that `git diff Cargo.lock` only adds the line `"oku-slides-reference",` to the dependencies of the `oku-slides-render-worker` package (no new package).

Wire protocol: three requests and one response.

```diff
--- a/crates/oku-slides-render-worker/src/protocol.rs
+++ b/crates/oku-slides-render-worker/src/protocol.rs
@@ -76,6 +76,19 @@ pub(crate) enum Request {
         slot: SlotId,
         generations: Vec<u64>,
     },
+    /// Rend la présentation de la charge (`.pptx`) par le témoin, avec ces polices.
+    Reference {
+        fonts: Vec<String>,
+    },
+    /// La page `page` d'une référence, en PNG de `width` pixels de large.
+    ReferencePng {
+        reference: u64,
+        page: usize,
+        width: u32,
+    },
+    ReleaseReference {
+        reference: u64,
+    },
 }
 
 /// Ce que le travailleur dit d'un rendu réussi.
@@ -102,6 +115,12 @@ pub(crate) enum Response {
     Compiled(Box<CompiledHeader>),
     /// Charge : octets du résultat.
     Bytes,
+    /// Une référence rendue : son identifiant, le témoin et la taille de chaque page (pt).
+    Reference {
+        reference: u64,
+        witness: String,
+        pages: Vec<(f32, f32)>,
+    },
     Error {
         error: oku_slides_render::WireError,
     },
```

Frame limits: a reference PNG may take the whole frame size, and a request payload now includes the `.pptx`.

```diff
--- a/crates/oku-slides-render-worker/src/frame.rs
+++ b/crates/oku-slides-render-worker/src/frame.rs
@@ -3,7 +3,7 @@
 //!
 //! Bornes par partie et par sorte ([`FrameLimits`]), tirées de `max_frame_mib` :
 //! un en-tête (JSON : deck, pack, mise en page) au plus un huitième, un SVG au
-//! plus un quart, une police, un asset, un PNG ou un PDF la taille entière ;
+//! plus un quart, une police, un asset, un `.pptx`, un PNG ou un PDF la taille entière ;
 //! les autres réponses n'ont pas de charge.
 
 use std::io::{Read, Write};
@@ -32,17 +32,17 @@ impl FrameLimits {
         Self { header: max, payload: max }
     }
 
-    /// Une requête de l'hôte : l'en-tête au plus `max / 8`, la charge (police, asset) `max`.
+    /// Une requête de l'hôte : l'en-tête au plus `max / 8`, la charge (police, asset, `.pptx`) `max`.
     pub(crate) fn requests(max: usize) -> Self {
         Self { header: max / HEADER_SHARE, payload: max }
     }
 
     /// La réponse à `req` : l'en-tête au plus `max / 8` ; la charge au plus
-    /// `max / 4` pour un SVG, `max` pour un PNG ou un PDF, rien sinon.
+    /// `max / 4` pour un SVG, `max` pour un PNG (d'une page ou d'une référence) ou un PDF, rien sinon.
     pub(crate) fn reply_to(req: &Request, max: usize) -> Self {
         let payload = match req {
             Request::Svg { .. } => max / SVG_SHARE,
-            Request::Png { .. } | Request::Pdf { .. } => max,
+            Request::Png { .. } | Request::Pdf { .. } | Request::ReferencePng { .. } => max,
             _ => 0,
         };
         Self { header: max / HEADER_SHARE, payload }
```

Serve side: the worker keeps the `ReferencePdf`s by id and answers the three requests.

```diff
--- a/crates/oku-slides-render-worker/src/serve.rs
+++ b/crates/oku-slides-render-worker/src/serve.rs
@@ -16,6 +16,11 @@
 //!   retirés (l'hôte applique la même règle).
 //! - `Svg`, `Png` et `Pdf` d'une génération absente répondent `Error Stale`.
 //! - `Release` retire les générations nommées ; `CloseSlot` l'emplacement.
+//! - `Reference` rend la charge (un `.pptx`) par le témoin (`Office2Pdf`) avec
+//!   les polices nommées, sous un nouvel identifiant (même compteur que les
+//!   générations) ; `ReferencePng` en rastérise une page, `ReleaseReference`
+//!   l'oublie. Une référence absente répond `Error Stale` ; un échec du témoin,
+//!   `Error Setup` (`reference: …`).
 //! - Une requête qui échoue répond `Error` et la boucle continue ; une trame
 //!   illisible arrête `serve`.
 
@@ -27,6 +32,7 @@ use std::time::Instant;
 
 use oku_slides_core::WidgetFile;
 use oku_slides_i18n::Localizer;
+use oku_slides_reference::{Office2Pdf, ReferenceError, ReferencePdf, ReferenceRenderer};
 use oku_slides_render::{Compiled, Fonts, PackWidgets, Registries, RenderError, Renderer, WireError};
 use serde_json::{Map, Value};
 use snafu::Snafu;
@@ -57,6 +63,7 @@ struct Worker {
     registries: Option<Registries>,
     localizer: Option<Localizer>,
     next_generation: u64,
+    references: BTreeMap<u64, ReferencePdf>,
 }
 
 /// Sert les requêtes de `input` jusqu'à sa fin.
@@ -103,6 +110,10 @@ fn render_error(e: &RenderError) -> WireError {
     WireError::from_render(e)
 }
 
+fn reference_error(e: &ReferenceError) -> WireError {
+    setup(format!("reference: {e}"))
+}
+
 /// Le pack d'une requête ; un nom de fichier invalide répond `Error Setup`.
 fn pack_of(p: WirePack) -> Result<PackWidgets, WireError> {
     let files = p
@@ -150,10 +161,34 @@ impl Worker {
                 }
                 ok()
             }
+            Request::Reference { fonts } => self.reference(&fonts, &payload),
+            Request::ReferencePng { reference, page, width } => self.reference_png(reference, page, width),
+            Request::ReleaseReference { reference } => {
+                self.references.remove(&reference);
+                ok()
+            }
         };
         result.unwrap_or_else(error_reply)
     }
 
+    fn reference(&mut self, fonts: &[String], pptx: &[u8]) -> Result<Reply, WireError> {
+        let files = self.fonts_of(fonts)?.files().map(<[u8]>::to_vec).collect::<Vec<_>>();
+        let witness = Office2Pdf;
+        let pdf = witness.render(pptx, &files).map_err(|e| reference_error(&e))?;
+        let pages = (0..pdf.page_count()).map(|i| pdf.page_size(i)).collect::<Result<Vec<_>, _>>();
+        let pages = pages.map_err(|e| reference_error(&e))?;
+        self.next_generation += 1;
+        let reference = self.next_generation;
+        self.references.insert(reference, pdf);
+        Ok((Response::Reference { reference, witness: witness.witness().to_owned(), pages }, Vec::new()))
+    }
+
+    fn reference_png(&self, reference: u64, page: usize, width: u32) -> Result<Reply, WireError> {
+        let pdf = self.references.get(&reference).ok_or(WireError::Stale)?;
+        let png = pdf.raster(page, width).and_then(|r| r.to_png()).map_err(|e| reference_error(&e))?;
+        Ok((Response::Bytes, png))
+    }
+
     fn fonts_of(&self, ids: &[String]) -> Result<Fonts, WireError> {
         let files = ids
             .iter()
```

Client: `Deferred::ReleaseReference` (dropped when its epoch is not the current one) and `put_fonts`, factored out of `open_slot`.

```diff
--- a/crates/oku-slides-render-worker/src/client.rs
+++ b/crates/oku-slides-render-worker/src/client.rs
@@ -4,6 +4,8 @@
 //! - [`RenderSlot`] (`slot`) : un emplacement, même surface que `Renderer`.
 //! - [`RemoteCompiled`](crate::RemoteCompiled) (`compiled`) : une génération rendue ; SVG, PNG et
 //!   PDF à la demande.
+//! - [`RemoteReference`](crate::RemoteReference) (`reference`) : le rendu de référence d'une
+//!   présentation importée (témoin) ; PNG de ses pages à la demande.
 //!
 //! Les règles ci-dessous valent pour les trois.
 //!
@@ -56,6 +58,11 @@ pub(crate) enum Deferred {
     Close {
         slot: SlotId,
     },
+    /// Oublie une référence de l'époque `epoch`.
+    ReleaseReference {
+        reference: u64,
+        epoch: u64,
+    },
 }
 
 /// Ce que partagent les poignées d'un travailleur, hors du verrou du lien.
@@ -178,6 +185,10 @@ impl RenderWorker {
                     // Une génération d'un travailleur précédent : l'actuel ne la connaît pas.
                     Deferred::Release { .. } => continue,
                     Deferred::Close { slot } => Request::CloseSlot { slot },
+                    Deferred::ReleaseReference { reference, epoch } if epoch == current => {
+                        Request::ReleaseReference { reference }
+                    }
+                    Deferred::ReleaseReference { .. } => continue,
                 };
                 link.call(&req, &[])?;
             }
@@ -229,18 +240,7 @@ impl RenderWorker {
     ) -> Result<RenderSlot, RenderError> {
         let mut link = self.link();
         self.prepare(&mut link)?;
-        let mut ids = Vec::new();
-        for bytes in fonts.files() {
-            let id = hex::encode(Sha256::digest(bytes));
-            if !link.state.fonts.contains_key(&id) {
-                link.state.fonts.insert(id.clone(), Arc::from(bytes));
-            }
-            if !link.sent_fonts.contains(&id) {
-                expect_ok(&mut link, &Request::PutFont { id: id.clone() }, bytes)?;
-                link.sent_fonts.insert(id.clone());
-            }
-            ids.push(id);
-        }
+        let ids = put_fonts(&mut link, fonts)?;
         set_locale(&mut link, l)?;
         let slot = self.1.last_slot.fetch_add(1, Ordering::Relaxed) + 1;
         expect_ok(&mut link, &Request::OpenSlot { slot, fonts: ids.clone() }, &[])?;
@@ -256,6 +256,23 @@ impl RenderWorker {
     }
 }
 
+/// Envoie au travailleur celles de `fonts` qu'il n'a pas ; leurs identifiants (empreintes).
+pub(crate) fn put_fonts(link: &mut Link, fonts: &Fonts) -> Result<Vec<String>, RenderError> {
+    let mut ids = Vec::new();
+    for bytes in fonts.files() {
+        let id = hex::encode(Sha256::digest(bytes));
+        if !link.state.fonts.contains_key(&id) {
+            link.state.fonts.insert(id.clone(), Arc::from(bytes));
+        }
+        if !link.sent_fonts.contains(&id) {
+            expect_ok(link, &Request::PutFont { id: id.clone() }, bytes)?;
+            link.sent_fonts.insert(id.clone());
+        }
+        ids.push(id);
+    }
+    Ok(ids)
+}
+
 /// Pose la langue `l` dans le travailleur si elle n'y est pas déjà.
 pub(crate) fn set_locale(link: &mut Link, l: &Localizer) -> Result<(), RenderError> {
     let locale = l.locale();
```

New client module and its exports.

```diff
new file mode 100644
--- /dev/null
+++ b/crates/oku-slides-render-worker/src/reference.rs
@@ -0,0 +1,95 @@
+//! Le rendu de référence d'une présentation importée, fait par le témoin dans
+//! le travailleur (`oku_slides_reference::Office2Pdf`) : le fichier non fiable
+//! y est lu sous les bornes du rendu (délai, mémoire).
+//!
+//! Règles (voir aussi `client`) : aucun pack n'est accusé (il n'y en a pas) ;
+//! une référence d'une autre époque répond `Stale` sans appel ; le `Drop` met
+//! `ReleaseReference` en file.
+
+use oku_slides_render::{Fonts, RenderError};
+
+use crate::client::{Deferred, RenderWorker, put_fonts, unexpected};
+use crate::protocol::{Request, Response};
+
+/// Le rendu de référence d'une présentation, gardé par le travailleur.
+pub struct RemoteReference {
+    worker: RenderWorker,
+    reference: u64,
+    /// Époque du travailleur qui l'a rendu.
+    epoch: u64,
+    witness: String,
+    pages: Vec<(f32, f32)>,
+}
+
+impl std::fmt::Debug for RemoteReference {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        f.debug_struct("RemoteReference")
+            .field("reference", &self.reference)
+            .field("witness", &self.witness)
+            .field("pages", &self.pages.len())
+            .finish_non_exhaustive()
+    }
+}
+
+impl RenderWorker {
+    /// Le rendu de référence de `pptx` par le témoin, avec `fonts` en priorité.
+    pub fn reference(&self, pptx: &[u8], fonts: &Fonts) -> Result<RemoteReference, RenderError> {
+        let mut link = self.link();
+        self.prepare(&mut link)?;
+        let ids = put_fonts(&mut link, fonts)?;
+        let epoch = link.restarts;
+        match link.call(&Request::Reference { fonts: ids }, pptx)? {
+            (Response::Reference { reference, witness, pages }, _) => {
+                Ok(RemoteReference { worker: self.clone(), reference, epoch, witness, pages })
+            }
+            (Response::Error { error }, _) => Err(error.into_render()),
+            (other, _) => Err(unexpected(&other)),
+        }
+    }
+}
+
+impl RemoteReference {
+    /// Nom et version du témoin (`office2pdf 0.8.1`).
+    pub fn witness(&self) -> &str {
+        &self.witness
+    }
+
+    /// Nombre de pages.
+    pub fn page_count(&self) -> usize {
+        self.pages.len()
+    }
+
+    /// Taille de la page `index`, en pt : largeur, hauteur.
+    pub fn page_size(&self, index: usize) -> Option<(f32, f32)> {
+        self.pages.get(index).copied()
+    }
+
+    /// PNG de la page `index` (0 = première), à `width` pixels de large.
+    ///
+    /// Une référence d'un travailleur précédent répond `Stale` sans appel.
+    pub fn png(&self, index: usize, width: u32) -> Result<Vec<u8>, RenderError> {
+        let mut link = self.worker.link();
+        if link.restarts != self.epoch {
+            return Err(RenderError::stale());
+        }
+        self.worker.prepare(&mut link)?;
+        if link.restarts != self.epoch {
+            return Err(RenderError::stale());
+        }
+        match link.call(&Request::ReferencePng { reference: self.reference, page: index, width }, &[])? {
+            (Response::Bytes, png) => Ok(png),
+            (Response::Error { error }, _) => Err(error.into_render()),
+            (other, _) => Err(unexpected(&other)),
+        }
+    }
+}
+
+impl Drop for RemoteReference {
+    fn drop(&mut self) {
+        self.worker.1.defer(Deferred::ReleaseReference { reference: self.reference, epoch: self.epoch });
+    }
+}
+
+#[cfg(test)]
+#[path = "reference_tests.rs"]
+mod reference_tests;
```

```diff
--- a/crates/oku-slides-render-worker/src/lib.rs
+++ b/crates/oku-slides-render-worker/src/lib.rs
@@ -11,6 +11,8 @@
 //! - [`PackProbe`] : la sonde d'un pack, fichier par fichier.
 //! - [`RenderSlot`] : un emplacement, même surface que `Renderer`.
 //! - [`RemoteCompiled`] : une génération rendue ; SVG, PNG et PDF à la demande.
+//! - [`RemoteReference`] : le rendu de référence d'une présentation importée,
+//!   fait par le témoin dans le travailleur ; PNG de ses pages à la demande.
 //! - [`serve`] : la boucle du travailleur ([`ServeError`] si le tube est illisible).
 //! - [`exit_if_worker`] : à appeler en tête de `main`.
 
@@ -23,6 +25,7 @@ mod frame;
 mod link;
 mod pack_probe;
 mod protocol;
+mod reference;
 mod rss;
 mod serve;
 mod slot;
@@ -35,5 +38,6 @@ mod watchdog;
 pub use client::RenderWorker;
 pub use compiled::RemoteCompiled;
 pub use pack_probe::PackProbe;
+pub use reference::RemoteReference;
 pub use serve::{ServeError, exit_if_worker, serve};
 pub use slot::RenderSlot;
```

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-render-worker`
Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-render-worker --all-targets -- -D warnings`
Run: `cargo fmt -p oku-slides-render-worker -- --check`
Expected: all tests of the crate pass (including the 3 new `serve_tests`, the 6 of `reference_tests` and the new `client_tests` one); clippy and fmt clean. `reference_beyond_the_timeout_is_a_timeout` uses a 1 ms timeout and must report `RenderErrorKind::Timeout`.

- [ ] **Step 5: Commit**
```bash
git add Cargo.lock crates/oku-slides-render-worker/Cargo.toml crates/oku-slides-render-worker/src/client.rs crates/oku-slides-render-worker/src/client_tests.rs crates/oku-slides-render-worker/src/frame.rs crates/oku-slides-render-worker/src/lib.rs crates/oku-slides-render-worker/src/protocol.rs crates/oku-slides-render-worker/src/reference.rs crates/oku-slides-render-worker/src/reference_tests.rs crates/oku-slides-render-worker/src/serve.rs crates/oku-slides-render-worker/src/serve_tests.rs
```
Write the message to a file (for example `/tmp/commit-task-2.txt`) containing exactly:
```
feat(render-worker) : rendu de référence par le témoin dans le processus de rendu (t-692)
```
then:
```bash
git commit -F /tmp/commit-task-2.txt
```

**Extracting from the draft:** the implementer may take the files of this task from the draft commit instead of retyping them: `git checkout dc32a579 -- crates/oku-slides-render-worker/Cargo.toml crates/oku-slides-render-worker/src/client.rs crates/oku-slides-render-worker/src/client_tests.rs crates/oku-slides-render-worker/src/frame.rs crates/oku-slides-render-worker/src/lib.rs crates/oku-slides-render-worker/src/protocol.rs crates/oku-slides-render-worker/src/reference.rs crates/oku-slides-render-worker/src/reference_tests.rs crates/oku-slides-render-worker/src/serve.rs crates/oku-slides-render-worker/src/serve_tests.rs` (the draft commit also touches `crates/oku-slides-render-worker/ARCHITECTURE.md`, excluded here because Task 7 carries it). `Cargo.lock` is also changed on `main` since the draft base `53454df2` (compare with `git diff --name-only 53454df2 main`): do not check it out, let cargo update it as described in Step 3 (or apply `git show dc32a579 -- Cargo.lock | git apply --3way`). The other file of this crate changed on `main`, `crates/oku-slides-render-worker/AGENTS.md`, is not part of this task.

### Task 3: Import `metrics`: slide gap in OKLab and regions from the items

This task adds the pure module `metrics` to `oku-slides-import`: both images go to OKLab, are softened by a Gaussian blur (σ = 1 px), and the slide score is the mean ΔE OKLab (×100) between the reference and the Oku render. Regions come from the frames of the source items (open groups, children only), each scored by its mean gap, and the report keeps the 3 worst, so the faulty element is designated. The module takes images from the caller and does no I/O. This is the first version of the region score, exactly as committed; Task 5 later rewrites it (pixel ownership, realignment, worst tile) and adds a `shift` field to `RegionScore`.

**Files:**
- Create: `crates/oku-slides-import/src/metrics.rs` (`Image`, `Region`, `RegionScore`, `SlideScore`, `score`, `regions`, blur)
- Create: `crates/oku-slides-import/src/metrics_tests.rs` (unit tests of the module)
- Modify: `crates/oku-slides-import/src/lib.rs` (`pub mod metrics;` and the crate doc line)
- `crates/oku-slides-import/ARCHITECTURE.md`: updated in Task 7 (English).

**Interfaces:**
- Consumes: nothing new (existing `crate::retrofit::color::{delta_e, oklab, Oklab}`, `crate::source::{Item, ItemKind, Rgba, SourceSlide}`, and the test helpers `crate::test_support::{item, source_slide, text_item}`).
- Produces:
  - `pub const BLUR_SIGMA: f64 = 1.0;`, `pub const REPORTED_REGIONS: usize = 3;`
  - `pub struct Image<'a> { pub width: u32, pub height: u32, pub rgba: &'a [u8] }` (`Debug, Clone, Copy`)
  - `pub struct Region { pub item: String, pub x: f64, pub y: f64, pub w: f64, pub h: f64 }` (`Debug, Clone, PartialEq`)
  - `pub struct RegionScore { pub item: String, pub score: f64 }` (`Debug, Clone, PartialEq, Serialize`)
  - `pub struct SlideScore { pub score: f64, pub regions: Vec<RegionScore> }` (`Debug, Clone, PartialEq, Serialize`)
  - `pub fn regions(slide: &SourceSlide, scale: f64) -> Vec<Region>`
  - `pub fn score(reference: Image<'_>, oku: Image<'_>, regions: &[Region]) -> Option<SlideScore>`

- [ ] **Step 1: Write the failing tests**

Add the module declaration first (Step 3 shows the whole `lib.rs` hunk) so the test file compiles against the missing items; the tests are:

```diff
--- /dev/null
+++ b/crates/oku-slides-import/src/metrics_tests.rs
@@ -0,0 +1,99 @@
+use speculoos::prelude::*;
+
+use super::*;
+use crate::source::ItemKind;
+use crate::test_support::{item, source_slide, text_item};
+
+/// Une image `w × h` d'une seule couleur.
+fn plain(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
+    (0..w * h).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect()
+}
+
+/// `rgba` (`w` de large) avec le carré `x, y, side` peint en `rgb`.
+fn with_square(mut rgba: Vec<u8>, w: u32, (x, y, side): (u32, u32, u32), rgb: [u8; 3]) -> Vec<u8> {
+    for row in y..y + side {
+        for col in x..x + side {
+            let i = ((row * w + col) * 4) as usize;
+            rgba[i..i + 3].copy_from_slice(&rgb);
+        }
+    }
+    rgba
+}
+
+fn image(w: u32, h: u32, rgba: &[u8]) -> Image<'_> {
+    Image { width: w, height: h, rgba }
+}
+
+fn region(item: &str, (x, y, w, h): (f64, f64, f64, f64)) -> Region {
+    Region { item: item.to_owned(), x, y, w, h }
+}
+
+#[test]
+fn score_of_identical_images_is_zero() {
+    let white = plain(40, 20, [255, 255, 255]);
+    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("a", (0.0, 0.0, 10.0, 10.0))]).unwrap();
+    assert_that!((s.score, s.regions[0].score)).is_equal_to((0.0, 0.0));
+}
+
+#[test]
+fn score_of_white_against_black_is_one_hundred() {
+    let (white, black) = (plain(20, 10, [255, 255, 255]), plain(20, 10, [0, 0, 0]));
+    let s = score(image(20, 10, &white), image(20, 10, &black), &[]).unwrap();
+    assert_that!(s.score).is_close_to(100.0, 0.01);
+}
+
+#[test]
+fn score_designates_the_region_that_differs() {
+    let white = plain(100, 50, [255, 255, 255]);
+    let marked = with_square(white.clone(), 100, (20, 20, 10), [255, 0, 0]);
+    let regions = [region("fond", (0.0, 0.0, 100.0, 50.0)), region("carre", (20.0, 20.0, 10.0, 10.0))];
+    let s = score(image(100, 50, &white), image(100, 50, &marked), &regions).unwrap();
+    let order: Vec<&str> = s.regions.iter().map(|r| r.item.as_str()).collect();
+    assert_that!(order).is_equal_to(vec!["carre", "fond"]);
+}
+
+#[test]
+fn score_blur_softens_a_one_pixel_shift() {
+    let white = plain(60, 30, [255, 255, 255]);
+    let (a, b) =
+        (with_square(white.clone(), 60, (20, 10, 6), [0, 0, 0]), with_square(white, 60, (21, 10, 6), [0, 0, 0]));
+    let s = score(image(60, 30, &a), image(60, 30, &b), &[region("carre", (20.0, 10.0, 7.0, 6.0))]).unwrap();
+    // Sans flou, deux colonnes sur sept diffèrent de 100 : 28,6.
+    assert_that!(s.regions[0].score).is_less_than(20.0);
+}
+
+#[test]
+fn score_keeps_the_worst_regions_only() {
+    let white = plain(40, 10, [255, 255, 255]);
+    let regions: Vec<Region> = (0..5).map(|i| region(&format!("r{i}"), (f64::from(i) * 8.0, 0.0, 8.0, 10.0))).collect();
+    let s = score(image(40, 10, &white), image(40, 10, &white), &regions).unwrap();
+    assert_that!(s.regions.len()).is_equal_to(REPORTED_REGIONS);
+}
+
+#[test]
+fn score_skips_a_region_outside_the_image() {
+    let white = plain(20, 10, [255, 255, 255]);
+    let s = score(image(20, 10, &white), image(20, 10, &white), &[region("dehors", (30.0, 0.0, 5.0, 5.0))]).unwrap();
+    assert_that!(s.regions).is_empty();
+}
+
+#[test]
+fn score_of_images_of_different_sizes_is_none() {
+    let (a, b) = (plain(20, 10, [255, 255, 255]), plain(20, 11, [255, 255, 255]));
+    assert_that!(score(image(20, 10, &a), image(20, 11, &b), &[])).is_none();
+}
+
+#[test]
+fn score_of_truncated_bytes_is_none() {
+    let a = plain(20, 10, [255, 255, 255]);
+    assert_that!(score(image(20, 10, &a), image(20, 10, &a[..40]), &[])).is_none();
+}
+
+#[test]
+fn regions_open_groups_and_scale_frames() {
+    let child = text_item("s1/3", (10.0, 20.0, 30.0, 40.0), "x");
+    let group = item("s1/2", (0.0, 0.0, 100.0, 100.0), ItemKind::Group { items: vec![child], smartart: false });
+    let slide = source_slide(1, None, vec![text_item("s1/1", (1.0, 2.0, 3.0, 4.0), "y"), group]);
+    let r = regions(&slide, 0.5);
+    assert_that!(r).is_equal_to(vec![region("s1/1", (0.5, 1.0, 1.5, 2.0)), region("s1/3", (5.0, 10.0, 15.0, 20.0))]);
+}
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import metrics`
Expected: compile error, `metrics` is not a module of the crate (then, with only `pub mod metrics;` and an empty `metrics.rs`, errors naming `score`, `regions`, `Image` and `Region` as unresolved).

- [ ] **Step 3: Implement**

`lib.rs`: declare the module and mention it in the crate documentation.

```diff
--- a/crates/oku-slides-import/src/lib.rs
+++ b/crates/oku-slides-import/src/lib.rs
@@ -11,13 +11,15 @@
 //! la source sur un pack et un thème existants ([`RetrofitTarget`]) ; le hi-fi
 //! ([`hifi::project`]) la reproduit fidèlement sur un thème et un pack générés
 //! pour elle ([`HifiTarget`]), que le plan porte avec les polices à embarquer.
-//! [`coverage::coverage`] mesure ce que le deck obtenu garde de la source.
+//! [`coverage::coverage`] mesure ce que le deck obtenu garde de la source ;
+//! [`metrics::score`], l'écart visuel d'une slide hi-fi à un rendu de référence.
 //! [`fonts::resolve`] choisit la police de chaque famille source et les
 //! fichiers à embarquer dans le deck.
 
 pub mod coverage;
 pub mod fonts;
 pub mod hifi;
+pub mod metrics;
 pub mod plan;
 pub mod report;
 pub mod retrofit;
```

`metrics.rs`: the whole module.

```diff
--- /dev/null
+++ b/crates/oku-slides-import/src/metrics.rs
@@ -0,0 +1,192 @@
+//! Écart visuel d'une slide hi-fi : son rendu Oku contre le rendu de
+//! référence d'un témoin, à la même taille (spec, « Rendu de référence et
+//! métriques »).
+//!
+//! Les deux images passent en OKLab, sont adoucies par un flou gaussien
+//! ([`BLUR_SIGMA`] pixels : l'anticrénelage et un décalage d'un pixel ne
+//! comptent presque pas), puis l'écart de chaque pixel est le ΔE OKLab (×100,
+//! celui de `retrofit::color`). Le score d'une slide est l'écart moyen ; celui
+//! d'une région, l'écart moyen sur le cadre d'un item source ([`regions`]),
+//! pour désigner l'élément fautif. Le rapport garde les [`REPORTED_REGIONS`]
+//! pires. Pur : les images viennent de l'appelant.
+
+use serde::Serialize;
+
+use crate::retrofit::color::{delta_e, oklab};
+use crate::source::{Item, ItemKind, Rgba, SourceSlide};
+
+/// Écart-type du flou gaussien, en pixels (à 960 px de large).
+pub const BLUR_SIGMA: f64 = 1.0;
+
+/// Nombre de régions gardées par slide, les pires d'abord.
+pub const REPORTED_REGIONS: usize = 3;
+
+/// Une image RGBA 8 bits empruntée, ligne à ligne depuis le haut ; l'opacité est ignorée.
+#[derive(Debug, Clone, Copy)]
+pub struct Image<'a> {
+    /// Largeur, en pixels.
+    pub width: u32,
+    /// Hauteur, en pixels.
+    pub height: u32,
+    /// Octets RGBA (`width × height × 4`).
+    pub rgba: &'a [u8],
+}
+
+/// Un rectangle de l'image, en pixels, attaché à un item source.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Region {
+    /// Id de l'item source.
+    pub item: String,
+    /// Bord gauche.
+    pub x: f64,
+    /// Bord haut.
+    pub y: f64,
+    /// Largeur.
+    pub w: f64,
+    /// Hauteur.
+    pub h: f64,
+}
+
+/// L'écart d'une région.
+#[derive(Debug, Clone, PartialEq, Serialize)]
+pub struct RegionScore {
+    /// Id de l'item source.
+    pub item: String,
+    /// ΔE OKLab moyen sur son cadre, arrondi au centième.
+    pub score: f64,
+}
+
+/// L'écart visuel d'une slide.
+#[derive(Debug, Clone, PartialEq, Serialize)]
+pub struct SlideScore {
+    /// ΔE OKLab moyen sur la page, arrondi au centième.
+    pub score: f64,
+    /// Les régions les plus éloignées de la référence, au plus [`REPORTED_REGIONS`].
+    pub regions: Vec<RegionScore>,
+}
+
+/// Les régions de `slide` : le cadre de chaque item, les groupes ouverts
+/// (leurs enfants seulement), à `scale` pixels par pt de la page source.
+pub fn regions(slide: &SourceSlide, scale: f64) -> Vec<Region> {
+    fn walk(items: &[Item], scale: f64, out: &mut Vec<Region>) {
+        for item in items {
+            match &item.kind {
+                ItemKind::Group { items, .. } => walk(items, scale, out),
+                _ => {
+                    let f = item.frame;
+                    out.push(Region {
+                        item: item.id.clone(),
+                        x: f.x * scale,
+                        y: f.y * scale,
+                        w: f.w * scale,
+                        h: f.h * scale,
+                    });
+                }
+            }
+        }
+    }
+    let mut out = Vec::new();
+    walk(&slide.items, scale, &mut out);
+    out
+}
+
+/// L'écart de `oku` à `reference` ; `None` si les images n'ont pas la même
+/// taille, sont vides ou que leurs octets ne la font pas.
+pub fn score(reference: Image<'_>, oku: Image<'_>, regions: &[Region]) -> Option<SlideScore> {
+    let (w, h) = (reference.width as usize, reference.height as usize);
+    let valid = |i: Image<'_>| i.rgba.len() == w * h * 4;
+    if (oku.width, oku.height) != (reference.width, reference.height)
+        || w == 0
+        || h == 0
+        || !valid(reference)
+        || !valid(oku)
+    {
+        return None;
+    }
+    let (a, b) = (blurred(reference), blurred(oku));
+    let diff: Vec<f64> = a.iter().zip(&b).map(|(x, y)| delta_e(*x, *y)).collect();
+    let mut scored: Vec<RegionScore> = regions
+        .iter()
+        .filter_map(|r| region_mean(&diff, w, h, r).map(|s| RegionScore { item: r.item.clone(), score: round2(s) }))
+        .collect();
+    scored.sort_by(|x, y| y.score.total_cmp(&x.score).then_with(|| x.item.cmp(&y.item)));
+    scored.truncate(REPORTED_REGIONS);
+    let mean = diff.iter().sum::<f64>() / diff.len() as f64;
+    Some(SlideScore { score: round2(mean), regions: scored })
+}
+
+/// L'écart moyen sur le rectangle `r` borné à l'image ; `None` s'il n'en couvre aucun pixel.
+fn region_mean(diff: &[f64], w: usize, h: usize, r: &Region) -> Option<f64> {
+    let clamp = |v: f64, max: usize| (v.max(0.0) as usize).min(max);
+    let (x0, y0) = (clamp(r.x.floor(), w), clamp(r.y.floor(), h));
+    let (x1, y1) = (clamp((r.x + r.w).ceil(), w), clamp((r.y + r.h).ceil(), h));
+    if x1 <= x0 || y1 <= y0 {
+        return None;
+    }
+    let sum: f64 = (y0..y1).map(|y| diff[y * w + x0..y * w + x1].iter().sum::<f64>()).sum();
+    Some(sum / ((x1 - x0) * (y1 - y0)) as f64)
+}
+
+/// L'image en OKLab, adoucie par un flou gaussien séparable de [`BLUR_SIGMA`].
+fn blurred(image: Image<'_>) -> Vec<crate::retrofit::color::Oklab> {
+    let (w, h) = (image.width as usize, image.height as usize);
+    let lab: Vec<[f64; 3]> = image
+        .rgba
+        .as_chunks::<4>()
+        .0
+        .iter()
+        .map(|&[r, g, b, _]| {
+            let c = oklab(Rgba::new(r, g, b));
+            [c.l, c.a, c.b]
+        })
+        .collect();
+    let kernel = gaussian(BLUR_SIGMA);
+    let rows = convolve(&lab, w, h, &kernel, true);
+    convolve(&rows, w, h, &kernel, false)
+        .into_iter()
+        .map(|[l, a, b]| crate::retrofit::color::Oklab { l, a, b })
+        .collect()
+}
+
+/// Noyau gaussien normalisé de rayon `⌈3σ⌉`.
+fn gaussian(sigma: f64) -> Vec<f64> {
+    let radius = (3.0 * sigma).ceil() as i64;
+    let k: Vec<f64> = (-radius..=radius).map(|i| (-((i * i) as f64) / (2.0 * sigma * sigma)).exp()).collect();
+    let total: f64 = k.iter().sum();
+    k.into_iter().map(|v| v / total).collect()
+}
+
+/// Convolution de `src` par `kernel`, en ligne (`horizontal`) ou en colonne ;
+/// les bords sont prolongés.
+fn convolve(src: &[[f64; 3]], w: usize, h: usize, kernel: &[f64], horizontal: bool) -> Vec<[f64; 3]> {
+    let radius = (kernel.len() / 2) as i64;
+    let mut out = vec![[0.0; 3]; src.len()];
+    for y in 0..h {
+        for x in 0..w {
+            let mut acc = [0.0; 3];
+            for (k, weight) in kernel.iter().enumerate() {
+                let d = k as i64 - radius;
+                let (sx, sy) = if horizontal {
+                    ((x as i64 + d).clamp(0, w as i64 - 1) as usize, y)
+                } else {
+                    (x, (y as i64 + d).clamp(0, h as i64 - 1) as usize)
+                };
+                let p = src[sy * w + sx];
+                for c in 0..3 {
+                    acc[c] += weight * p[c];
+                }
+            }
+            out[y * w + x] = acc;
+        }
+    }
+    out
+}
+
+/// Arrondi au centième.
+fn round2(v: f64) -> f64 {
+    (v * 100.0).round() / 100.0
+}
+
+#[cfg(test)]
+#[path = "metrics_tests.rs"]
+mod metrics_tests;
```

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import`
Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-import --all-targets -- -D warnings`
Run: `cargo fmt -p oku-slides-import -- --check`
Expected: all tests pass (the 9 `metrics` tests included), clippy clean, fmt clean.

- [ ] **Step 5: Commit**
```bash
git add crates/oku-slides-import/src/lib.rs crates/oku-slides-import/src/metrics.rs crates/oku-slides-import/src/metrics_tests.rs
```
Write the message to a file (for example `/tmp/commit-msg-task-3.txt`) containing exactly:
```
feat(import) : écart visuel OKLab d'une slide et de ses régions (t-692)
```
Then, as a separate command:
```bash
git commit -F /tmp/commit-msg-task-3.txt
```

**Extracting from the draft:** the implementer may take the files of this task from the draft commit instead of retyping them: `git checkout e72b2749 -- crates/oku-slides-import/src/lib.rs crates/oku-slides-import/src/metrics.rs crates/oku-slides-import/src/metrics_tests.rs` (never a directory; the `ARCHITECTURE.md` of the draft commit is not taken, see Task 7). None of these files changed on `main` since the draft base `53454df2` (checked with `git diff --name-only 53454df2 main`); if that changes, apply the file with `git show e72b2749 -- <file> | git apply --3way` instead.

### Task 4: Import report: reference, score per slide, `Overflow` loss

This task extends the import report. A hi-fi import now records its reference render (`ReferenceReport`: witness, threshold, and a `ReferenceCause` when the reference is missing), each slide carries an optional visual `score`, and a new `LossKind::Overflow` reports an element rendered past the edge of its slide. `ImportReport` gains `above_threshold` and `mean_score`, and `to_json` writes `above_threshold` per slide and a localized `message` for a missing reference. `font_notice` moves to `report/fonts.rs` so `report.rs` stays under 500 lines (494 after this task). Two Fluent keys (`import-loss-overflow`, `import-reference-unavailable`) are added to all 20 `document.ftl`.

Note on translations: the `ja`, `zh-Hans` and `zh-Hant` `import-reference-unavailable` messages below are given directly in their **final form** (no space between a number and its CJK counter, an AGENTS.md rule of `oku-slides-i18n`), taken from `e33bc28`. They differ from the draft commit `40a05b9d` on the `[pages]` line only. Task 5 therefore no longer touches `document.ftl`.

**Files:**
- Create: `crates/oku-slides-import/src/report/reference.rs` (`ReferenceReport`, `ReferenceCause`, its `Localize` impl)
- Create: `crates/oku-slides-import/src/report/reference_tests.rs` (unit tests of the cause messages and `is_available`)
- Create: `crates/oku-slides-import/src/report/fonts.rs` (`font_notice`, moved out of `report.rs`)
- Modify: `crates/oku-slides-import/src/report.rs` (module declarations and re-exports, `SlideReport::score`, `ImportReport::reference`, `LossKind::Overflow`, `above_threshold`, `mean_score`, `to_json` additions, `font_notice` removed)
- Modify: `crates/oku-slides-import/src/report_tests.rs` (`score: None` in the `slide` helper, overflow and score/threshold/reference tests)
- Modify: `crates/oku-slides-import/src/retrofit/slide.rs` (`score: None` in the `SlideReport` constructor)
- Modify: `crates/oku-slides-import/src/hifi/slide.rs` (`score: None` in the `SlideReport` constructor)
- Modify: the 20 `crates/oku-slides-i18n/src/locales/<tag>/document.ftl` (`ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`): two keys each
- `crates/oku-slides-import/ARCHITECTURE.md`: updated in Task 7 (English).
- Test: `crates/oku-slides-import/src/report_tests.rs`, `crates/oku-slides-import/src/report/reference_tests.rs`

**Interfaces:**
- Consumes: `crate::metrics::{SlideScore, RegionScore}` from the metrics task (`SlideScore { score: f64, regions: Vec<RegionScore> }`, `RegionScore { item: String, score: f64 }` as at the draft commit; `SlideScore` derives `Debug, Clone, PartialEq, Serialize`). If a later task adds a field to `RegionScore` (the draft later adds `shift: [i32; 2]`), that task updates the `scored` helper and the `to_json_writes_the_score_and_its_regions` expectation of this task.
- Produces:
  - `pub enum ReferenceCause { Failed, Timeout, Memory, Pages { reference: u32, deck: u32 } }` (serde `snake_case`, `tag = "type"`; `Localize`), `pub struct ReferenceReport { pub witness: String, pub threshold: f64, pub unavailable: Option<ReferenceCause> }` with `pub fn is_available(&self) -> bool`; both re-exported from `crate::report` (`pub use reference::{ReferenceCause, ReferenceReport}`).
  - `SlideReport.score: Option<SlideScore>` (new pub field; every constructor must set it), `ImportReport.reference: Option<ReferenceReport>` (new pub field, `None` from `ImportReport::new`).
  - `pub fn above_threshold(&self) -> impl Iterator<Item = &SlideReport>`, `pub fn mean_score(&self) -> Option<f64>` on `ImportReport`.
  - `LossKind::Overflow { element: String }` (message id `import-loss-overflow`).
  - `pub(crate) fn font_notice(c: &FontChoice, l: &Localizer) -> Option<String>` now in `crate::report::fonts` (private module, used by `report.rs`).
  - Fluent keys `import-loss-overflow` (variable `$element`) and `import-reference-unavailable` (variables `$cause` in `timeout|memory|pages|failed`, `$reference`, `$deck`).

- [ ] **Step 1: Write the failing tests**

New unit tests of the cause messages:

```diff
--- /dev/null
+++ b/crates/oku-slides-import/src/report/reference_tests.rs
@@ -0,0 +1,29 @@
+use oku_slides_i18n::{Localize, Localizer, english};
+use rstest::rstest;
+use speculoos::prelude::*;
+
+use super::*;
+
+#[rstest]
+#[case(ReferenceCause::Failed)]
+#[case(ReferenceCause::Timeout)]
+#[case(ReferenceCause::Memory)]
+#[case(ReferenceCause::Pages { reference: 7, deck: 8 })]
+fn cause_localize_in_every_locale_formats_its_own_message(#[case] cause: ReferenceCause) {
+    for tag in oku_slides_i18n::available() {
+        let msg = cause.localize(&Localizer::new(Some(tag)));
+        assert_that!(msg.is_empty() || msg.contains('{')).named(tag).is_false();
+    }
+}
+
+#[test]
+fn cause_pages_gives_both_counts() {
+    let msg = ReferenceCause::Pages { reference: 7, deck: 8 }.localize(english());
+    assert_that!((msg.contains('7'), msg.contains('8'))).is_equal_to((true, true));
+}
+
+#[test]
+fn reference_without_cause_is_available() {
+    let r = ReferenceReport { witness: "office2pdf 0.8.1".into(), threshold: 5.0, unavailable: None };
+    assert_that!(r.is_available()).is_true();
+}
```

Tests added to the existing report tests (the `slide` helper gets `score: None`, `FontDecision` is imported, `Overflow` joins the locale table):

```diff
--- a/crates/oku-slides-import/src/report_tests.rs
+++ b/crates/oku-slides-import/src/report_tests.rs
@@ -4,7 +4,7 @@ use serde_json::json;
 use speculoos::prelude::*;
 
 use super::*;
-use crate::fonts::FontOrigin;
+use crate::fonts::{FontDecision, FontOrigin};
 use crate::source::{Warning, WarningKind};
 
 fn slide(losses: Vec<Loss>) -> SlideReport {
@@ -15,6 +15,7 @@ fn slide(losses: Vec<Loss>) -> SlideReport {
         placement: Placement::Flow,
         losses,
         fallbacks: vec!["stack".into()],
+        score: None,
     }
 }
 
@@ -56,6 +57,7 @@ fn slide(losses: Vec<Loss>) -> SlideReport {
 #[case(LossKind::LayoutDropped { layout: "Titre".into() })]
 #[case(LossKind::DecorTruncated { omitted: 12 })]
 #[case(LossKind::TableStyle)]
+#[case(LossKind::Overflow { element: "s3-2".into() })]
 fn loss_localize_in_every_locale_formats_its_own_message(#[case] kind: LossKind) {
     let id = kind.message_id();
     let loss = Loss::slide(kind);
@@ -340,3 +342,64 @@ fn import_report_to_json_without_fonts_writes_no_fonts_section() {
 fn ratio_is_complete_when_everything_is_found(#[case] r: Ratio, #[case] complete: bool) {
     assert_that!(r.is_complete()).is_equal_to(complete);
 }
+
+#[test]
+fn loss_localize_overflow_names_the_element() {
+    let msg = Loss::slide(LossKind::Overflow { element: "s3-2".into() }).localize(english());
+    assert_that!(msg).contains("s3-2");
+}
+
+fn scored(n: u32, score: f64) -> SlideReport {
+    let regions = vec![crate::metrics::RegionScore { item: format!("s{n}/1"), score }];
+    SlideReport { source: n, slide: format!("s{n}"), score: Some(SlideScore { score, regions }), ..slide(vec![]) }
+}
+
+fn with_reference(slides: Vec<SlideReport>, unavailable: Option<ReferenceCause>) -> ImportReport {
+    let reference = ReferenceReport { witness: "office2pdf 0.8.1".into(), threshold: 5.0, unavailable };
+    ImportReport { slides, reference: Some(reference), ..ImportReport::new(Mode::Hifi) }
+}
+
+#[test]
+fn to_json_marks_the_slides_above_the_threshold() {
+    let v = with_reference(vec![scored(1, 2.0), scored(2, 7.5)], None).to_json(english());
+    let marks: Vec<Option<bool>> = (0..2).map(|i| v["slides"][i]["above_threshold"].as_bool()).collect();
+    assert_that!(marks).is_equal_to(vec![None, Some(true)]);
+}
+
+#[test]
+fn to_json_writes_the_score_and_its_regions() {
+    let v = with_reference(vec![scored(1, 2.0)], None).to_json(english());
+    assert_that!(v["slides"][0]["score"].clone())
+        .is_equal_to(json!({"score": 2.0, "regions": [{"item": "s1/1", "score": 2.0}]}));
+}
+
+#[test]
+fn to_json_without_score_writes_no_score() {
+    let v = with_reference(vec![slide(vec![])], None).to_json(english());
+    assert_that!(v["slides"][0].get("score")).is_none();
+}
+
+#[test]
+fn to_json_explains_a_missing_reference() {
+    let v = with_reference(vec![], Some(ReferenceCause::Timeout)).to_json(english());
+    assert_that!(v["reference"]["message"].as_str().unwrap()).contains("too long");
+}
+
+#[test]
+fn to_json_without_reference_writes_none() {
+    let v = ImportReport::new(Mode::Retrofit).to_json(english());
+    assert_that!(v.get("reference")).is_none();
+}
+
+#[test]
+fn above_threshold_lists_the_slides_beyond_it() {
+    let r = with_reference(vec![scored(1, 2.0), scored(2, 7.5), scored(3, 5.0)], None);
+    let above: Vec<u32> = r.above_threshold().map(|s| s.source).collect();
+    assert_that!(above).is_equal_to(vec![2]);
+}
+
+#[test]
+fn mean_score_averages_the_scored_slides() {
+    let r = with_reference(vec![scored(1, 2.0), slide(vec![]), scored(3, 5.0)], None);
+    assert_that!(r.mean_score()).is_equal_to(Some(3.5));
+}
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import report`
Expected: compile errors naming the missing items: no field `score` on `SlideReport`, no field `reference` on `ImportReport`, no variant `Overflow` on `LossKind`, unresolved `ReferenceReport` / `ReferenceCause`, no method `above_threshold` / `mean_score`. (`reference_tests.rs` is not yet declared as a module, so its tests only appear once Step 3 adds `reference.rs`.)

- [ ] **Step 3: Implement**

The cause type and the reference report (new file; the tests of Step 1 are included through `#[path]`):

```diff
--- /dev/null
+++ b/crates/oku-slides-import/src/report/reference.rs
@@ -0,0 +1,62 @@
+//! Ce que le rapport dit du rendu de référence d'un import hi-fi : le témoin,
+//! le seuil, et pourquoi la référence manque (jamais une erreur de l'import).
+
+use oku_slides_i18n::{Localize, Localizer};
+use serde::Serialize;
+
+/// Le rendu de référence d'un import hi-fi.
+#[derive(Debug, Clone, PartialEq, Serialize)]
+pub struct ReferenceReport {
+    /// Nom et version du témoin (`office2pdf 0.8.1`).
+    pub witness: String,
+    /// Écart (ΔE OKLab ×100) au-delà duquel une slide est signalée.
+    pub threshold: f64,
+    /// Pourquoi les slides n'ont pas de score ; `None` : elles en ont un.
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub unavailable: Option<ReferenceCause>,
+}
+
+impl ReferenceReport {
+    /// Les slides ont-elles un score ?
+    pub fn is_available(&self) -> bool {
+        self.unavailable.is_none()
+    }
+}
+
+/// Pourquoi la référence manque.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
+#[serde(rename_all = "snake_case", tag = "type")]
+pub enum ReferenceCause {
+    /// Le témoin n'a pas rendu la présentation (fichier qu'il ne lit pas, erreur).
+    Failed,
+    /// Le témoin a dépassé le délai du processus de rendu.
+    Timeout,
+    /// Le témoin a dépassé la mémoire du processus de rendu.
+    Memory,
+    /// Le témoin et le deck n'ont pas le même nombre de pages.
+    Pages {
+        /// Pages du témoin.
+        reference: u32,
+        /// Pages du deck Oku.
+        deck: u32,
+    },
+}
+
+impl Localize for ReferenceCause {
+    fn localize(&self, l: &Localizer) -> String {
+        let (cause, reference, deck) = match self {
+            Self::Failed => ("failed", 0, 0),
+            Self::Timeout => ("timeout", 0, 0),
+            Self::Memory => ("memory", 0, 0),
+            Self::Pages { reference, deck } => ("pages", *reference, *deck),
+        };
+        l.tr_with(
+            "import-reference-unavailable",
+            &[("cause", cause.into()), ("reference", reference.into()), ("deck", deck.into())],
+        )
+    }
+}
+
+#[cfg(test)]
+#[path = "reference_tests.rs"]
+mod reference_tests;
```

Move `font_notice` into its own module (new file, text unchanged):

```diff
--- /dev/null
+++ b/crates/oku-slides-import/src/report/fonts.rs
@@ -0,0 +1,36 @@
+//! Ce que le rapport dit d'une police : substitut, absence, droits restreints.
+
+use oku_slides_i18n::Localizer;
+
+use crate::fonts::{FontChoice, FontDecision};
+
+/// Ce que le rapport dit d'une police : substitut, absence (fichiers trop
+/// gros, droits restreints ou introuvable, avec la police de repli retenue
+/// nommée, ou sans repli), droits restreints ; `None` pour une police reprise
+/// telle quelle.
+pub(crate) fn font_notice(c: &FontChoice, l: &Localizer) -> Option<String> {
+    let family = ("family", c.family.as_str().into());
+    match (&c.decision, c.used.as_deref(), c.restricted) {
+        (FontDecision::Substituted { .. }, Some(by), restricted) => {
+            let id = if restricted { "import-font-restricted-substituted" } else { "import-font-substituted" };
+            Some(l.tr_with(id, &[family, ("by", by.into())]))
+        }
+        (FontDecision::Missing, _, restricted) => {
+            // Le repli est nommé : « police du corps » serait faux (circulaire)
+            // quand la famille absente est elle-même la police du corps.
+            let Some(by) = c.fallback.as_deref() else {
+                return Some(l.tr_with("import-font-missing-no-fallback", &[family]));
+            };
+            let id = if c.too_large {
+                "import-font-too-large"
+            } else if restricted {
+                "import-font-restricted-missing"
+            } else {
+                "import-font-missing"
+            };
+            Some(l.tr_with(id, &[family, ("by", by.into())]))
+        }
+        (_, _, true) => Some(l.tr_with("import-font-restricted", &[family])),
+        (_, _, false) => None,
+    }
+}
```

Declare the modules, add `Overflow`, `score`, `reference`, `above_threshold`, `mean_score` and the `to_json` additions, and remove `font_notice` from `report.rs`:

```diff
--- a/crates/oku-slides-import/src/report.rs
+++ b/crates/oku-slides-import/src/report.rs
@@ -4,12 +4,21 @@
 //! Une perte n'est jamais une erreur : l'import continue, le rapport la dit.
 //! [`ImportReport::to_json`] donne la forme écrite par `--report`, avec le
 //! message localisé de chaque perte et de chaque avertissement du lecteur.
+//! En hi-fi, le rendu de référence ([`ReferenceReport`]) et l'écart visuel de
+//! chaque slide (`SlideReport::score`, [`crate::metrics`]) s'y ajoutent.
+
+mod fonts;
+mod reference;
+
+use fonts::font_notice;
+pub use reference::{ReferenceCause, ReferenceReport};
 
 use oku_slides_i18n::{Localize, Localizer};
 use serde::Serialize;
 use serde_json::Value;
 
-use crate::fonts::{FontChoice, FontDecision};
+use crate::fonts::FontChoice;
+use crate::metrics::SlideScore;
 use crate::source::{OpaqueReason, Warning, WarningKind};
 
 /// Mode d'import.
@@ -150,6 +159,12 @@ pub enum LossKind {
     /// largeurs de colonnes ou tailles de texte que le widget `table` ne rend
     /// pas, ou un style autre que celui du thème généré (le premier tableau).
     TableStyle,
+    /// Élément ou champ rendu au-delà du bord de sa slide (relevé sur le rendu,
+    /// dans les deux modes) : un contenu plus haut que sa place (t-816).
+    Overflow {
+        /// Id de l'élément Oku, ou `<élément>/<champ>` pour un champ.
+        element: String,
+    },
 }
 
 /// Une perte, sur un item source ou sur la slide entière.
@@ -234,6 +249,7 @@ impl LossKind {
             LossKind::LayoutDropped { .. } => "import-loss-layout-dropped",
             LossKind::DecorTruncated { .. } => "import-loss-decor-truncated",
             LossKind::TableStyle => "import-loss-table-style",
+            LossKind::Overflow { .. } => "import-loss-overflow",
         }
     }
 }
@@ -258,6 +274,7 @@ impl Localize for Loss {
             }
             LossKind::LayoutDropped { layout } => l.tr_with(id, &[("layout", layout.as_str().into())]),
             LossKind::DecorTruncated { omitted } => l.tr_with(id, &[("omitted", (*omitted).into())]),
+            LossKind::Overflow { element } => l.tr_with(id, &[("element", element.as_str().into())]),
             LossKind::ListLevels
             | LossKind::MergedCells
             | LossKind::PercentStacked
@@ -300,37 +317,6 @@ impl Localize for Warning {
     }
 }
 
-/// Ce que le rapport dit d'une police : substitut, absence (fichiers trop
-/// gros, droits restreints ou introuvable, avec la police de repli retenue
-/// nommée, ou sans repli), droits restreints ; `None` pour une police reprise
-/// telle quelle.
-pub(crate) fn font_notice(c: &FontChoice, l: &Localizer) -> Option<String> {
-    let family = ("family", c.family.as_str().into());
-    match (&c.decision, c.used.as_deref(), c.restricted) {
-        (FontDecision::Substituted { .. }, Some(by), restricted) => {
-            let id = if restricted { "import-font-restricted-substituted" } else { "import-font-substituted" };
-            Some(l.tr_with(id, &[family, ("by", by.into())]))
-        }
-        (FontDecision::Missing, _, restricted) => {
-            // Le repli est nommé : « police du corps » serait faux (circulaire)
-            // quand la famille absente est elle-même la police du corps.
-            let Some(by) = c.fallback.as_deref() else {
-                return Some(l.tr_with("import-font-missing-no-fallback", &[family]));
-            };
-            let id = if c.too_large {
-                "import-font-too-large"
-            } else if restricted {
-                "import-font-restricted-missing"
-            } else {
-                "import-font-missing"
-            };
-            Some(l.tr_with(id, &[family, ("by", by.into())]))
-        }
-        (_, _, true) => Some(l.tr_with("import-font-restricted", &[family])),
-        (_, _, false) => None,
-    }
-}
-
 /// Ajoute à chaque objet de `written` (le tableau `key` de `v`) le message localisé de l'entrée de même rang.
 fn add_messages<T: Localize>(v: &mut Value, key: &str, entries: &[T], l: &Localizer) {
     let Some(written) = v.get_mut(key).and_then(Value::as_array_mut) else { return };
@@ -356,6 +342,9 @@ pub struct SlideReport {
     pub losses: Vec<Loss>,
     /// Sortes de widget de repli employées (`stack`…).
     pub fallbacks: Vec<String>,
+    /// Écart visuel au rendu de référence (hi-fi, référence disponible).
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub score: Option<SlideScore>,
 }
 
 /// Une proportion : `found` sur `total`.
@@ -437,12 +426,15 @@ pub struct ImportReport {
     /// Polices : une décision par famille source (hi-fi ; vide et non écrit en retrofit).
     #[serde(skip_serializing_if = "Vec::is_empty")]
     pub fonts: Vec<FontChoice>,
+    /// Rendu de référence (hi-fi, `--reference auto`) ; non écrit sans lui.
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub reference: Option<ReferenceReport>,
 }
 
 impl ImportReport {
     /// Rapport vide du mode `mode`.
     pub fn new(mode: Mode) -> Self {
-        Self { mode, slides: Vec::new(), warnings: Vec::new(), coverage: None, fonts: Vec::new() }
+        Self { mode, slides: Vec::new(), warnings: Vec::new(), coverage: None, fonts: Vec::new(), reference: None }
     }
 
     /// Nombre total de pertes.
@@ -450,18 +442,42 @@ impl ImportReport {
         self.slides.iter().map(|s| s.losses.len()).sum()
     }
 
+    /// Les slides dont l'écart visuel dépasse le seuil de la référence.
+    pub fn above_threshold(&self) -> impl Iterator<Item = &SlideReport> {
+        let threshold = self.reference.as_ref().map(|r| r.threshold);
+        self.slides.iter().filter(move |s| s.score.as_ref().zip(threshold).is_some_and(|(sc, t)| sc.score > t))
+    }
+
+    /// L'écart visuel moyen des slides qui en ont un.
+    pub fn mean_score(&self) -> Option<f64> {
+        let scores: Vec<f64> = self.slides.iter().filter_map(|s| s.score.as_ref().map(|sc| sc.score)).collect();
+        (!scores.is_empty()).then(|| (scores.iter().sum::<f64>() / scores.len() as f64 * 100.0).round() / 100.0)
+    }
+
     /// La forme écrite : la sérialisation, plus un `message` localisé à côté
-    /// de chaque perte, de chaque avertissement et de chaque police
-    /// remplacée, absente ou aux droits restreints.
+    /// de chaque perte, de chaque avertissement, de chaque police remplacée,
+    /// absente ou aux droits restreints, et de la référence absente ;
+    /// `above_threshold` sur chaque slide dont l'écart dépasse le seuil.
     pub fn to_json(&self, l: &Localizer) -> Value {
         // Ces types se sérialisent toujours : clés textuelles, nombres finis.
         let mut v = serde_json::to_value(self).unwrap_or_default();
         add_messages(&mut v, "warnings", &self.warnings, l);
+        let threshold = self.reference.as_ref().map(|r| r.threshold);
         if let Some(slides) = v.get_mut("slides").and_then(Value::as_array_mut) {
             for (slide, report) in slides.iter_mut().zip(&self.slides) {
                 add_messages(slide, "losses", &report.losses, l);
+                let above = report.score.as_ref().zip(threshold).is_some_and(|(s, t)| s.score > t);
+                if let (Some(o), true) = (slide.as_object_mut(), above) {
+                    o.insert("above_threshold".to_owned(), Value::Bool(true));
+                }
             }
         }
+        if let (Some(o), Some(cause)) = (
+            v.get_mut("reference").and_then(Value::as_object_mut),
+            self.reference.as_ref().and_then(|r| r.unavailable.as_ref()),
+        ) {
+            o.insert("message".to_owned(), Value::String(cause.localize(l)));
+        }
         if let Some(fonts) = v.get_mut("fonts").and_then(Value::as_array_mut) {
             for (w, choice) in fonts.iter_mut().zip(&self.fonts) {
                 if let (Some(o), Some(msg)) = (w.as_object_mut(), font_notice(choice, l)) {
```

Every `SlideReport` constructor sets `score: None` (the CLI task fills it in later):

```diff
--- a/crates/oku-slides-import/src/retrofit/slide.rs
+++ b/crates/oku-slides-import/src/retrofit/slide.rs
@@ -404,6 +404,7 @@ pub(crate) fn project_slide<'a>(
         placement,
         losses,
         fallbacks: kinds,
+        score: None,
     };
     Some(Projected { ops, report, assets, fallbacks, origins })
 }
```

```diff
--- a/crates/oku-slides-import/src/hifi/slide.rs
+++ b/crates/oku-slides-import/src/hifi/slide.rs
@@ -120,6 +120,7 @@ pub(super) fn project_slide(src: &SourceSlide, cx: &SlideCx<'_>) -> Projected {
         placement: Placement::Free,
         losses,
         fallbacks: Vec::new(),
+        score: None,
     };
     Projected { slide, report, assets: out.assets, origins: out.origins }
 }
```

Fluent keys, English first (the reference locale), then the 19 others. Placement is after `import-loss-table-style`, before `import-font-substituted`. The `ja`, `zh-Hans` and `zh-Hant` `[pages]` lines are in their final form (no space around the numbers); the implementer may extract all 20 files with the command under "Extracting from the draft" and then fix only those three `[pages]` lines to match the diffs below.

`en`:

```diff
--- a/crates/oku-slides-i18n/src/locales/en/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/en/document.ftl
@@ -80,6 +80,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Too many layouts for one pack: “{ $layout }” has no template; its slides use the blank widget and get its decoration as elements
 import-loss-decor-truncated = Decoration cut to fit the size of a pack file: { $omitted } items left out
 import-loss-table-style = Table style approximated: its own cell backgrounds, borders, column widths or text sizes are lost
+import-loss-overflow = Element “{ $element }” runs past the edge of the slide
+import-reference-unavailable = No visual score: { $cause ->
+    [timeout] the reference render took too long
+    [memory] the reference render used too much memory
+    [pages] the reference render has { $reference } pages for { $deck } in the deck
+   *[failed] the reference renderer could not read the presentation
+}
 import-font-substituted = { $family } replaced by { $by }, with matching metrics
 import-font-missing = { $family } not found: the text uses { $by }
 import-font-restricted = { $family }: its license forbids embedding some of its faces in the deck
```

`fr`:

```diff
--- a/crates/oku-slides-i18n/src/locales/fr/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/fr/document.ftl
@@ -80,6 +80,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Trop de layouts pour un pack : « { $layout } » n'a pas de modèle, ses slides prennent le widget vierge et reçoivent son décor en éléments
 import-loss-decor-truncated = Décor coupé pour tenir dans la taille d'un fichier de pack : { $omitted } éléments omis
 import-loss-table-style = Style du tableau approché : ses fonds de cellule, bordures, largeurs de colonnes ou tailles de texte propres sont perdus
+import-loss-overflow = L'élément « { $element } » dépasse le bord de la slide
+import-reference-unavailable = Pas d'écart visuel : { $cause ->
+    [timeout] le rendu de référence a pris trop de temps
+    [memory] le rendu de référence a demandé trop de mémoire
+    [pages] le rendu de référence a { $reference } pages pour { $deck } dans le deck
+   *[failed] le témoin n'a pas pu lire la présentation
+}
 import-font-substituted = { $family } remplacée par { $by }, aux métriques compatibles
 import-font-missing = { $family } introuvable : le texte prend { $by }
 import-font-restricted = { $family } : sa licence interdit d'embarquer certaines de ses faces dans le deck
```

`ar`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ar/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/ar/document.ftl
@@ -85,6 +85,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = تخطيطات كثيرة جدًا لحزمة واحدة: «{ $layout }» بلا قالب؛ تستخدم شرائحه الأداة الفارغة وتتلقى زخرفته كعناصر
 import-loss-decor-truncated = قُصّت الزخرفة لتلائم حجم ملف الحزمة: عناصر محذوفة: { $omitted }
 import-loss-table-style = قُرّب نمط الجدول: خلفيات الخلايا أو الحدود أو عروض الأعمدة أو أحجام النص الخاصة به فُقدت
+import-loss-overflow = العنصر «{ $element }» يتجاوز حافة الشريحة
+import-reference-unavailable = لا توجد درجة مرئية: { $cause ->
+    [timeout] استغرق العرض المرجعي وقتًا طويلًا
+    [memory] استهلك العرض المرجعي ذاكرة كبيرة
+    [pages] يحتوي العرض المرجعي على { $reference } صفحات مقابل { $deck } في العرض التقديمي
+   *[failed] تعذّر على أداة العرض المرجعي قراءة العرض التقديمي
+}
 import-font-substituted = استُبدل { $family } بـ { $by } ذي المقاييس المتوافقة
 import-font-missing = لم يُعثر على { $family }: يستخدم النص { $by }
 import-font-restricted = { $family }: يمنع ترخيصه تضمين بعض أنماطه في العرض التقديمي
```

`bn`:

```diff
--- a/crates/oku-slides-i18n/src/locales/bn/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/bn/document.ftl
@@ -81,6 +81,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = একটি প্যাকের জন্য অনেক বেশি লেআউট: “{ $layout }”-এর কোনো টেমপ্লেট নেই; এর স্লাইডগুলো ফাঁকা উইজেট নেয় এবং এর সজ্জা উপাদান হিসেবে পায়
 import-loss-decor-truncated = প্যাক ফাইলের আকারে আঁটাতে সজ্জা ছাঁটা হয়েছে: বাদ পড়া উপাদান: { $omitted }
 import-loss-table-style = টেবিলের শৈলী আনুমানিক: এর নিজস্ব ঘরের পটভূমি, সীমানা, কলামের প্রস্থ বা লেখার আকার হারিয়েছে
+import-loss-overflow = উপাদান “{ $element }” স্লাইডের প্রান্ত ছাড়িয়ে যায়
+import-reference-unavailable = কোনো দৃশ্যমান স্কোর নেই: { $cause ->
+    [timeout] রেফারেন্স রেন্ডারে খুব বেশি সময় লেগেছে
+    [memory] রেফারেন্স রেন্ডার খুব বেশি মেমরি নিয়েছে
+    [pages] রেফারেন্স রেন্ডারে উপস্থাপনার { $deck }টির বদলে { $reference }টি পাতা আছে
+   *[failed] রেফারেন্স রেন্ডারার উপস্থাপনাটি পড়তে পারেনি
+}
 import-font-substituted = { $family } মিলযুক্ত মাপের { $by } দিয়ে বদলানো হয়েছে
 import-font-missing = { $family } পাওয়া যায়নি: লেখা { $by } ব্যবহার করে
 import-font-restricted = { $family }: এর লাইসেন্স কিছু শৈলী প্রেজেন্টেশনে এমবেড করতে দেয় না
```

`de`:

```diff
--- a/crates/oku-slides-i18n/src/locales/de/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/de/document.ftl
@@ -80,6 +80,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Zu viele Layouts für ein Paket: „{ $layout }“ hat keine Vorlage; seine Folien nutzen das leere Widget und erhalten seine Dekoration als Elemente
 import-loss-decor-truncated = Dekoration gekürzt, um in die Größe einer Paketdatei zu passen: { $omitted } Elemente ausgelassen
 import-loss-table-style = Tabellenstil angenähert: eigene Zellhintergründe, Rahmen, Spaltenbreiten oder Textgrößen gehen verloren
+import-loss-overflow = Element „{ $element }“ ragt über den Folienrand hinaus
+import-reference-unavailable = Keine visuelle Bewertung: { $cause ->
+    [timeout] das Referenz-Rendering hat zu lange gedauert
+    [memory] das Referenz-Rendering hat zu viel Speicher benötigt
+    [pages] das Referenz-Rendering hat { $reference } Seiten für { $deck } im Deck
+   *[failed] der Referenz-Renderer konnte die Präsentation nicht lesen
+}
 import-font-substituted = { $family } ersetzt durch { $by } mit passenden Metriken
 import-font-missing = { $family } nicht gefunden: der Text verwendet { $by }
 import-font-restricted = { $family }: die Lizenz verbietet, einige Schnitte in die Präsentation einzubetten
```

`es`:

```diff
--- a/crates/oku-slides-i18n/src/locales/es/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/es/document.ftl
@@ -80,6 +80,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Demasiados diseños para un paquete: «{ $layout }» no tiene plantilla; sus diapositivas usan el widget vacío y reciben su decoración como elementos
 import-loss-decor-truncated = Decoración recortada para caber en el tamaño de un archivo del paquete: { $omitted } elementos omitidos
 import-loss-table-style = Estilo de tabla aproximado: se pierden sus fondos de celda, bordes, anchos de columna o tamaños de texto propios
+import-loss-overflow = El elemento «{ $element }» sobrepasa el borde de la diapositiva
+import-reference-unavailable = Sin puntuación visual: { $cause ->
+    [timeout] el renderizado de referencia tardó demasiado
+    [memory] el renderizado de referencia usó demasiada memoria
+    [pages] el renderizado de referencia tiene { $reference } páginas para { $deck } en la presentación
+   *[failed] el renderizador de referencia no pudo leer la presentación
+}
 import-font-substituted = { $family } sustituida por { $by }, con métricas compatibles
 import-font-missing = { $family } no encontrada: el texto usa { $by }
 import-font-restricted = { $family }: su licencia prohíbe incrustar algunas de sus variantes en la presentación
```

`hi`:

```diff
--- a/crates/oku-slides-i18n/src/locales/hi/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/hi/document.ftl
@@ -81,6 +81,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = एक पैक के लिए बहुत अधिक लेआउट: “{ $layout }” का कोई टेम्पलेट नहीं; इसकी स्लाइडें खाली विजेट लेती हैं और इसकी सजावट तत्वों के रूप में पाती हैं
 import-loss-decor-truncated = पैक फ़ाइल के आकार में समाने के लिए सजावट काटी गई: छोड़े गए तत्व: { $omitted }
 import-loss-table-style = तालिका की शैली अनुमानित: इसकी अपनी सेल पृष्ठभूमि, किनारे, स्तंभ चौड़ाई या पाठ आकार खो गए
+import-loss-overflow = तत्व “{ $element }” स्लाइड के किनारे से बाहर जाता है
+import-reference-unavailable = कोई दृश्य स्कोर नहीं: { $cause ->
+    [timeout] संदर्भ रेंडर में बहुत समय लगा
+    [memory] संदर्भ रेंडर ने बहुत अधिक मेमोरी ली
+    [pages] संदर्भ रेंडर में प्रस्तुति के { $deck } के बदले { $reference } पेज हैं
+   *[failed] संदर्भ रेंडरर प्रस्तुति नहीं पढ़ सका
+}
 import-font-substituted = { $family } को मिलते-जुलते माप वाले { $by } से बदला गया
 import-font-missing = { $family } नहीं मिला: टेक्स्ट { $by } का उपयोग करता है
 import-font-restricted = { $family }: इसका लाइसेंस कुछ शैलियों को प्रेज़ेंटेशन में एम्बेड करने से रोकता है
```

`it`:

```diff
--- a/crates/oku-slides-i18n/src/locales/it/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/it/document.ftl
@@ -80,6 +80,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Troppi layout per un pacchetto: «{ $layout }» non ha un modello; le sue slide usano il widget vuoto e ricevono la sua decorazione come elementi
 import-loss-decor-truncated = Decorazione tagliata per stare nella dimensione di un file del pacchetto: { $omitted } elementi omessi
 import-loss-table-style = Stile della tabella approssimato: si perdono sfondi delle celle, bordi, larghezze delle colonne o dimensioni del testo propri
+import-loss-overflow = L’elemento «{ $element }» supera il bordo della slide
+import-reference-unavailable = Nessun punteggio visivo: { $cause ->
+    [timeout] il rendering di riferimento ha richiesto troppo tempo
+    [memory] il rendering di riferimento ha usato troppa memoria
+    [pages] il rendering di riferimento ha { $reference } pagine per { $deck } nella presentazione
+   *[failed] il renderer di riferimento non ha potuto leggere la presentazione
+}
 import-font-substituted = { $family } sostituito da { $by }, con metriche compatibili
 import-font-missing = { $family } non trovato: il testo usa { $by }
 import-font-restricted = { $family }: la licenza vieta di incorporare alcune sue varianti nella presentazione
```

`ja`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ja/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/ja/document.ftl
@@ -77,6 +77,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = 1つのパックにはレイアウトが多すぎます：「{ $layout }」にはテンプレートがなく、そのスライドは空のウィジェットを使い、装飾を要素として受け取ります
 import-loss-decor-truncated = パックファイルのサイズに収めるため装飾を切り詰めました：{ $omitted }個の要素を省きました
 import-loss-table-style = 表のスタイルを近似しました：独自のセルの背景、罫線、列幅、文字サイズは失われます
+import-loss-overflow = 要素「{ $element }」がスライドの端からはみ出しています
+import-reference-unavailable = 見た目の差のスコアなし：{ $cause ->
+    [timeout] 参照レンダリングに時間がかかりすぎました
+    [memory] 参照レンダリングがメモリを使いすぎました
+    [pages] 参照レンダリングは { $reference }ページで、デッキは{ $deck }ページです
+   *[failed] 参照レンダラーがプレゼンテーションを読めませんでした
+}
 import-font-substituted = { $family } をメトリクス互換の { $by } に置き換えました
 import-font-missing = { $family } が見つかりません。テキストは { $by } を使います
 import-font-restricted = { $family }：ライセンスにより一部のスタイルをプレゼンテーションに埋め込めません
```

`ko`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ko/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/ko/document.ftl
@@ -78,6 +78,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = 한 팩에 레이아웃이 너무 많습니다: “{ $layout }”에는 템플릿이 없어 해당 슬라이드는 빈 위젯을 쓰고 장식을 요소로 받습니다
 import-loss-decor-truncated = 팩 파일 크기에 맞추려고 장식을 잘랐습니다: 빠진 요소 { $omitted }개
 import-loss-table-style = 표 스타일을 근사했습니다: 고유한 셀 배경, 테두리, 열 너비 또는 글자 크기가 사라집니다
+import-loss-overflow = 요소 “{ $element }”이(가) 슬라이드 가장자리를 벗어납니다
+import-reference-unavailable = 시각 점수 없음: { $cause ->
+    [timeout] 참조 렌더링이 너무 오래 걸렸습니다
+    [memory] 참조 렌더링이 메모리를 너무 많이 사용했습니다
+    [pages] 참조 렌더링은 { $reference }쪽이고 덱은 { $deck }쪽입니다
+   *[failed] 참조 렌더러가 프레젠테이션을 읽지 못했습니다
+}
 import-font-substituted = { $family }을(를) 메트릭이 호환되는 { $by }(으)로 바꿨습니다
 import-font-missing = { $family }을(를) 찾을 수 없습니다. 텍스트는 { $by } 글꼴을 사용합니다
 import-font-restricted = { $family }: 라이선스상 일부 스타일을 프레젠테이션에 포함할 수 없습니다
```

`nl`:

```diff
--- a/crates/oku-slides-i18n/src/locales/nl/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/nl/document.ftl
@@ -81,6 +81,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Te veel lay-outs voor één pakket: ‘{ $layout }’ heeft geen sjabloon; de dia's ervan gebruiken de lege widget en krijgen de decoratie als elementen
 import-loss-decor-truncated = Decoratie ingekort om in de grootte van een pakketbestand te passen: { $omitted } elementen weggelaten
 import-loss-table-style = Tabelstijl benaderd: eigen celachtergronden, randen, kolombreedtes of tekstgroottes gaan verloren
+import-loss-overflow = Element ‘{ $element }’ steekt buiten de rand van de dia
+import-reference-unavailable = Geen visuele score: { $cause ->
+    [timeout] de referentieweergave duurde te lang
+    [memory] de referentieweergave gebruikte te veel geheugen
+    [pages] de referentieweergave heeft { $reference } pagina’s voor { $deck } in de presentatie
+   *[failed] de referentierenderer kon de presentatie niet lezen
+}
 import-font-substituted = { $family } vervangen door { $by }, met passende metrieken
 import-font-missing = { $family } niet gevonden: de tekst gebruikt { $by }
 import-font-restricted = { $family }: de licentie verbiedt sommige stijlen in de presentatie in te sluiten
```

`pl`:

```diff
--- a/crates/oku-slides-i18n/src/locales/pl/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/pl/document.ftl
@@ -83,6 +83,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Za dużo układów na jeden pakiet: „{ $layout }” nie ma szablonu; jego slajdy używają pustego widżetu i dostają jego dekorację jako elementy
 import-loss-decor-truncated = Dekoracja przycięta, by zmieścić się w rozmiarze pliku pakietu: pominięte elementy: { $omitted }
 import-loss-table-style = Styl tabeli przybliżony: jej własne tła komórek, obramowania, szerokości kolumn lub rozmiary tekstu są tracone
+import-loss-overflow = Element „{ $element }” wychodzi poza krawędź slajdu
+import-reference-unavailable = Brak oceny wizualnej: { $cause ->
+    [timeout] renderowanie referencyjne trwało zbyt długo
+    [memory] renderowanie referencyjne zużyło zbyt dużo pamięci
+    [pages] renderowanie referencyjne ma { $reference } stron na { $deck } w prezentacji
+   *[failed] renderer referencyjny nie mógł odczytać prezentacji
+}
 import-font-substituted = { $family } zastąpiona czcionką { $by } o zgodnych metrykach
 import-font-missing = Nie znaleziono { $family }: tekst używa czcionki { $by }
 import-font-restricted = { $family }: licencja zabrania osadzania niektórych jej odmian w prezentacji
```

`pt`:

```diff
--- a/crates/oku-slides-i18n/src/locales/pt/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/pt/document.ftl
@@ -81,6 +81,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Layouts demais para um pacote: «{ $layout }» não tem modelo; seus slides usam o widget vazio e recebem sua decoração como elementos
 import-loss-decor-truncated = Decoração cortada para caber no tamanho de um arquivo do pacote: { $omitted } elementos omitidos
 import-loss-table-style = Estilo da tabela aproximado: seus fundos de célula, bordas, larguras de coluna ou tamanhos de texto próprios se perdem
+import-loss-overflow = O elemento “{ $element }” ultrapassa a borda do slide
+import-reference-unavailable = Sem pontuação visual: { $cause ->
+    [timeout] a renderização de referência demorou demais
+    [memory] a renderização de referência usou memória demais
+    [pages] a renderização de referência tem { $reference } páginas para { $deck } na apresentação
+   *[failed] o renderizador de referência não conseguiu ler a apresentação
+}
 import-font-substituted = { $family } substituída por { $by }, com métricas compatíveis
 import-font-missing = { $family } não encontrada: o texto usa { $by }
 import-font-restricted = { $family }: a licença proíbe incorporar algumas das suas variantes na apresentação
```

`ru`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ru/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/ru/document.ftl
@@ -83,6 +83,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Слишком много макетов для одного пакета: у «{ $layout }» нет шаблона; его слайды используют пустой виджет и получают его оформление как элементы
 import-loss-decor-truncated = Оформление урезано, чтобы уложиться в размер файла пакета: пропущено элементов: { $omitted }
 import-loss-table-style = Стиль таблицы приближён: собственные фоны ячеек, границы, ширина столбцов или размеры текста потеряны
+import-loss-overflow = Элемент «{ $element }» выходит за край слайда
+import-reference-unavailable = Нет визуальной оценки: { $cause ->
+    [timeout] эталонный рендеринг занял слишком много времени
+    [memory] эталонный рендеринг потребовал слишком много памяти
+    [pages] в эталонном рендеринге { $reference } стр. при { $deck } в презентации
+   *[failed] эталонный рендерер не смог прочитать презентацию
+}
 import-font-substituted = { $family } заменён на { $by } с совместимыми метриками
 import-font-missing = { $family } не найден: текст использует шрифт { $by }
 import-font-restricted = { $family }: лицензия запрещает встраивать некоторые начертания в презентацию
```

`th`:

```diff
--- a/crates/oku-slides-i18n/src/locales/th/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/th/document.ftl
@@ -78,6 +78,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = เลย์เอาต์มากเกินไปสำหรับแพ็กเดียว: “{ $layout }” ไม่มีเทมเพลต สไลด์ของมันใช้วิดเจ็ตว่างและได้รับการตกแต่งเป็นองค์ประกอบ
 import-loss-decor-truncated = ตัดการตกแต่งให้พอดีกับขนาดไฟล์แพ็ก: ละองค์ประกอบ { $omitted } รายการ
 import-loss-table-style = รูปแบบตารางเป็นค่าประมาณ: พื้นหลังเซลล์ เส้นขอบ ความกว้างคอลัมน์ หรือขนาดข้อความเฉพาะของตารางหายไป
+import-loss-overflow = องค์ประกอบ “{ $element }” เกินขอบสไลด์
+import-reference-unavailable = ไม่มีคะแนนภาพ: { $cause ->
+    [timeout] การเรนเดอร์อ้างอิงใช้เวลานานเกินไป
+    [memory] การเรนเดอร์อ้างอิงใช้หน่วยความจำมากเกินไป
+    [pages] การเรนเดอร์อ้างอิงมี { $reference } หน้า แต่เด็คมี { $deck } หน้า
+   *[failed] ตัวเรนเดอร์อ้างอิงอ่านงานนำเสนอไม่ได้
+}
 import-font-substituted = แทนที่ { $family } ด้วย { $by } ที่มีเมตริกเข้ากันได้
 import-font-missing = ไม่พบ { $family }: ข้อความใช้ฟอนต์ { $by }
 import-font-restricted = { $family }: สัญญาอนุญาตห้ามฝังบางสไตล์ลงในงานนำเสนอ
```

`tr`:

```diff
--- a/crates/oku-slides-i18n/src/locales/tr/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/tr/document.ftl
@@ -78,6 +78,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Bir paket için çok fazla düzen: “{ $layout }” için şablon yok; slaytları boş widget'ı kullanır ve süslemesini öğe olarak alır
 import-loss-decor-truncated = Süsleme bir paket dosyasının boyutuna sığması için kısaltıldı: { $omitted } öğe atlandı
 import-loss-table-style = Tablo stili yaklaşık: kendi hücre arka planları, kenarlıkları, sütun genişlikleri veya metin boyutları kayboldu
+import-loss-overflow = “{ $element }” öğesi slaytın kenarını aşıyor
+import-reference-unavailable = Görsel puan yok: { $cause ->
+    [timeout] referans oluşturma çok uzun sürdü
+    [memory] referans oluşturma çok fazla bellek kullandı
+    [pages] referans oluşturmada sunudaki { $deck } sayfaya karşılık { $reference } sayfa var
+   *[failed] referans oluşturucu sunuyu okuyamadı
+}
 import-font-substituted = { $family } yerine uyumlu ölçülere sahip { $by } kullanıldı
 import-font-missing = { $family } bulunamadı: metin { $by } yazı tipini kullanıyor
 import-font-restricted = { $family }: lisansı bazı stillerin sunuya gömülmesini yasaklıyor
```

`uk`:

```diff
--- a/crates/oku-slides-i18n/src/locales/uk/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/uk/document.ftl
@@ -83,6 +83,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Забагато макетів для одного пакета: у «{ $layout }» немає шаблону; його слайди використовують порожній віджет і отримують його оформлення як елементи
 import-loss-decor-truncated = Оформлення скорочено, щоб вміститися в розмір файлу пакета: пропущено елементів: { $omitted }
 import-loss-table-style = Стиль таблиці наближено: власні тла клітинок, межі, ширина стовпців або розміри тексту втрачено
+import-loss-overflow = Елемент «{ $element }» виходить за край слайда
+import-reference-unavailable = Немає візуальної оцінки: { $cause ->
+    [timeout] еталонний рендеринг тривав надто довго
+    [memory] еталонний рендеринг використав надто багато пам’яті
+    [pages] в еталонному рендерингу { $reference } стор. при { $deck } у презентації
+   *[failed] еталонний рендерер не зміг прочитати презентацію
+}
 import-font-substituted = { $family } замінено на { $by } із сумісними метриками
 import-font-missing = { $family } не знайдено: текст використовує шрифт { $by }
 import-font-restricted = { $family }: ліцензія забороняє вбудовувати деякі накреслення у презентацію
```

`vi`:

```diff
--- a/crates/oku-slides-i18n/src/locales/vi/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/vi/document.ftl
@@ -78,6 +78,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = Quá nhiều bố cục cho một gói: “{ $layout }” không có mẫu; các trang chiếu của nó dùng widget trống và nhận phần trang trí dưới dạng phần tử
 import-loss-decor-truncated = Phần trang trí bị cắt để vừa kích thước tệp gói: bỏ qua { $omitted } phần tử
 import-loss-table-style = Kiểu bảng được ước lượng: nền ô, đường viền, độ rộng cột hoặc cỡ chữ riêng của bảng bị mất
+import-loss-overflow = Phần tử “{ $element }” vượt quá mép trang chiếu
+import-reference-unavailable = Không có điểm hình ảnh: { $cause ->
+    [timeout] bản kết xuất tham chiếu mất quá nhiều thời gian
+    [memory] bản kết xuất tham chiếu dùng quá nhiều bộ nhớ
+    [pages] bản kết xuất tham chiếu có { $reference } trang so với { $deck } trong bản trình bày
+   *[failed] trình kết xuất tham chiếu không đọc được bản trình bày
+}
 import-font-substituted = Đã thay { $family } bằng { $by }, có số đo tương thích
 import-font-missing = Không tìm thấy { $family }: văn bản dùng phông chữ { $by }
 import-font-restricted = { $family }: giấy phép cấm nhúng một số kiểu của phông vào bản trình bày
```

`zh-Hans`:

```diff
--- a/crates/oku-slides-i18n/src/locales/zh-Hans/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/zh-Hans/document.ftl
@@ -78,6 +78,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = 一个包中的版式过多：“{ $layout }”没有模板，其幻灯片使用空白组件，并以元素形式获得其装饰
 import-loss-decor-truncated = 为适应包文件大小而截断了装饰：省略了{ $omitted }个元素
 import-loss-table-style = 表格样式为近似：其自身的单元格背景、边框、列宽或文字大小已丢失
+import-loss-overflow = 元素“{ $element }”超出了幻灯片边缘
+import-reference-unavailable = 无视觉评分：{ $cause ->
+    [timeout] 参考渲染耗时过长
+    [memory] 参考渲染占用内存过多
+    [pages] 参考渲染有{ $reference }页，而演示文稿有{ $deck }页
+   *[failed] 参考渲染器无法读取该演示文稿
+}
 import-font-substituted = { $family } 已替换为度量兼容的 { $by }
 import-font-missing = 找不到 { $family }：文本使用 { $by }
 import-font-restricted = { $family }：其许可证禁止将部分字形嵌入演示文稿
```

`zh-Hant`:

```diff
--- a/crates/oku-slides-i18n/src/locales/zh-Hant/document.ftl
+++ b/crates/oku-slides-i18n/src/locales/zh-Hant/document.ftl
@@ -78,6 +78,13 @@ import-loss-shape-style = { $effect ->
 import-loss-layout-dropped = 一個套件中的版面配置過多：「{ $layout }」沒有範本，其投影片使用空白元件，並以元素形式取得其裝飾
 import-loss-decor-truncated = 為符合套件檔案大小而截斷了裝飾：省略了{ $omitted }個元素
 import-loss-table-style = 表格樣式為近似：其自身的儲存格背景、框線、欄寬或文字大小已遺失
+import-loss-overflow = 元素「{ $element }」超出了投影片邊緣
+import-reference-unavailable = 無視覺評分：{ $cause ->
+    [timeout] 參考轉譯耗時過長
+    [memory] 參考轉譯佔用記憶體過多
+    [pages] 參考轉譯有{ $reference }頁，而簡報有{ $deck }頁
+   *[failed] 參考轉譯器無法讀取該簡報
+}
 import-font-substituted = { $family } 已替換為度量相容的 { $by }
 import-font-missing = 找不到 { $family }：文字使用 { $by }
 import-font-restricted = { $family }：其授權禁止將部分字樣內嵌到簡報
```

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import -p oku-slides-i18n`
Expected: all pass, including `cause_localize_in_every_locale_formats_its_own_message` (4 cases x 20 locales), `loss_localize_in_every_locale_formats_its_own_message` for `Overflow`, and the new `to_json_*`, `above_threshold_*`, `mean_score_*` tests.

Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-import --all-targets -- -D warnings`
Expected: no warning. Any other crate that builds a `SlideReport` or `ImportReport` by struct literal fails to compile here; the draft needed none outside `retrofit/slide.rs` and `hifi/slide.rs`.

Run: `cargo fmt -p oku-slides-import -- --check`
Expected: no diff.

Run: `wc -l crates/oku-slides-import/src/report.rs`
Expected: under 500 lines (494 in the draft).

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-import/src/report.rs crates/oku-slides-import/src/report_tests.rs crates/oku-slides-import/src/report/fonts.rs crates/oku-slides-import/src/report/reference.rs crates/oku-slides-import/src/report/reference_tests.rs crates/oku-slides-import/src/retrofit/slide.rs crates/oku-slides-import/src/hifi/slide.rs crates/oku-slides-i18n/src/locales/ar/document.ftl crates/oku-slides-i18n/src/locales/bn/document.ftl crates/oku-slides-i18n/src/locales/de/document.ftl crates/oku-slides-i18n/src/locales/en/document.ftl crates/oku-slides-i18n/src/locales/es/document.ftl crates/oku-slides-i18n/src/locales/fr/document.ftl crates/oku-slides-i18n/src/locales/hi/document.ftl crates/oku-slides-i18n/src/locales/it/document.ftl crates/oku-slides-i18n/src/locales/ja/document.ftl crates/oku-slides-i18n/src/locales/ko/document.ftl crates/oku-slides-i18n/src/locales/nl/document.ftl crates/oku-slides-i18n/src/locales/pl/document.ftl crates/oku-slides-i18n/src/locales/pt/document.ftl crates/oku-slides-i18n/src/locales/ru/document.ftl crates/oku-slides-i18n/src/locales/th/document.ftl crates/oku-slides-i18n/src/locales/tr/document.ftl crates/oku-slides-i18n/src/locales/uk/document.ftl crates/oku-slides-i18n/src/locales/vi/document.ftl crates/oku-slides-i18n/src/locales/zh-Hans/document.ftl crates/oku-slides-i18n/src/locales/zh-Hant/document.ftl
```

Message file (written first), then the commit as a separate command:

```
feat(import) : rapport du rendu de référence, score par slide et perte de débordement (t-692)
```

```bash
git commit -F <message file>
```

**Extracting from the draft:** instead of retyping, take the files of this task from the draft commit `40a05b9d`:
`git checkout 40a05b9d -- crates/oku-slides-import/src/hifi/slide.rs crates/oku-slides-import/src/report.rs crates/oku-slides-import/src/report/fonts.rs crates/oku-slides-import/src/report/reference.rs crates/oku-slides-import/src/report/reference_tests.rs crates/oku-slides-import/src/report_tests.rs crates/oku-slides-import/src/retrofit/slide.rs` plus the 20 files `crates/oku-slides-i18n/src/locales/<tag>/document.ftl` (explicit list, never a directory). Afterwards, in `ja`, `zh-Hans` and `zh-Hant` `document.ftl`, remove the spaces around `{ $reference }` and `{ $deck }` on the `[pages]` line so they match the diffs above (or extract those three files from `e33bc28` instead, which already carries the final text). `crates/oku-slides-import/ARCHITECTURE.md` is in the draft commit but must not be extracted (Task 7). Of the files above, none was changed on `main` since the draft base `53454df2` (`git diff --name-only 53454df2 main`) apart from `ARCHITECTURE.md`; if that no longer holds when you run it, apply the file with `git show 40a05b9d -- <file> | git apply --3way` instead. Do not extract `report_tests.rs` from `e33bc28` or later: it carries the `shift` field of a later task.

### Task 5: Region score: owned pixels, realignment, worst tile (calibrated on alto)

This task rewrites the region scoring of `metrics.rs`. A plain mean of the gap over an item's frame was not enough on alto: a 1-2 px systematic baseline shift between the witness and Oku drowned the p. 7 callout line break; tiny frames (a 32x2 accent bar) gave antialiasing noise; containers inherited the gap of their text; and the mean over a large, mostly empty frame diluted a moved word. The score now uses owned pixels (each pixel belongs to the topmost item covering it), an integer realignment of at most `MAX_SHIFT` px (reported as `RegionScore.shift`), the mean gap of the worst half-overlapping `TILE` px tile, and no score under `MIN_PIXELS` own pixels. The OKLab conversion is memoized per color.

Calibration result (on alto): on p. 8 the bullet lists of pillars 2 and 3 are among the 3 worst regions (shift -3 px, the 27 pt indent); on p. 7 the callout (`s7/21`) is 2nd behind the title, which the witness draws about 1 % wider (a witness limit).

The draft commit also touches three `document.ftl` hunks (ja, zh-Hans, zh-Hant): they are already in Task 4 and are excluded here.

**Files:**
- Modify: `crates/oku-slides-import/src/metrics.rs` (constants `MAX_SHIFT`, `TILE`, `MIN_PIXELS`, `ALIGN_STEP`; `ownership`, `bounds`, `Pair::region`; `RegionScore.shift`; OKLab memo)
- Modify: `crates/oku-slides-import/src/metrics_tests.rs` (new region-score tests, `with_rect`)
- Modify: `crates/oku-slides-import/src/report_tests.rs` (two lines: the new `shift` field)
- `crates/oku-slides-import/ARCHITECTURE.md`: updated in Task 7 (English).

**Interfaces:**
- Consumes: nothing new (`retrofit::color::{Oklab, delta_e, oklab}`, `source::{Item, ItemKind, Rgba, SourceSlide}` already exist).
- Produces: `pub const MAX_SHIFT: i64 = 3`, `pub const TILE: usize = 16`, `pub const MIN_PIXELS: usize = 100`; `RegionScore { item: String, score: f64, shift: [i64; 2] }` (`shift` = `[dx, dy]`, where Oku draws the region relative to the reference; serialized in the JSON report); `regions()` returns regions in drawing order (last on top); `score(reference, oku, regions) -> Option<SlideScore>` keeps its signature.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/crates/oku-slides-import/src/metrics_tests.rs
+++ b/crates/oku-slides-import/src/metrics_tests.rs
@@ -9,10 +9,10 @@ fn plain(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
     (0..w * h).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect()
 }
 
-/// `rgba` (`w` de large) avec le carré `x, y, side` peint en `rgb`.
-fn with_square(mut rgba: Vec<u8>, w: u32, (x, y, side): (u32, u32, u32), rgb: [u8; 3]) -> Vec<u8> {
-    for row in y..y + side {
-        for col in x..x + side {
+/// `rgba` (`w` de large) avec le rectangle `x, y, rw, rh` peint en `rgb`.
+fn with_rect(mut rgba: Vec<u8>, w: u32, (x, y, rw, rh): (u32, u32, u32, u32), rgb: [u8; 3]) -> Vec<u8> {
+    for row in y..y + rh {
+        for col in x..x + rw {
             let i = ((row * w + col) * 4) as usize;
             rgba[i..i + 3].copy_from_slice(&rgb);
         }
@@ -28,11 +28,15 @@ fn region(item: &str, (x, y, w, h): (f64, f64, f64, f64)) -> Region {
     Region { item: item.to_owned(), x, y, w, h }
 }
 
+fn items(s: &SlideScore) -> Vec<&str> {
+    s.regions.iter().map(|r| r.item.as_str()).collect()
+}
+
 #[test]
 fn score_of_identical_images_is_zero() {
     let white = plain(40, 20, [255, 255, 255]);
-    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("a", (0.0, 0.0, 10.0, 10.0))]).unwrap();
-    assert_that!((s.score, s.regions[0].score)).is_equal_to((0.0, 0.0));
+    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("a", (0.0, 0.0, 20.0, 20.0))]).unwrap();
+    assert_that!((s.score, s.regions[0].score, s.regions[0].shift)).is_equal_to((0.0, 0.0, [0, 0]));
 }
 
 #[test]
@@ -45,35 +49,70 @@ fn score_of_white_against_black_is_one_hundred() {
 #[test]
 fn score_designates_the_region_that_differs() {
     let white = plain(100, 50, [255, 255, 255]);
-    let marked = with_square(white.clone(), 100, (20, 20, 10), [255, 0, 0]);
-    let regions = [region("fond", (0.0, 0.0, 100.0, 50.0)), region("carre", (20.0, 20.0, 10.0, 10.0))];
+    let marked = with_rect(white.clone(), 100, (60, 20, 12, 12), [255, 0, 0]);
+    let regions = [region("gauche", (0.0, 0.0, 50.0, 50.0)), region("droite", (50.0, 0.0, 50.0, 50.0))];
     let s = score(image(100, 50, &white), image(100, 50, &marked), &regions).unwrap();
-    let order: Vec<&str> = s.regions.iter().map(|r| r.item.as_str()).collect();
-    assert_that!(order).is_equal_to(vec!["carre", "fond"]);
+    assert_that!(items(&s)).is_equal_to(vec!["droite", "gauche"]);
 }
 
 #[test]
-fn score_blur_softens_a_one_pixel_shift() {
-    let white = plain(60, 30, [255, 255, 255]);
+fn score_realigns_a_small_shift_and_reports_it() {
+    let white = plain(60, 40, [255, 255, 255]);
     let (a, b) =
-        (with_square(white.clone(), 60, (20, 10, 6), [0, 0, 0]), with_square(white, 60, (21, 10, 6), [0, 0, 0]));
-    let s = score(image(60, 30, &a), image(60, 30, &b), &[region("carre", (20.0, 10.0, 7.0, 6.0))]).unwrap();
-    // Sans flou, deux colonnes sur sept diffèrent de 100 : 28,6.
-    assert_that!(s.regions[0].score).is_less_than(20.0);
+        (with_rect(white.clone(), 60, (20, 10, 12, 12), [0, 0, 0]), with_rect(white, 60, (22, 11, 12, 12), [0, 0, 0]));
+    let s = score(image(60, 40, &a), image(60, 40, &b), &[region("carre", (10.0, 0.0, 40.0, 40.0))]).unwrap();
+    assert_that!((s.regions[0].score, s.regions[0].shift)).is_equal_to((0.0, [2, 1]));
+}
+
+#[test]
+fn score_keeps_a_shift_beyond_max_shift_as_a_gap() {
+    let white = plain(80, 40, [255, 255, 255]);
+    let (a, b) =
+        (with_rect(white.clone(), 80, (10, 10, 12, 12), [0, 0, 0]), with_rect(white, 80, (40, 10, 12, 12), [0, 0, 0]));
+    let s = score(image(80, 40, &a), image(80, 40, &b), &[region("carre", (0.0, 0.0, 80.0, 40.0))]).unwrap();
+    assert_that!(s.regions[0].score).is_greater_than(20.0);
+}
+
+#[test]
+fn score_gives_a_local_gap_its_full_weight_in_a_large_region() {
+    let white = plain(400, 100, [255, 255, 255]);
+    let marked = with_rect(white.clone(), 400, (300, 40, 16, 16), [0, 0, 0]);
+    let s =
+        score(image(400, 100, &white), image(400, 100, &marked), &[region("cadre", (0.0, 0.0, 400.0, 100.0))]).unwrap();
+    // Sur tout le cadre, l'écart moyen vaudrait 0,64 ; sa pire tuile le porte entier.
+    assert_that!(s.regions[0].score).is_greater_than(40.0);
+}
+
+#[test]
+fn score_gives_the_gap_to_the_item_on_top() {
+    let white = plain(100, 60, [255, 255, 255]);
+    let marked = with_rect(white.clone(), 100, (40, 20, 16, 16), [0, 0, 0]);
+    let regions = [region("carte", (0.0, 0.0, 100.0, 60.0)), region("texte", (30.0, 10.0, 40.0, 40.0))];
+    let s = score(image(100, 60, &white), image(100, 60, &marked), &regions).unwrap();
+    let carte = s.regions.iter().find(|r| r.item == "carte").map(|r| r.score);
+    assert_that!((items(&s)[0], carte)).is_equal_to(("texte", Some(0.0)));
+}
+
+#[test]
+fn score_skips_a_region_with_too_few_pixels_of_its_own() {
+    let white = plain(40, 20, [255, 255, 255]);
+    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("filet", (0.0, 0.0, 30.0, 2.0))]).unwrap();
+    assert_that!(s.regions).is_empty();
 }
 
 #[test]
 fn score_keeps_the_worst_regions_only() {
-    let white = plain(40, 10, [255, 255, 255]);
-    let regions: Vec<Region> = (0..5).map(|i| region(&format!("r{i}"), (f64::from(i) * 8.0, 0.0, 8.0, 10.0))).collect();
-    let s = score(image(40, 10, &white), image(40, 10, &white), &regions).unwrap();
+    let white = plain(100, 20, [255, 255, 255]);
+    let regions: Vec<Region> =
+        (0..5).map(|i| region(&format!("r{i}"), (f64::from(i) * 20.0, 0.0, 20.0, 20.0))).collect();
+    let s = score(image(100, 20, &white), image(100, 20, &white), &regions).unwrap();
     assert_that!(s.regions.len()).is_equal_to(REPORTED_REGIONS);
 }
 
 #[test]
 fn score_skips_a_region_outside_the_image() {
     let white = plain(20, 10, [255, 255, 255]);
-    let s = score(image(20, 10, &white), image(20, 10, &white), &[region("dehors", (30.0, 0.0, 5.0, 5.0))]).unwrap();
+    let s = score(image(20, 10, &white), image(20, 10, &white), &[region("dehors", (30.0, 0.0, 50.0, 50.0))]).unwrap();
     assert_that!(s.regions).is_empty();
 }
 
```
```diff
--- a/crates/oku-slides-import/src/report_tests.rs
+++ b/crates/oku-slides-import/src/report_tests.rs
@@ -350,7 +350,7 @@ fn loss_localize_overflow_names_the_element() {
 }
 
 fn scored(n: u32, score: f64) -> SlideReport {
-    let regions = vec![crate::metrics::RegionScore { item: format!("s{n}/1"), score }];
+    let regions = vec![crate::metrics::RegionScore { item: format!("s{n}/1"), score, shift: [0, 0] }];
     SlideReport { source: n, slide: format!("s{n}"), score: Some(SlideScore { score, regions }), ..slide(vec![]) }
 }
 
@@ -370,7 +370,7 @@ fn to_json_marks_the_slides_above_the_threshold() {
 fn to_json_writes_the_score_and_its_regions() {
     let v = with_reference(vec![scored(1, 2.0)], None).to_json(english());
     assert_that!(v["slides"][0]["score"].clone())
-        .is_equal_to(json!({"score": 2.0, "regions": [{"item": "s1/1", "score": 2.0}]}));
+        .is_equal_to(json!({"score": 2.0, "regions": [{"item": "s1/1", "score": 2.0, "shift": [0, 0]}]}));
 }
 
 #[test]
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import metrics report`
Expected: compile error, `RegionScore` has no field `shift`.

- [ ] **Step 3: Implement**

Region scoring in `metrics.rs`: module doc, constants, `RegionScore.shift`, ownership, realignment, worst tile, color memo.

```diff
--- a/crates/oku-slides-import/src/metrics.rs
+++ b/crates/oku-slides-import/src/metrics.rs
@@ -2,22 +2,49 @@
 //! référence d'un témoin, à la même taille (spec, « Rendu de référence et
 //! métriques »).
 //!
-//! Les deux images passent en OKLab, sont adoucies par un flou gaussien
-//! ([`BLUR_SIGMA`] pixels : l'anticrénelage et un décalage d'un pixel ne
-//! comptent presque pas), puis l'écart de chaque pixel est le ΔE OKLab (×100,
-//! celui de `retrofit::color`). Le score d'une slide est l'écart moyen ; celui
-//! d'une région, l'écart moyen sur le cadre d'un item source ([`regions`]),
-//! pour désigner l'élément fautif. Le rapport garde les [`REPORTED_REGIONS`]
-//! pires. Pur : les images viennent de l'appelant.
+//! Les deux images passent en OKLab et sont adoucies par un flou gaussien
+//! ([`BLUR_SIGMA`] pixels : l'anticrénelage compte peu) ; l'écart d'un pixel
+//! est le ΔE OKLab (×100, celui de `retrofit::color`). Le score d'une slide est
+//! l'écart moyen sur la page.
+//!
+//! Le score d'une région désigne l'élément fautif ; il est calibré sur alto
+//! (p. 7 : la ligne coupée de l'encadré ; p. 8 : le retrait des puces) :
+//! - chaque pixel appartient à l'item le plus haut qui le couvre (ordre de
+//!   dessin de [`regions`]) : une carte ne porte pas l'écart de son texte ;
+//! - la région est recalée sur la référence (le décalage entier, au plus
+//!   [`MAX_SHIFT`] pixels, qui minimise son écart moyen ; il est rapporté) :
+//!   un décalage de quelques pixels, qui se corrige seul, ne masque pas une
+//!   coupure ou un retrait ;
+//! - son score est l'écart moyen de sa pire tuile de [`TILE`] pixels : un mot
+//!   déplacé dans un grand cadre presque vide compte pleinement ;
+//! - une région de moins de [`MIN_PIXELS`] pixels à elle n'a pas de score
+//!   (ses pixels sont surtout des bords anticrénelés).
+//!
+//! Le rapport garde les [`REPORTED_REGIONS`] pires. Pur : les images viennent
+//! de l'appelant.
+
+use std::collections::{BTreeMap, HashMap};
 
 use serde::Serialize;
 
-use crate::retrofit::color::{delta_e, oklab};
+use crate::retrofit::color::{Oklab, delta_e, oklab};
 use crate::source::{Item, ItemKind, Rgba, SourceSlide};
 
 /// Écart-type du flou gaussien, en pixels (à 960 px de large).
 pub const BLUR_SIGMA: f64 = 1.0;
 
+/// Décalage maximal, en pixels et par axe, du recalage d'une région.
+pub const MAX_SHIFT: i64 = 3;
+
+/// Un pixel propre sur `ALIGN_STEP` sert à chercher le recalage d'une région.
+const ALIGN_STEP: usize = 4;
+
+/// Côté d'une tuile, en pixels.
+pub const TILE: usize = 16;
+
+/// Pixels propres en deçà desquels une région n'a pas de score.
+pub const MIN_PIXELS: usize = 100;
+
 /// Nombre de régions gardées par slide, les pires d'abord.
 pub const REPORTED_REGIONS: usize = 3;
 
@@ -52,8 +79,11 @@ pub struct Region {
 pub struct RegionScore {
     /// Id de l'item source.
     pub item: String,
-    /// ΔE OKLab moyen sur son cadre, arrondi au centième.
+    /// ΔE OKLab moyen de sa pire tuile, une fois recalée, arrondi au centième.
     pub score: f64,
+    /// Le recalage retenu, en pixels (`[dx, dy]` : où le rendu Oku la dessine
+    /// par rapport à la référence).
+    pub shift: [i64; 2],
 }
 
 /// L'écart visuel d'une slide.
@@ -66,7 +96,8 @@ pub struct SlideScore {
 }
 
 /// Les régions de `slide` : le cadre de chaque item, les groupes ouverts
-/// (leurs enfants seulement), à `scale` pixels par pt de la page source.
+/// (leurs enfants seulement), à `scale` pixels par pt de la page source, dans
+/// l'ordre de dessin (la dernière est au-dessus).
 pub fn regions(slide: &SourceSlide, scale: f64) -> Vec<Region> {
     fn walk(items: &[Item], scale: f64, out: &mut Vec<Region>) {
         for item in items {
@@ -95,57 +126,129 @@ pub fn regions(slide: &SourceSlide, scale: f64) -> Vec<Region> {
 pub fn score(reference: Image<'_>, oku: Image<'_>, regions: &[Region]) -> Option<SlideScore> {
     let (w, h) = (reference.width as usize, reference.height as usize);
     let valid = |i: Image<'_>| i.rgba.len() == w * h * 4;
-    if (oku.width, oku.height) != (reference.width, reference.height)
-        || w == 0
-        || h == 0
-        || !valid(reference)
-        || !valid(oku)
-    {
+    let same = (oku.width, oku.height) == (reference.width, reference.height);
+    if !same || w == 0 || h == 0 || !valid(reference) || !valid(oku) {
         return None;
     }
-    let (a, b) = (blurred(reference), blurred(oku));
-    let diff: Vec<f64> = a.iter().zip(&b).map(|(x, y)| delta_e(*x, *y)).collect();
-    let mut scored: Vec<RegionScore> = regions
+    let pair = Pair { a: blurred(reference), b: blurred(oku), w, h };
+    let mean = (0..w * h).map(|i| delta_e(pair.a[i], pair.b[i])).sum::<f64>() / (w * h) as f64;
+    let mut scored: Vec<RegionScore> = ownership(regions, w, h)
         .iter()
-        .filter_map(|r| region_mean(&diff, w, h, r).map(|s| RegionScore { item: r.item.clone(), score: round2(s) }))
+        .zip(regions)
+        .filter_map(|(pixels, r)| {
+            pair.region(pixels).map(|(score, shift)| RegionScore { item: r.item.clone(), score: round2(score), shift })
+        })
         .collect();
     scored.sort_by(|x, y| y.score.total_cmp(&x.score).then_with(|| x.item.cmp(&y.item)));
     scored.truncate(REPORTED_REGIONS);
-    let mean = diff.iter().sum::<f64>() / diff.len() as f64;
     Some(SlideScore { score: round2(mean), regions: scored })
 }
 
-/// L'écart moyen sur le rectangle `r` borné à l'image ; `None` s'il n'en couvre aucun pixel.
-fn region_mean(diff: &[f64], w: usize, h: usize, r: &Region) -> Option<f64> {
+/// Les pixels propres de chaque région : ceux de son cadre (borné à l'image)
+/// qu'aucune région suivante ne recouvre.
+fn ownership(regions: &[Region], w: usize, h: usize) -> Vec<Vec<usize>> {
+    let mut owner = vec![usize::MAX; w * h];
+    for (i, r) in regions.iter().enumerate() {
+        let (x0, y0, x1, y1) = bounds(r, w, h);
+        for y in y0..y1 {
+            owner[y * w + x0..y * w + x1].fill(i);
+        }
+    }
+    let mut pixels = vec![Vec::new(); regions.len()];
+    for (p, &i) in owner.iter().enumerate() {
+        if let Some(v) = pixels.get_mut(i) {
+            v.push(p);
+        }
+    }
+    pixels
+}
+
+/// Le rectangle `r` en pixels entiers, borné à l'image : `x0, y0, x1, y1`.
+fn bounds(r: &Region, w: usize, h: usize) -> (usize, usize, usize, usize) {
     let clamp = |v: f64, max: usize| (v.max(0.0) as usize).min(max);
-    let (x0, y0) = (clamp(r.x.floor(), w), clamp(r.y.floor(), h));
-    let (x1, y1) = (clamp((r.x + r.w).ceil(), w), clamp((r.y + r.h).ceil(), h));
-    if x1 <= x0 || y1 <= y0 {
-        return None;
+    (clamp(r.x.floor(), w), clamp(r.y.floor(), h), clamp((r.x + r.w).ceil(), w), clamp((r.y + r.h).ceil(), h))
+}
+
+/// La référence et le rendu Oku en OKLab adouci, de même taille.
+struct Pair {
+    a: Vec<Oklab>,
+    b: Vec<Oklab>,
+    w: usize,
+    h: usize,
+}
+
+impl Pair {
+    /// L'écart au pixel `p` de la référence, le rendu Oku décalé de `dx, dy` (bords prolongés).
+    fn at(&self, p: usize, dx: i64, dy: i64) -> f64 {
+        let (x, y) = ((p % self.w) as i64, (p / self.w) as i64);
+        let sx = (x + dx).clamp(0, self.w as i64 - 1) as usize;
+        let sy = (y + dy).clamp(0, self.h as i64 - 1) as usize;
+        delta_e(self.a[p], self.b[sy * self.w + sx])
+    }
+
+    /// Le score d'une région de pixels propres `pixels` et son recalage ;
+    /// `None` sous [`MIN_PIXELS`].
+    fn region(&self, pixels: &[usize]) -> Option<(f64, [i64; 2])> {
+        if pixels.len() < MIN_PIXELS {
+            return None;
+        }
+        let shifts = (-MAX_SHIFT..=MAX_SHIFT).flat_map(|dy| (-MAX_SHIFT..=MAX_SHIFT).map(move |dx| (dx, dy)));
+        // Le recalage se cherche sur un pixel sur ALIGN_STEP : le décalage d'un
+        // dessin se voit sur tous ses pixels, et le coût de la recherche (49
+        // décalages) reste celui d'un seul passage.
+        let total = |(dx, dy): (i64, i64)| pixels.iter().step_by(ALIGN_STEP).map(|&p| self.at(p, dx, dy)).sum::<f64>();
+        // À écart égal, le plus petit décalage (|dx| + |dy|), puis l'ordre de parcours.
+        let (dx, dy) = shifts
+            .map(|s| (total(s), s))
+            .min_by(|x, y| {
+                x.0.total_cmp(&y.0).then_with(|| (x.1.0.abs() + x.1.1.abs()).cmp(&(y.1.0.abs() + y.1.1.abs())))
+            })
+            .map_or((0, 0), |(_, s)| s);
+        // Tuiles de TILE pixels au pas de TILE / 2 : un écart de la taille d'une
+        // tuile tombe entier dans l'une d'elles, où qu'il soit.
+        let half = TILE / 2;
+        let mut tiles: BTreeMap<(usize, usize), (f64, usize)> = BTreeMap::new();
+        for &p in pixels {
+            let (x, y) = (p % self.w / half, p / self.w / half);
+            let d = self.at(p, dx, dy);
+            for tx in x.saturating_sub(1)..=x {
+                for ty in y.saturating_sub(1)..=y {
+                    let t = tiles.entry((tx, ty)).or_default();
+                    t.0 += d;
+                    t.1 += 1;
+                }
+            }
+        }
+        // Une tuile au bord de la région, presque vide, ne décide pas seule ;
+        // une région plus étroite qu'une tuile garde son écart moyen.
+        let full = |n: usize| n * 4 >= TILE * TILE;
+        let mean = || pixels.iter().map(|&p| self.at(p, dx, dy)).sum::<f64>() / pixels.len() as f64;
+        let peak =
+            tiles.values().filter(|(_, n)| full(*n)).map(|(s, n)| s / *n as f64).reduce(f64::max).unwrap_or_else(mean);
+        Some((peak, [dx, dy]))
     }
-    let sum: f64 = (y0..y1).map(|y| diff[y * w + x0..y * w + x1].iter().sum::<f64>()).sum();
-    Some(sum / ((x1 - x0) * (y1 - y0)) as f64)
 }
 
 /// L'image en OKLab, adoucie par un flou gaussien séparable de [`BLUR_SIGMA`].
-fn blurred(image: Image<'_>) -> Vec<crate::retrofit::color::Oklab> {
+fn blurred(image: Image<'_>) -> Vec<Oklab> {
     let (w, h) = (image.width as usize, image.height as usize);
+    // Une page n'a que peu de couleurs distinctes : chacune n'est convertie qu'une fois.
+    let mut known: HashMap<[u8; 3], [f64; 3]> = HashMap::new();
     let lab: Vec<[f64; 3]> = image
         .rgba
         .as_chunks::<4>()
         .0
         .iter()
         .map(|&[r, g, b, _]| {
-            let c = oklab(Rgba::new(r, g, b));
-            [c.l, c.a, c.b]
+            *known.entry([r, g, b]).or_insert_with(|| {
+                let c = oklab(Rgba::new(r, g, b));
+                [c.l, c.a, c.b]
+            })
         })
         .collect();
     let kernel = gaussian(BLUR_SIGMA);
     let rows = convolve(&lab, w, h, &kernel, true);
-    convolve(&rows, w, h, &kernel, false)
-        .into_iter()
-        .map(|[l, a, b]| crate::retrofit::color::Oklab { l, a, b })
-        .collect()
+    convolve(&rows, w, h, &kernel, false).into_iter().map(|[l, a, b]| Oklab { l, a, b }).collect()
 }
 
 /// Noyau gaussien normalisé de rayon `⌈3σ⌉`.
```

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import`
Expected: all pass.
Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-import --all-targets -- -D warnings`
Expected: no warning.
Run: `cargo fmt -p oku-slides-import -- --check`
Expected: no output.

- [ ] **Step 5: Commit**
```bash
git add crates/oku-slides-import/src/metrics.rs crates/oku-slides-import/src/metrics_tests.rs crates/oku-slides-import/src/report_tests.rs
```
Write the message to a file (for example `/tmp/commit-msg-task-5.txt`):
```
feat(import) : score de région recalé, pixels propres et pire tuile, calibré sur alto (t-692)
```
```bash
git commit -F /tmp/commit-msg-task-5.txt
```

**Extracting from the draft:** the implementer may take the files of this task from the draft commit instead of retyping them: `git checkout e33bc28 -- crates/oku-slides-import/src/metrics.rs crates/oku-slides-import/src/metrics_tests.rs crates/oku-slides-import/src/report_tests.rs` (the draft commit also lists the `document.ftl` files of ja, zh-Hans and zh-Hant: leave them out, Task 4 carries them). None of these three files changed on `main` since the draft base `53454df2` (checked with `git diff --name-only 53454df2 main`); if one has, apply it with `git show e33bc28 -- <file> | git apply --3way` instead.

### Task 6: CLI: `--reference auto|none`, visual gap per slide, overflow losses

`oku-slides import` gains `--reference auto|none` (hifi only, default `auto`). After rendering the deck, the CLI records one `Overflow` loss per text field that leaves its page (both modes). In hifi it also has the witness (office2pdf, in the render process) render the presentation, crops each Oku slide to the source page, scores it against its reference page and writes the per-slide gap, the witness and the threshold to the report, plus a second output line. A missing, failed or page-count-mismatched reference is never an error: the report says why. The threshold comes from the new `[import] score_threshold` setting.

Calibration facts (measured 2026-10-14, office2pdf 0.8.1, fonts `assets/fonts`): the maximum slide mean gap is 1.90 for alto, 2.58 for libreoffice and 2.78 for kitchen. Hence the report threshold `3.0` (default of `score_threshold`) and the test bounds "measurement + 25 %": 2.4 for alto and 3.2 for libreoffice. The tests never launch LibreOffice. Each integration test imports a real fixture and takes about 15 s for alto in a debug build.

**Files:**
- Create: `crates/oku-slides-cli/src/import_overflow.rs` (`note_overflows`: `Overflow` losses from the layout)
- Create: `crates/oku-slides-cli/src/import_overflow_tests.rs` (unit tests of `note_overflows`)
- Create: `crates/oku-slides-cli/src/import_reference.rs` (`Scoring`, `score`: reference render and per-slide gap)
- Create: `crates/oku-slides-cli/tests/import_reference.rs` (8 integration tests on the fixtures)
- Modify: `Cargo.lock` (the CLI crate depends on `oku-slides-reference`)
- Modify: `crates/oku-slides-cli/Cargo.toml` (dependency `oku-slides-reference`)
- Modify: `crates/oku-slides-cli/src/import.rs` (`ReferenceMode`, `ImportArgs::reference`, `finish` renders, writes, notes overflows, scores; second output line)
- Modify: `crates/oku-slides-cli/src/main.rs` (modules, `--reference` argument, `write_rendered` split from `write_document`)
- Modify: `crates/oku-slides-cli/src/pack_render.rs` (`Rendered::png`, `DeckWidgets::worker`)
- Modify: `crates/oku-slides-import/src/hifi/mod.rs`, `crates/oku-slides-import/src/hifi/page.rs` (`hifi::source_rect`, `Page::source_rect`)
- Modify: `crates/oku-slides-platform/src/config.default.toml`, `crates/oku-slides-platform/src/config.rs` (`[import] score_threshold`, `ImportSettings`)
- Modify: the 20 `crates/oku-slides-i18n/src/locales/*/cli.ftl` (four keys)
- `crates/oku-slides-cli/ARCHITECTURE.md`: updated in Task 7 (English).
- Test: `crates/oku-slides-import/src/hifi/page_tests.rs`, `crates/oku-slides-platform/src/config_tests.rs`, `crates/oku-slides-cli/src/import_overflow_tests.rs`, `crates/oku-slides-cli/tests/import_reference.rs`

**Interfaces:**
- Consumes (earlier tasks): `oku_slides_reference::{Office2Pdf, Raster, ReferenceRenderer}` (`Raster::from_png`, `crop`, `width`, `height`, `rgba`; `witness()`); `oku_slides_render_worker::{RenderWorker::reference(&self, pptx: &[u8], fonts: &Fonts) -> Result<RemoteReference, RenderError>, RemoteReference::{page_count, png(index, width), witness}}`; `oku_slides_import::metrics::{score, regions, Image}`; `oku_slides_import::report::{Loss, LossKind::Overflow { element }, ReferenceCause::{Pages { reference, deck }, Timeout, Memory, Failed}, ReferenceReport { witness, threshold, unavailable }, SlideReport::score}`; `ImportReport::{reference, mean_score(), above_threshold()}`, `ImportPlan::report`, `Layout::{fields, slides, tokens, page_of}` and `FieldBox` of `oku-slides-render`; `Localize` for `ReferenceCause`; the `oku-slides-reference` entry of `[workspace.dependencies]`.
- Produces: `oku_slides_import::hifi::source_rect(deck: &SourceDeck) -> oku_slides_core::Frame`; `Settings::import: ImportSettings` with `ImportSettings { score_threshold: f64 }` (env `OKU_SLIDES_IMPORT__SCORE_THRESHOLD`); in the CLI crate (`pub(crate)`): `ReferenceMode { Auto, None }`, `ImportArgs::reference: Option<ReferenceMode>`, `import_overflow::note_overflows(layout: &Layout, report: &mut ImportReport)`, `import_reference::{REFERENCE_WIDTH: f64, Scoring<'a> { pptx, source, fonts, worker, threshold }, score(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport)}`, `Rendered::png(&self, index: usize, pixel_per_pt: f32) -> Result<Vec<u8>, RenderError>`, `DeckWidgets::worker(&self) -> Option<&RenderWorker>`, `write_rendered(platform, doc: &Rendered, bundle, output) -> Result<Written>`; Fluent keys `cli-import-reference-hifi-only`, `cli-import-scored`, `cli-import-unscored`, `cli-arg-reference`.

- [ ] **Step 1: Write the failing tests**

`Page::source_rect` test:

```diff
--- a/crates/oku-slides-import/src/hifi/page_tests.rs
+++ b/crates/oku-slides-import/src/hifi/page_tests.rs
@@ -66,3 +66,9 @@ fn page_cover_is_the_whole_oku_page() {
     let f = page.frame(&page.cover());
     assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((0.0, 0.0, PAGE_W, PAGE_H));
 }
+
+#[test]
+fn source_rect_of_a_4_3_source_is_the_centered_band() {
+    let r = Page::new(Size { w: 720.0, h: 540.0 }).source_rect();
+    assert_that!((r.x, r.y, r.w, r.h)).is_equal_to((240.0, 0.0, 1440.0, 1080.0));
+}
```

Settings tests:

```diff
--- a/crates/oku-slides-platform/src/config_tests.rs
+++ b/crates/oku-slides-platform/src/config_tests.rs
@@ -303,6 +303,17 @@ fn render_memory_and_frame_env_overrides_change_the_settings() {
     assert_that!((s.render.memory_mib, s.render.max_frame_mib)).is_equal_to((1024, 64));
 }
 
+#[test]
+fn settings_default_import_threshold_matches_config_defaults() {
+    assert_that!(Settings::defaults().unwrap().import.score_threshold).is_equal_to(3.0);
+}
+
+#[test]
+fn import_threshold_env_override_changes_the_setting() {
+    let s = Settings::from_sources(None, env(&[("OKU_SLIDES_IMPORT__SCORE_THRESHOLD", "4.5")])).unwrap();
+    assert_that!(s.import.score_threshold).is_equal_to(4.5);
+}
+
 #[test]
 fn render_timeout_env_override_changes_the_setting() {
     let s = Settings::from_sources(None, env(&[("OKU_SLIDES_RENDER__TIMEOUT_MS", "5000")])).unwrap();
```

Overflow unit tests (a new file; it compiles only once Step 3 declares `mod import_overflow_tests` through `import_overflow.rs`):

```diff
--- /dev/null
+++ b/crates/oku-slides-cli/src/import_overflow_tests.rs
@@ -0,0 +1,95 @@
+use oku_slides_import::report::{Mode, Placement, SlideReport};
+use oku_slides_render::{FieldBox, SlidePage};
+use serde_json::json;
+use speculoos::prelude::*;
+
+use super::*;
+
+fn field(owner: &str, page: usize, (x, y, w, h): (f64, f64, f64, f64)) -> FieldBox {
+    FieldBox {
+        owner: owner.into(),
+        field: "body".into(),
+        page,
+        x,
+        y,
+        w,
+        h,
+        ax: None,
+        aw: None,
+        style: None,
+        empty: false,
+        side: None,
+    }
+}
+
+/// Deux slides (pages 1 et 2) sur une page de 1920 × 1080.
+fn layout(fields: Vec<FieldBox>) -> Layout {
+    Layout {
+        slides: vec![SlidePage { slide: "s1".into(), page: 1 }, SlidePage { slide: "s2".into(), page: 2 }],
+        fields,
+        tokens: Some(json!({"page": {"width": 1920, "height": 1080}})),
+        ..Layout::default()
+    }
+}
+
+fn report() -> ImportReport {
+    let slide = |n: u32| SlideReport {
+        source: n,
+        slide: format!("s{n}"),
+        template: "t".into(),
+        placement: Placement::Flow,
+        losses: vec![],
+        fallbacks: vec![],
+        score: None,
+    };
+    ImportReport { slides: vec![slide(1), slide(2)], ..ImportReport::new(Mode::Retrofit) }
+}
+
+fn overflows(r: &ImportReport) -> Vec<(u32, String)> {
+    r.slides
+        .iter()
+        .flat_map(|s| s.losses.iter().map(move |l| (s.source, l)))
+        .filter_map(|(n, l)| match &l.kind {
+            LossKind::Overflow { element } => Some((n, element.clone())),
+            _ => None,
+        })
+        .collect()
+}
+
+#[test]
+fn note_overflows_reports_a_field_below_the_page_on_its_slide() {
+    let mut r = report();
+    note_overflows(&layout(vec![field("s2-1", 2, (100.0, 900.0, 800.0, 300.0))]), &mut r);
+    assert_that!(overflows(&r)).is_equal_to(vec![(2, "s2-1/body".to_owned())]);
+}
+
+#[test]
+fn note_overflows_ignores_a_field_inside_the_page() {
+    let mut r = report();
+    note_overflows(&layout(vec![field("s1-1", 1, (0.0, 0.0, 1920.0, 1080.5))]), &mut r);
+    assert_that!(overflows(&r)).is_empty();
+}
+
+#[test]
+fn note_overflows_ignores_an_empty_field() {
+    let mut r = report();
+    let empty = FieldBox { empty: true, ..field("s1-1", 1, (0.0, 1000.0, 10.0, 200.0)) };
+    note_overflows(&layout(vec![empty]), &mut r);
+    assert_that!(overflows(&r)).is_empty();
+}
+
+#[test]
+fn note_overflows_reports_a_field_once() {
+    let mut r = report();
+    let f = field("s1-1", 1, (1800.0, 0.0, 300.0, 10.0));
+    note_overflows(&layout(vec![f.clone(), f]), &mut r);
+    assert_that!(overflows(&r).len()).is_equal_to(1);
+}
+
+#[test]
+fn note_overflows_reads_the_page_size_from_the_tokens() {
+    let mut r = report();
+    let small = Layout { tokens: Some(json!({"page": {"width": 960, "height": 540}})), ..layout(vec![]) };
+    note_overflows(&Layout { fields: vec![field("s1-1", 1, (0.0, 500.0, 100.0, 100.0))], ..small }, &mut r);
+    assert_that!(overflows(&r).len()).is_equal_to(1);
+}
```

Integration tests:

```diff
--- /dev/null
+++ b/crates/oku-slides-cli/tests/import_reference.rs
@@ -0,0 +1,122 @@
+//! `oku-slides import --mode hifi`, comparé au rendu de référence du témoin
+//! (office2pdf, dans le processus de rendu) : un écart par slide au rapport,
+//! sous un seuil par fixture, et des régions qui désignent les écarts connus
+//! d'alto (t-863 : la coupure de la ligne de l'encadré p. 7, le retrait des
+//! puces p. 8). Polices de l'app (`--fonts assets/fonts`) : celles que le
+//! témoin reçoit aussi. Aucun test ne lance LibreOffice.
+
+mod common;
+
+use std::path::{Path, PathBuf};
+use std::process::Output;
+
+use common::{assert_success, oku_slides, workdir};
+use rstest::rstest;
+use serde_json::Value;
+use speculoos::prelude::*;
+
+fn root() -> PathBuf {
+    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
+}
+
+/// Importe la fixture `name` en hi-fi, avec `extra` ; la sortie et le rapport.
+fn import(dir: &Path, name: &str, extra: &[&str]) -> (Output, Value) {
+    let report = dir.join("r.json");
+    let out = oku_slides(dir, "en")
+        .arg("--fonts")
+        .arg(root().join("assets/fonts"))
+        .arg("import")
+        .arg(root().join("fixtures/import").join(name))
+        .args(["--mode", "hifi", "--no-system-fonts", "-o"])
+        .arg(dir.join("out.oku-slides.pdf"))
+        .arg("--report")
+        .arg(&report)
+        .args(extra)
+        .output()
+        .unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"));
+    assert_success("import", &out, &[]);
+    let report = serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|e| panic!("{e}")))
+        .unwrap_or_else(|e| panic!("rapport : {e}"));
+    (out, report)
+}
+
+/// Les ids des régions désignées de la slide `n` (1..), les pires d'abord.
+fn worst(report: &Value, n: usize) -> Vec<String> {
+    let regions = report["slides"][n - 1]["score"]["regions"].as_array().cloned().unwrap_or_default();
+    regions.iter().filter_map(|r| r["item"].as_str().map(str::to_owned)).collect()
+}
+
+/// Le plus grand écart moyen des slides du rapport ; échoue si une slide n'en a pas.
+fn max_score(report: &Value) -> f64 {
+    let slides = report["slides"].as_array().unwrap_or_else(|| panic!("rapport sans slides : {report}"));
+    slides
+        .iter()
+        .map(|s| s["score"]["score"].as_f64().unwrap_or_else(|| panic!("slide sans écart : {s}")))
+        .fold(0.0, f64::max)
+}
+
+#[rstest]
+// Mesures du 2026-10-14 (office2pdf 0.8.1), plus 25 % : alto 1,90, libreoffice 2,58.
+#[case("alto-wealth.pptx", 2.4)]
+#[case("libreoffice.pptx", 3.2)]
+fn import_hifi_scores_every_slide_under_its_threshold(#[case] name: &str, #[case] threshold: f64) {
+    let dir = workdir("import-reference");
+    let (_, report) = import(dir.as_ref(), name, &[]);
+    assert_that!(report["reference"]["witness"].as_str()).is_equal_to(Some("office2pdf 0.8.1"));
+    assert_that!(max_score(&report)).is_less_than_or_equal_to(threshold);
+}
+
+#[test]
+fn import_hifi_of_alto_designates_the_cut_line_of_the_callout_on_page_7() {
+    let dir = workdir("import-reference-p7");
+    let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
+    // Le titre le précède : le témoin dessine les titres d'alto plus larges (limite connue).
+    assert_that!(worst(&report, 7)[..2].to_vec()).contains("s7/21".to_owned());
+}
+
+#[test]
+fn import_hifi_of_alto_designates_the_bullet_indents_on_page_8() {
+    let dir = workdir("import-reference-p8");
+    let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
+    let lists = ["s8/8", "s8/12", "s8/16", "s8/21"];
+    let found = worst(&report, 8).iter().filter(|i| lists.contains(&i.as_str())).count();
+    assert_that!(found).is_greater_than_or_equal_to(2);
+}
+
+#[test]
+fn import_hifi_of_libreoffice_designates_its_table_on_page_1() {
+    let dir = workdir("import-reference-table");
+    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &[]);
+    assert_that!(worst(&report, 1).first().map(String::as_str)).is_equal_to(Some("s1/8"));
+}
+
+#[test]
+fn import_hifi_says_the_visual_gap_on_a_second_line() {
+    let dir = workdir("import-reference-line");
+    let (out, _) = import(dir.as_ref(), "libreoffice.pptx", &[]);
+    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
+    assert_that!(stdout.lines().nth(1).unwrap_or_default()).starts_with("Visual gap to office2pdf 0.8.1: ");
+}
+
+#[test]
+fn import_hifi_with_reference_none_writes_no_score() {
+    let dir = workdir("import-reference-none");
+    let (out, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "none"]);
+    assert_that!(report.get("reference")).is_none();
+    assert_that!(report["slides"][0].get("score")).is_none();
+    assert_that!(String::from_utf8_lossy(&out.stdout).lines().count()).is_equal_to(1);
+}
+
+#[test]
+fn import_retrofit_with_reference_is_refused_in_french() {
+    let dir = workdir("import-reference-retrofit");
+    let out = oku_slides(dir.as_ref(), "fr")
+        .arg("import")
+        .arg(root().join("fixtures/import/libreoffice.pptx"))
+        .args(["--pack", "essentiel", "--reference", "auto", "-o"])
+        .arg(dir.as_ref().join("out.oku-slides.pdf"))
+        .output()
+        .unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"));
+    assert_that!(out.status.success()).is_false();
+    assert_that!(String::from_utf8_lossy(&out.stderr).into_owned()).contains("--reference appartient au mode hifi");
+}
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import -p oku-slides-platform -E 'test(source_rect_of_a_4_3_source_is_the_centered_band) | test(import_threshold) | test(settings_default_import_threshold)'`
Expected: compile errors naming the missing `source_rect` method and the missing `import` field of `Settings`.

Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-cli --test import_reference`
Expected: the tests fail (clap rejects the unknown `--reference` argument, and no report carries a `reference` or a `score`).

- [ ] **Step 3: Implement**

Workspace dependency: the CLI crate now uses the witness crate. `Cargo.lock`: let cargo update the lock file (`cargo build`), then check that `git diff Cargo.lock` only adds the package name `oku-slides-reference` to the dependency list of `oku-slides-cli`. No new package.

`[import] score_threshold` setting (`3.0` is the calibrated threshold, see above):

```diff
--- a/crates/oku-slides-platform/src/config.default.toml
+++ b/crates/oku-slides-platform/src/config.default.toml
@@ -52,6 +52,12 @@ max_widget_kib = 256
 # Nombre maximal de fichiers de widgets d'une version de pack, `_commun` compris.
 max_widgets = 64
 
+[import]
+# Import hi-fi : écart visuel (ΔE OKLab ×100, moyen sur la slide) au rendu de
+# référence du témoin au-delà duquel le rapport signale une slide. Les fixtures
+# d'import, fidèles, restent sous 2,8 (le témoin dessine sommairement les graphiques).
+score_threshold = 3.0
+
 [serve]
 # Serveur de développement (`oku-slides serve`), sans authentification.
 # Il ne répond qu'aux requêtes dont l'en-tête Host nomme la boucle locale
```
```diff
--- a/crates/oku-slides-platform/src/config.rs
+++ b/crates/oku-slides-platform/src/config.rs
@@ -34,6 +34,7 @@ pub struct Settings {
     pub resume: ResumeSettings,
     pub sources: SourcesSettings,
     pub render: RenderSettings,
+    pub import: ImportSettings,
     pub serve: ServeSettings,
     pub i18n: I18nSettings,
     pub appearance: AppearanceSettings,
@@ -107,6 +108,14 @@ pub struct RenderSettings {
     pub max_widgets: usize,
 }
 
+/// Import d'une présentation (`oku-slides import`).
+#[derive(Debug, Clone, PartialEq, Deserialize)]
+pub struct ImportSettings {
+    /// Hi-fi : écart visuel (ΔE OKLab ×100, moyen sur la slide) au rendu de
+    /// référence au-delà duquel le rapport signale une slide.
+    pub score_threshold: f64,
+}
+
 impl SourcesSettings {
     /// [`Self::compact_after_days`] en secondes.
     pub fn compact_after_secs(&self) -> i64 {
```

`hifi::source_rect`: the source page on the Oku page, where the Oku render is compared with the reference render:

```diff
--- a/crates/oku-slides-import/src/hifi/mod.rs
+++ b/crates/oku-slides-import/src/hifi/mod.rs
@@ -165,6 +165,12 @@ pub fn project(deck: &SourceDeck, target: &HifiTarget<'_>) -> ImportPlan {
     plan
 }
 
+/// La page source de `deck` sur la page Oku (pt page) : le rendu Oku d'une
+/// slide, rogné à ce cadre, se compare au rendu de référence de la source.
+pub fn source_rect(deck: &SourceDeck) -> oku_slides_core::Frame {
+    Page::new(deck.size).source_rect()
+}
+
 /// Les familles que [`project`] cherche au catalogue des polices pour
 /// `deck` (voir [`sought`]) : l'appelant n'y ajoute que celles du système
 /// qui en sont.
```
```diff
--- a/crates/oku-slides-import/src/hifi/page.rs
+++ b/crates/oku-slides-import/src/hifi/page.rs
@@ -64,6 +64,12 @@ impl Page {
         }
     }
 
+    /// La page source sur la page Oku, en pt page : là où le rendu Oku se
+    /// compare au rendu de référence.
+    pub(crate) fn source_rect(&self) -> oku_slides_core::Frame {
+        self.frame(&Frame::new(0.0, 0.0, self.source.w, self.source.h))
+    }
+
     /// La boîte source que la page Oku entière recouvre (bandes de centrage
     /// comprises) : celle du fond propre d'une slide.
     pub(crate) fn cover(&self) -> Frame {
```

CLI crate dependency:

```diff
--- a/crates/oku-slides-cli/Cargo.toml
+++ b/crates/oku-slides-cli/Cargo.toml
@@ -27,6 +27,7 @@ tracing.workspace = true
 oku-slides-platform = { workspace = true, features = ["listen"] }
 oku-slides-i18n.workspace = true
 oku-slides-import.workspace = true
+oku-slides-reference.workspace = true
 fontdb.workspace = true
 sha2.workspace = true
 clap.workspace = true
```

`Rendered::png` and `DeckWidgets::worker`:

```diff
--- a/crates/oku-slides-cli/src/pack_render.rs
+++ b/crates/oku-slides-cli/src/pack_render.rs
@@ -93,6 +93,14 @@ impl Rendered {
             Self::Remote { compiled, .. } => compiled.pdf(),
         }
     }
+
+    /// PNG de la page `index` (0 = première), à `pixel_per_pt` pixels par point.
+    pub(crate) fn png(&self, index: usize, pixel_per_pt: f32) -> Result<Vec<u8>, RenderError> {
+        match self {
+            Self::Local(c) => c.png(index, pixel_per_pt),
+            Self::Remote { compiled, .. } => compiled.png(index, pixel_per_pt),
+        }
+    }
 }
 
 /// Les widgets d'un deck : le registre sondé (la base surchargée par les
@@ -107,6 +115,11 @@ pub(crate) struct DeckWidgets {
 }
 
 impl DeckWidgets {
+    /// Le processus de rendu du pack, s'il en a un.
+    pub(crate) fn worker(&self) -> Option<&RenderWorker> {
+        self.remote.as_ref().map(|(w, _)| w)
+    }
+
     /// Sonde le pack que `deck` référence, pris dans `packs`.
     ///
     /// Sans référence, ou sans widgets : la base. Version absente de `packs`
```

Overflow losses (new file; the tests of Step 1 are included by its `#[path]` module):

```diff
--- /dev/null
+++ b/crates/oku-slides-cli/src/import_overflow.rs
@@ -0,0 +1,38 @@
+//! Débordements relevés sur le rendu d'un import (les deux modes) : un champ
+//! de texte qui sort de sa page devient une perte `Overflow` de sa slide
+//! (t-816 : un contenu plus haut que sa place). Seuls les champs comptent :
+//! une forme ou une image qui déborde volontairement (fond perdu) n'en est pas une.
+
+use oku_slides_import::ImportReport;
+use oku_slides_import::report::{Loss, LossKind};
+use oku_slides_import::retrofit::PageTokens;
+use oku_slides_render::Layout;
+
+/// Marge tolérée au bord de la page, en pt (arrondis du rendu).
+const TOLERANCE: f64 = 1.0;
+
+/// Ajoute à `report` une perte `Overflow` par champ non vide de `layout` qui
+/// sort de sa page, sur la slide qui le porte (la première fois seulement).
+pub(crate) fn note_overflows(layout: &Layout, report: &mut ImportReport) {
+    let page = layout.tokens.as_ref().and_then(|t| t.as_object()).map(PageTokens::from_tokens).unwrap_or_default();
+    for field in layout.fields.iter().filter(|f| !f.empty) {
+        let inside = field.x >= -TOLERANCE
+            && field.y >= -TOLERANCE
+            && field.x + field.w <= page.width + TOLERANCE
+            && field.y + field.h <= page.height + TOLERANCE;
+        if inside {
+            continue;
+        }
+        let Some(slide) = layout.slides.iter().find(|s| s.page == field.page) else { continue };
+        let Some(r) = report.slides.iter_mut().find(|r| r.slide == slide.slide) else { continue };
+        let element = format!("{}/{}", field.owner, field.field);
+        let loss = Loss::slide(LossKind::Overflow { element });
+        if !r.losses.contains(&loss) {
+            r.losses.push(loss);
+        }
+    }
+}
+
+#[cfg(test)]
+#[path = "import_overflow_tests.rs"]
+mod import_overflow_tests;
```

Scoring against the reference (new file):

```diff
--- /dev/null
+++ b/crates/oku-slides-cli/src/import_reference.rs
@@ -0,0 +1,97 @@
+//! L'écart visuel d'un import hi-fi (`--reference auto`, le défaut en hi-fi) :
+//! le témoin rend la présentation dans le processus de rendu
+//! ([`RenderWorker::reference`]), puis chaque slide Oku, rognée à la page
+//! source ([`hifi::source_rect`]), est comparée à sa page de référence à
+//! [`REFERENCE_WIDTH`] pixels de large ([`metrics::score`], régions des items
+//! source). Une référence absente, en échec ou d'un autre nombre de pages
+//! n'est jamais une erreur : le rapport dit pourquoi (`ReferenceCause`).
+
+use oku_slides_import::metrics::{self, Image};
+use oku_slides_import::report::{ReferenceCause, ReferenceReport};
+use oku_slides_import::source::SourceDeck;
+use oku_slides_import::{ImportReport, hifi};
+use oku_slides_reference::{Office2Pdf, Raster, ReferenceRenderer};
+use oku_slides_render::{Fonts, RenderError, RenderErrorKind};
+use oku_slides_render_worker::{RemoteReference, RenderWorker};
+
+use crate::pack_render::Rendered;
+
+/// Largeur, en pixels, de la page source comparée (spec : 960 px).
+pub(crate) const REFERENCE_WIDTH: f64 = 960.0;
+
+/// Ce qu'il faut pour mesurer l'écart d'un import hi-fi.
+pub(crate) struct Scoring<'a> {
+    /// Les octets du `.pptx`.
+    pub(crate) pptx: &'a [u8],
+    /// Sa source lue.
+    pub(crate) source: &'a SourceDeck,
+    /// Les polices du rendu Oku (celles de l'app et celles du deck), données au témoin.
+    pub(crate) fonts: &'a Fonts,
+    /// Le processus de rendu.
+    pub(crate) worker: &'a RenderWorker,
+    /// Le seuil du rapport (`[import] score_threshold`).
+    pub(crate) threshold: f64,
+}
+
+/// Pose dans `report` la référence et l'écart de chaque slide de `doc`.
+pub(crate) fn score(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport) {
+    let result = s.worker.reference(s.pptx, s.fonts).map_err(|e| cause(&e)).and_then(|reference| {
+        let (pages, deck) = (reference.page_count(), doc.page_count());
+        if pages != deck || pages != report.slides.len() {
+            let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
+            return Err(ReferenceCause::Pages { reference: count(pages), deck: count(deck) });
+        }
+        scores(s, &reference, doc, report).map(|()| reference.witness().to_owned())
+    });
+    let (witness, unavailable) = match result {
+        Ok(witness) => (witness, None),
+        Err(cause) => {
+            for slide in &mut report.slides {
+                slide.score = None;
+            }
+            (Office2Pdf.witness().to_owned(), Some(cause))
+        }
+    };
+    report.reference = Some(ReferenceReport { witness, threshold: s.threshold, unavailable });
+}
+
+/// L'écart de chaque slide : sa page Oku rognée à la page source contre sa page de référence.
+fn scores(
+    s: &Scoring<'_>,
+    reference: &RemoteReference,
+    doc: &Rendered,
+    report: &mut ImportReport,
+) -> Result<(), ReferenceCause> {
+    let rect = hifi::source_rect(s.source);
+    // Pixels par pt de la page Oku, pour que la page source y fasse REFERENCE_WIDTH pixels.
+    let ppp = REFERENCE_WIDTH / rect.w;
+    let px = |v: f64| (v * ppp).round().max(0.0) as u32;
+    let width = px(rect.w);
+    let scale = f64::from(width) / s.source.size.w;
+    for (index, (slide, src)) in report.slides.iter_mut().zip(&s.source.slides).enumerate() {
+        let page = doc.layout().page_of(&slide.slide).map_or(index, |p| p.saturating_sub(1));
+        let oku =
+            Raster::from_png(&doc.png(page, ppp as f32).map_err(|e| cause(&e))?).map_err(|_| ReferenceCause::Failed)?;
+        let refr = Raster::from_png(&reference.png(index, width).map_err(|e| cause(&e))?)
+            .map_err(|_| ReferenceCause::Failed)?;
+        let height = refr.height().min(px(rect.h));
+        let oku = oku.crop(px(rect.x), px(rect.y), width, height);
+        let refr = refr.crop(0, 0, width, height);
+        slide.score = metrics::score(image(&refr), image(&oku), &metrics::regions(src, scale));
+    }
+    Ok(())
+}
+
+/// La vue de `r` que lit [`metrics::score`].
+fn image(r: &Raster) -> Image<'_> {
+    Image { width: r.width(), height: r.height(), rgba: r.rgba() }
+}
+
+/// Pourquoi un échec du processus de rendu prive l'import de sa référence.
+fn cause(e: &RenderError) -> ReferenceCause {
+    match e.kind() {
+        RenderErrorKind::Timeout => ReferenceCause::Timeout,
+        RenderErrorKind::OutOfMemory => ReferenceCause::Memory,
+        _ => ReferenceCause::Failed,
+    }
+}
```

`main.rs`: modules, `--reference` argument, `write_rendered` split from `write_document` so `finish` can keep the `Rendered` for scoring:

```diff
--- a/crates/oku-slides-cli/src/main.rs
+++ b/crates/oku-slides-cli/src/main.rs
@@ -8,6 +8,8 @@ mod bench;
 mod guard;
 mod import;
 mod import_hifi;
+mod import_overflow;
+mod import_reference;
 mod library;
 mod out;
 mod pack_render;
@@ -30,7 +32,7 @@ use oku_slides_render::Fonts;
 use oku_slides_session::{PackLibrary, Storage, adopt_default_pack, adopt_default_theme, read_document, theme_tokens};
 
 use crate::bench::ms;
-use crate::import::{ImportArgs, ImportMode};
+use crate::import::{ImportArgs, ImportMode, ReferenceMode};
 use crate::library::{beside_deck, embed_theme, pack_library, theme_library};
 use crate::out::Out;
 use crate::pack_render::DeckWidgets;
@@ -91,6 +93,8 @@ enum Cmd {
         report: Option<PathBuf>,
         #[arg(long)]
         no_system_fonts: bool,
+        #[arg(long, value_enum)]
+        reference: Option<ReferenceMode>,
     },
     Bench {
         deck: PathBuf,
@@ -147,6 +151,7 @@ fn command(l: &Localizer) -> clap::Command {
                 .mut_arg("output", |a| a.help(help("cli-arg-output")))
                 .mut_arg("report", |a| a.help(help("cli-arg-report")))
                 .mut_arg("no_system_fonts", |a| a.help(help("cli-arg-no-system-fonts")))
+                .mut_arg("reference", |a| a.help(help("cli-arg-reference")))
         })
         .mut_subcommand("bench", |c| {
             c.about(help("cli-cmd-bench"))
@@ -274,6 +279,17 @@ fn write_document(
     output: &Path,
 ) -> Result<Written> {
     let doc = pack_render::render(platform, l, fonts, widgets, bundle, bundle.deck())?;
+    write_rendered(platform, &doc, bundle, output)
+}
+
+/// Joint les sources de `bundle` au PDF de `doc`, son rendu, et écrit le tout
+/// dans `output` (écriture atomique).
+fn write_rendered(
+    platform: &Platform,
+    doc: &pack_render::Rendered,
+    bundle: &SourceBundle,
+    output: &Path,
+) -> Result<Written> {
     let pdf = doc.pdf()?;
     let zip = bundle.to_zip()?;
     let written = oku_slides_pdf::embed_sources(&pdf, &zip)?;
@@ -440,8 +456,9 @@ fn run(cli: Cli, platform: &Platform, out: &mut Out<'_>) -> Result<()> {
             storage.write_atomic(&output, oku_slides_pdf::strip_sources(&pdf)?)?;
             out.line("cli-stripped", &[("path", output.display().to_string().into())])?;
         }
-        Cmd::Import { input, mode, pack, theme, output, report, no_system_fonts } => {
-            let args = ImportArgs { input, mode, pack, theme, output, report, system_fonts: !no_system_fonts };
+        Cmd::Import { input, mode, pack, theme, output, report, no_system_fonts, reference } => {
+            let args =
+                ImportArgs { input, mode, pack, theme, output, report, system_fonts: !no_system_fonts, reference };
             import::import(platform, out, &app_fonts, args)?;
         }
         Cmd::Bench { deck, iterations } => {
```

`import.rs`: `ReferenceMode`, the retrofit refusal, and `finish` (render, write, overflows, scoring, second output line):

```diff
--- a/crates/oku-slides-cli/src/import.rs
+++ b/crates/oku-slides-cli/src/import.rs
@@ -11,7 +11,11 @@
 //!
 //! Dans les deux modes, la lecture est bornée, puis les ops du plan
 //! s'appliquent en une transaction et le deck est rendu et enregistré en
-//! `.oku-slides.pdf`, comme `save`, avec son rapport s'il est demandé.
+//! `.oku-slides.pdf`, comme `save`, avec son rapport s'il est demandé. Le
+//! rendu relève les champs qui sortent de leur page (perte `Overflow`,
+//! [`crate::import_overflow`]) ; en hi-fi, sauf `--reference none`, il est
+//! comparé au rendu de référence du témoin ([`crate::import_reference`]) :
+//! un écart par slide au rapport, une ligne de plus en sortie.
 
 use std::collections::BTreeMap;
 use std::path::PathBuf;
@@ -22,18 +26,22 @@ use serde_json::{Map, Value};
 use oku_slides_core::tokens::effective_tokens;
 use oku_slides_core::{Deck, PackManifest, PackRef, SourceBundle, apply_all};
 use oku_slides_i18n::{Localize, Localizer};
+use oku_slides_import::report::ReferenceReport;
 use oku_slides_import::retrofit::{self, ColorTokens, PageTokens};
 use oku_slides_import::source::SourceDeck;
-use oku_slides_import::{ImportPlan, ReadLimits, RetrofitTarget, read_pptx};
+use oku_slides_import::{ImportPlan, ImportReport, ReadLimits, RetrofitTarget, read_pptx};
 use oku_slides_platform::Platform;
 use oku_slides_platform::storage::StorageErrorKind;
 use oku_slides_render::Fonts;
+use oku_slides_render_worker::RenderWorker;
 use oku_slides_session::theme_schema::default_tokens;
 use oku_slides_session::{ThemeLibrary, adopt_default_theme, theme_tokens};
 
+use crate::import_overflow::note_overflows;
+use crate::import_reference::{self, Scoring};
 use crate::library::{embed_theme, pack_library, resolve_pack, theme_library};
 use crate::out::Out;
-use crate::pack_render::DeckWidgets;
+use crate::pack_render::{self, DeckWidgets};
 
 /// Mode d'import.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
@@ -44,6 +52,15 @@ pub(crate) enum ImportMode {
     Hifi,
 }
 
+/// Rendu de référence d'un import hi-fi.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
+pub(crate) enum ReferenceMode {
+    /// Le témoin rend la présentation ; l'écart de chaque slide va au rapport.
+    Auto,
+    /// Pas de rendu de référence.
+    None,
+}
+
 /// Les arguments de `import`.
 #[derive(Debug)]
 pub(crate) struct ImportArgs {
@@ -61,6 +78,8 @@ pub(crate) struct ImportArgs {
     pub(crate) report: Option<PathBuf>,
     /// Hi-fi : chercher les polices de la présentation parmi celles du système.
     pub(crate) system_fonts: bool,
+    /// Hi-fi : rendu de référence (`None` : le défaut, `auto`) ; refusé en retrofit.
+    pub(crate) reference: Option<ReferenceMode>,
 }
 
 /// La présentation lue : ses octets (l'identité du hi-fi en dérive) et sa source.
@@ -92,6 +111,9 @@ pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args
             return Err(eyre!(l.tr("cli-import-hifi-no-target")));
         }
         ImportMode::Hifi => None,
+        ImportMode::Retrofit if args.reference.is_some() => {
+            return Err(eyre!(l.tr("cli-import-reference-hifi-only")));
+        }
         ImportMode::Retrofit => Some(args.pack.as_deref().ok_or_else(|| eyre!(l.tr("cli-import-pack-required")))?),
     };
     let input = read_input(platform, l, &args)?;
@@ -99,7 +121,7 @@ pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args
         Some(pack) => prepare_retrofit(platform, l, fonts, &args, pack, &input.source)?,
         None => crate::import_hifi::prepare(platform, l, fonts, &args, &input)?,
     };
-    finish(platform, out, fonts, &args, prepared)
+    finish(platform, out, fonts, &args, &input, prepared)
 }
 
 /// Lit `args.input`, borné par [`ReadLimits::default`].
@@ -157,8 +179,17 @@ fn prepare_retrofit(
 }
 
 /// Applique le plan de `prepared` en une transaction, rend et écrit le deck
-/// (avec ses polices embarquées), puis le rapport ; une ligne de résultat.
-fn finish(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: &ImportArgs, prepared: Prepared) -> Result<()> {
+/// (avec ses polices embarquées), relève débordements et, en hi-fi, l'écart à
+/// la référence, puis écrit le rapport ; une ligne de résultat, plus celle de
+/// l'écart en hi-fi.
+fn finish(
+    platform: &Platform,
+    out: &mut Out<'_>,
+    fonts: &Fonts,
+    args: &ImportArgs,
+    input: &Input,
+    prepared: Prepared,
+) -> Result<()> {
     let l = out.localizer();
     let Prepared { mut bundle, widgets, mut plan } = prepared;
     for asset in plan.assets.values() {
@@ -170,8 +201,24 @@ fn finish(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: &ImportAr
     crate::report_issues(l, bundle.deck(), &widgets.registry, &bundle.assets);
     let target = args.output.display().to_string();
     let fonts = fonts.with(bundle.fonts.values().cloned());
-    let written = crate::write_document(platform, l, fonts, &widgets, &bundle, &args.output)
-        .wrap_err_with(|| l.tr_with("cli-import-write-failed", &[("path", target.as_str().into())]))?;
+    let failed = || l.tr_with("cli-import-write-failed", &[("path", target.as_str().into())]);
+    let doc =
+        pack_render::render(platform, l, fonts.clone(), &widgets, &bundle, bundle.deck()).wrap_err_with(failed)?;
+    let written = crate::write_rendered(platform, &doc, &bundle, &args.output).wrap_err_with(failed)?;
+    note_overflows(doc.layout(), &mut plan.report);
+    if args.mode == ImportMode::Hifi && args.reference != Some(ReferenceMode::None) {
+        let own;
+        let worker = match widgets.worker() {
+            Some(w) => w,
+            None => {
+                own = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
+                &own
+            }
+        };
+        let threshold = platform.settings.import.score_threshold;
+        let scoring = Scoring { pptx: &input.bytes, source: &input.source, fonts: &fonts, worker, threshold };
+        import_reference::score(&scoring, &doc, &mut plan.report);
+    }
     if let Some(path) = &args.report {
         platform.storage.write(path, serde_json::to_string_pretty(&plan.report_json(l))?)?;
     }
@@ -185,5 +232,26 @@ fn finish(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: &ImportAr
             ("requests", plan.widget_requests.len().into()),
         ],
     )?;
+    if let Some(reference) = &plan.report.reference {
+        reference_line(out, reference, &plan.report)?;
+    }
+    Ok(())
+}
+
+/// La ligne de l'écart visuel : moyenne et slides au-delà du seuil, ou pourquoi la référence manque.
+fn reference_line(out: &mut Out<'_>, reference: &ReferenceReport, report: &ImportReport) -> Result<()> {
+    let l = out.localizer();
+    match (&reference.unavailable, report.mean_score()) {
+        (Some(cause), _) => out.line("cli-import-unscored", &[("reason", cause.localize(l).into())])?,
+        (None, mean) => out.line(
+            "cli-import-scored",
+            &[
+                ("witness", reference.witness.as_str().into()),
+                ("mean", mean.unwrap_or_default().into()),
+                ("above", report.above_threshold().count().into()),
+                ("threshold", reference.threshold.into()),
+            ],
+        )?,
+    }
     Ok(())
 }
```

Fluent keys. English first (the reference locale), then the 19 others. Fluent plural categories differ per language; each file is exactly the draft file.

```diff
--- a/crates/oku-slides-i18n/src/locales/en/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/en/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = could not write the imported deck { $path }
 cli-import-hifi-no-target = --pack and --theme belong to retrofit mode: the high-fidelity import (--mode hifi) generates its own pack and theme
 cli-import-probe-mismatch = the generated pack does not probe as declared (widget { $widget }): this is a bug in the importer
 cli-import-probe-limit = the generated pack exceeds the probe limits (widget { $widget }): raise max_widgets, max_widget_kib or probe_budget_ms in the [render] section of the configuration
+cli-import-reference-hifi-only = --reference belongs to hifi mode: a retrofit import is not compared with a reference render
+cli-import-scored = Visual gap to { $witness }: { $mean } on average, { $above ->
+    [0] no slide
+    [one] { $above } slide
+   *[other] { $above } slides
+} above { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Render and store Oku Slides decks
 cli-arg-fonts = Extra font folder (TTF/OTF); default: paths.fonts in the settings
@@ -83,3 +90,4 @@ cli-arg-pack = Target pack: id, pack folder (pack.json, widgets/) or name
 cli-arg-import-theme = Target theme: a built-in or library theme, by id or name; default: the default theme
 cli-arg-report = Also write the import report (JSON): losses, fallbacks, coverage
 cli-arg-no-system-fonts = In hifi mode, do not look for the presentation's fonts among the system fonts
+cli-arg-reference = In hifi mode, compare each slide with a reference render of the presentation (auto, the default) or not (none)
```

`ar`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ar/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ar/cli.ftl
@@ -69,6 +69,13 @@ cli-import-write-failed = تعذّرت كتابة العرض المستورد {
 cli-import-hifi-no-target = --pack و--theme خاصّان بوضع retrofit: الاستيراد عالي الدقة (--mode hifi) يولّد حزمته وسمته
 cli-import-probe-mismatch = الحزمة المولَّدة لا تُفحص كما أُعلنت (الأداة { $widget }): هذا خلل في المستورِد
 cli-import-probe-limit = تتجاوز الحزمة المولَّدة حدود الفحص (الأداة { $widget }): ارفع max_widgets أو max_widget_kib أو probe_budget_ms في القسم [render] من الإعدادات
+cli-import-reference-hifi-only = ‎--reference خاص بوضع hifi: لا يُقارَن استيراد retrofit بعرض مرجعي
+cli-import-scored = الفرق المرئي عن { $witness }: { $mean } في المتوسط، { $above ->
+    [0] لا توجد شرائح
+    [one] { $above } شريحة
+   *[other] { $above } شرائح
+} فوق { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = عرض عروض Oku Slides التقديمية وحفظها
 cli-arg-fonts = مجلد خطوط إضافي (TTF/OTF)؛ الافتراضي: paths.fonts في الإعدادات
@@ -99,3 +106,4 @@ cli-arg-pack = حزمة القوالب الهدف: المعرّف أو مجلد
 cli-arg-import-theme = السمة الهدف: سمة مدمجة أو من المكتبة، بمعرّفها أو اسمها؛ الافتراضي: السمة الافتراضية
 cli-arg-report = كتابة تقرير الاستيراد (JSON) أيضًا: المفقودات والبدائل والتغطية
 cli-arg-no-system-fonts = في وضع hifi، لا تبحث عن خطوط العرض بين خطوط النظام
+cli-arg-reference = في وضع hifi، قارن كل شريحة بعرض مرجعي للعرض التقديمي (auto، الافتراضي) أو لا (none)
```

`bn`:

```diff
--- a/crates/oku-slides-i18n/src/locales/bn/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/bn/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = ইমপোর্ট করা ডেক { $path } 
 cli-import-hifi-no-target = --pack ও --theme শুধু retrofit মোডের জন্য: উচ্চ বিশ্বস্ততার ইমপোর্ট (--mode hifi) নিজের প্যাক ও থিম তৈরি করে
 cli-import-probe-mismatch = তৈরি করা প্যাক ঘোষণা অনুযায়ী যাচাই হয়নি (উইজেট { $widget }): এটি ইমপোর্টারের ত্রুটি
 cli-import-probe-limit = তৈরি প্যাকটি যাচাইয়ের সীমা ছাড়িয়েছে (উইজেট { $widget }): কনফিগারেশনের [render] অংশে max_widgets, max_widget_kib বা probe_budget_ms বাড়ান
+cli-import-reference-hifi-only = --reference hifi মোডের অংশ: retrofit আমদানি রেফারেন্স রেন্ডারের সঙ্গে তুলনা করা হয় না
+cli-import-scored = { $witness }-এর সঙ্গে দৃশ্যমান পার্থক্য: গড়ে { $mean }, { $threshold }-এর উপরে { $above ->
+    [0] কোনো স্লাইড নেই
+    [one] { $above }টি স্লাইড
+   *[other] { $above }টি স্লাইড
+}
+cli-import-unscored = { $reason }
 
 cli-about = Oku Slides প্রেজেন্টেশন রেন্ডার ও সংরক্ষণ করুন
 cli-arg-fonts = অতিরিক্ত ফন্ট ফোল্ডার (TTF/OTF); ডিফল্ট: সেটিংসে paths.fonts
@@ -83,3 +90,4 @@ cli-arg-pack = লক্ষ্য টেমপ্লেট প্যাক: আ
 cli-arg-import-theme = লক্ষ্য থিম: অন্তর্নির্মিত বা লাইব্রেরির থিম, আইডি বা নাম দিয়ে; ডিফল্ট: ডিফল্ট থিম
 cli-arg-report = ইমপোর্ট রিপোর্টও (JSON) লিখুন: ক্ষতি, বিকল্প, কভারেজ
 cli-arg-no-system-fonts = hifi মোডে, উপস্থাপনার ফন্ট সিস্টেম ফন্টে খুঁজবেন না
+cli-arg-reference = hifi মোডে, প্রতিটি স্লাইডকে উপস্থাপনার রেফারেন্স রেন্ডারের সঙ্গে তুলনা করুন (auto, ডিফল্ট) বা না (none)
```

`de`:

```diff
--- a/crates/oku-slides-i18n/src/locales/de/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/de/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = das importierte Deck { $path } konnte nicht geschriebe
 cli-import-hifi-no-target = --pack und --theme gelten nur im Modus retrofit: der originalgetreue Import (--mode hifi) erzeugt sein eigenes Paket und Design
 cli-import-probe-mismatch = das erzeugte Paket entspricht bei der Prüfung nicht seiner Deklaration (Widget { $widget }): ein Fehler des Imports
 cli-import-probe-limit = das erzeugte Paket überschreitet die Grenzen der Prüfung (Widget { $widget }): erhöhen Sie max_widgets, max_widget_kib oder probe_budget_ms im Abschnitt [render] der Konfiguration
+cli-import-reference-hifi-only = --reference gehört zum hifi-Modus: ein Retrofit-Import wird nicht mit einem Referenz-Rendering verglichen
+cli-import-scored = Visuelle Abweichung zu { $witness }: { $mean } im Mittel, { $above ->
+    [0] keine Folie
+    [one] { $above } Folie
+   *[other] { $above } Folien
+} über { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Oku-Slides-Präsentationen rendern und speichern
 cli-arg-fonts = Zusätzlicher Schriftordner (TTF/OTF); Standard: paths.fonts in den Einstellungen
@@ -83,3 +90,4 @@ cli-arg-pack = Ziel-Vorlagenpaket: ID, Paketordner (pack.json, widgets/) oder Na
 cli-arg-import-theme = Ziel-Design: ein mitgeliefertes oder eines aus der Bibliothek, per ID oder Name; Standard: das Standarddesign
 cli-arg-report = Zusätzlich den Importbericht (JSON) schreiben: Verluste, Ersatzlösungen, Abdeckung
 cli-arg-no-system-fonts = Im Modus hifi die Schriften der Präsentation nicht unter den Systemschriften suchen
+cli-arg-reference = Im hifi-Modus jede Folie mit einem Referenz-Rendering der Präsentation vergleichen (auto, Standard) oder nicht (none)
```

`es`:

```diff
--- a/crates/oku-slides-i18n/src/locales/es/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/es/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = no se pudo escribir la presentación importada { $path
 cli-import-hifi-no-target = --pack y --theme son del modo retrofit: la importación de alta fidelidad (--mode hifi) genera su propio paquete y tema
 cli-import-probe-mismatch = el paquete generado no se sondea como se declaró (widget { $widget }): es un error del importador
 cli-import-probe-limit = el paquete generado supera los límites de la sonda (widget { $widget }): aumente max_widgets, max_widget_kib o probe_budget_ms en la sección [render] de la configuración
+cli-import-reference-hifi-only = --reference pertenece al modo hifi: una importación retrofit no se compara con un renderizado de referencia
+cli-import-scored = Diferencia visual con { $witness }: { $mean } de media, { $above ->
+    [0] ninguna diapositiva
+    [one] { $above } diapositiva
+   *[other] { $above } diapositivas
+} por encima de { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Genera y almacena presentaciones de Oku Slides
 cli-arg-fonts = Carpeta de tipografías adicional (TTF/OTF); por defecto: paths.fonts en la configuración
@@ -83,3 +90,4 @@ cli-arg-pack = Paquete de destino: id, carpeta del paquete (pack.json, widgets/)
 cli-arg-import-theme = Tema de destino: uno incluido o de la biblioteca, por su id o nombre; por defecto: el tema predeterminado
 cli-arg-report = Escribir también el informe de importación (JSON): pérdidas, sustituciones, cobertura
 cli-arg-no-system-fonts = En modo hifi, no buscar las fuentes de la presentación entre las fuentes del sistema
+cli-arg-reference = En modo hifi, comparar cada diapositiva con un renderizado de referencia de la presentación (auto, por defecto) o no (none)
```

`fr`:

```diff
--- a/crates/oku-slides-i18n/src/locales/fr/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/fr/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = impossible d'écrire le deck importé { $path }
 cli-import-hifi-no-target = --pack et --theme sont propres au mode retrofit : l'import haute fidélité (--mode hifi) génère son propre pack et son propre thème
 cli-import-probe-mismatch = le pack généré ne se sonde pas comme il est déclaré (widget { $widget }) : c'est un bogue de l'import
 cli-import-probe-limit = le pack généré dépasse les bornes de la sonde (widget { $widget }) : relevez max_widgets, max_widget_kib ou probe_budget_ms dans la section [render] de la configuration
+cli-import-reference-hifi-only = --reference appartient au mode hifi : un import retrofit n'est pas comparé à un rendu de référence
+cli-import-scored = Écart visuel à { $witness } : { $mean } en moyenne, { $above ->
+    [0] aucune slide
+    [one] { $above } slide
+   *[other] { $above } slides
+} au-delà de { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Rendu et stockage des decks Oku Slides
 cli-arg-fonts = Dossier de polices supplémentaires (TTF/OTF) ; défaut : paths.fonts des réglages
@@ -83,3 +90,4 @@ cli-arg-pack = Pack cible : id, dossier de pack (pack.json, widgets/) ou nom
 cli-arg-import-theme = Thème cible : un thème intégré ou de la bibliothèque, par son id ou son nom ; défaut : le thème par défaut
 cli-arg-report = Écrit aussi le rapport d'import (JSON) : pertes, replis, couverture
 cli-arg-no-system-fonts = En mode hifi, ne cherche pas les polices de la présentation parmi celles du système
+cli-arg-reference = En mode hifi, comparer chaque slide à un rendu de référence de la présentation (auto, le défaut) ou non (none)
```

`hi`:

```diff
--- a/crates/oku-slides-i18n/src/locales/hi/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/hi/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = आयात किया गया डेक { $path
 cli-import-hifi-no-target = --pack और --theme केवल retrofit मोड के लिए हैं: उच्च-निष्ठा आयात (--mode hifi) अपना पैक और थीम ख़ुद बनाता है
 cli-import-probe-mismatch = बनाया गया पैक घोषणा के अनुसार जाँच में नहीं मिला (विजेट { $widget }): यह आयातक की त्रुटि है
 cli-import-probe-limit = बनाया गया पैक जाँच की सीमाएँ पार करता है (विजेट { $widget }): कॉन्फ़िगरेशन के [render] खंड में max_widgets, max_widget_kib या probe_budget_ms बढ़ाएँ
+cli-import-reference-hifi-only = --reference hifi मोड का हिस्सा है: retrofit आयात की तुलना संदर्भ रेंडर से नहीं होती
+cli-import-scored = { $witness } से दृश्य अंतर: औसत { $mean }, { $threshold } से ऊपर { $above ->
+    [0] कोई स्लाइड नहीं
+    [one] { $above } स्लाइड
+   *[other] { $above } स्लाइड
+}
+cli-import-unscored = { $reason }
 
 cli-about = Oku Slides प्रेज़ेंटेशन रेंडर और संग्रहीत करें
 cli-arg-fonts = अतिरिक्त फ़ॉन्ट फ़ोल्डर (TTF/OTF); डिफ़ॉल्ट: सेटिंग्स में paths.fonts
@@ -83,3 +90,4 @@ cli-arg-pack = लक्ष्य टेम्पलेट पैक: आईड
 cli-arg-import-theme = लक्ष्य थीम: बिल्ट-इन या लाइब्रेरी की थीम, आईडी या नाम से; डिफ़ॉल्ट: डिफ़ॉल्ट थीम
 cli-arg-report = आयात रिपोर्ट (JSON) भी लिखें: हानियाँ, विकल्प, कवरेज
 cli-arg-no-system-fonts = hifi मोड में, प्रस्तुति के फ़ॉन्ट सिस्टम फ़ॉन्ट में न खोजें
+cli-arg-reference = hifi मोड में, हर स्लाइड की तुलना प्रस्तुति के संदर्भ रेंडर से करें (auto, डिफ़ॉल्ट) या नहीं (none)
```

`it`:

```diff
--- a/crates/oku-slides-i18n/src/locales/it/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/it/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = impossibile scrivere la presentazione importata { $pat
 cli-import-hifi-no-target = --pack e --theme valgono solo in modalità retrofit: l'importazione ad alta fedeltà (--mode hifi) genera un proprio pacchetto e tema
 cli-import-probe-mismatch = il pacchetto generato non supera la sonda come dichiarato (widget { $widget }): è un errore dell'importazione
 cli-import-probe-limit = il pacchetto generato supera i limiti della sonda (widget { $widget }): aumenta max_widgets, max_widget_kib o probe_budget_ms nella sezione [render] della configurazione
+cli-import-reference-hifi-only = --reference appartiene alla modalità hifi: un'importazione retrofit non è confrontata con un rendering di riferimento
+cli-import-scored = Scarto visivo da { $witness }: { $mean } in media, { $above ->
+    [0] nessuna slide
+    [one] { $above } slide
+   *[other] { $above } slide
+} oltre { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Genera e archivia presentazioni Oku Slides
 cli-arg-fonts = Cartella di font aggiuntiva (TTF/OTF); predefinita: paths.fonts nelle impostazioni
@@ -83,3 +90,4 @@ cli-arg-pack = Pacchetto di destinazione: id, cartella del pacchetto (pack.json,
 cli-arg-import-theme = Tema di destinazione: uno incluso o della libreria, per id o nome; predefinito: il tema predefinito
 cli-arg-report = Scrivi anche il rapporto di importazione (JSON): perdite, sostituzioni, copertura
 cli-arg-no-system-fonts = In modalità hifi, non cercare i caratteri della presentazione tra quelli di sistema
+cli-arg-reference = In modalità hifi, confrontare ogni slide con un rendering di riferimento della presentazione (auto, predefinito) o no (none)
```

`ja`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ja/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ja/cli.ftl
@@ -41,6 +41,11 @@ cli-import-write-failed = インポートしたデッキ { $path } を書き込
 cli-import-hifi-no-target = --pack と --theme は retrofit モード専用です。高忠実度インポート（--mode hifi）は独自のパックとテーマを生成します
 cli-import-probe-mismatch = 生成されたパックが宣言どおりに検査されませんでした（ウィジェット { $widget }）。インポーターの不具合です
 cli-import-probe-limit = 生成したパックが検査の上限を超えています（ウィジェット { $widget }）：設定の [render] セクションで max_widgets、max_widget_kib、probe_budget_ms を引き上げてください
+cli-import-reference-hifi-only = --reference は hifi モード用です：retrofit インポートは参照レンダリングと比較されません
+cli-import-scored = { $witness } との見た目の差：平均 { $mean }、{ $threshold } を超えるスライド { $above ->
+   *[other] { $above }枚
+}
+cli-import-unscored = { $reason }
 
 cli-about = Oku Slides のプレゼンテーションをレンダリングして保存します
 cli-arg-fonts = 追加のフォントフォルダ（TTF/OTF）。デフォルト：設定の paths.fonts
@@ -71,3 +76,4 @@ cli-arg-pack = 対象のテンプレートパック：ID、パックのフォル
 cli-arg-import-theme = 対象のテーマ：組み込みまたはライブラリのテーマを ID か名前で指定。既定：既定のテーマ
 cli-arg-report = インポートレポート（JSON）も書き出します：損失、代替、カバレッジ
 cli-arg-no-system-fonts = hifi モードで、プレゼンテーションのフォントをシステムフォントから探しません
+cli-arg-reference = hifi モードで、各スライドをプレゼンテーションの参照レンダリングと比較する（auto、既定）かしない（none）
```

`ko`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ko/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ko/cli.ftl
@@ -41,6 +41,11 @@ cli-import-write-failed = 가져온 덱 { $path }을(를) 쓸 수 없습니다
 cli-import-hifi-no-target = --pack과 --theme는 retrofit 모드 전용입니다: 고충실도 가져오기(--mode hifi)는 자체 팩과 테마를 생성합니다
 cli-import-probe-mismatch = 생성된 팩이 선언대로 검사되지 않았습니다(위젯 { $widget }): 가져오기 기능의 버그입니다
 cli-import-probe-limit = 생성된 팩이 검사 한도를 넘습니다(위젯 { $widget }): 설정의 [render] 섹션에서 max_widgets, max_widget_kib 또는 probe_budget_ms를 늘리세요
+cli-import-reference-hifi-only = --reference는 hifi 모드용입니다: retrofit 가져오기는 참조 렌더링과 비교하지 않습니다
+cli-import-scored = { $witness }와의 시각 차이: 평균 { $mean }, { $threshold } 초과 슬라이드 { $above ->
+   *[other] { $above }개
+}
+cli-import-unscored = { $reason }
 
 cli-about = Oku Slides 프레젠테이션을 렌더링하고 저장합니다
 cli-arg-fonts = 추가 글꼴 폴더 (TTF/OTF). 기본값: 설정의 paths.fonts
@@ -71,3 +76,4 @@ cli-arg-pack = 대상 템플릿 팩: ID, 팩 폴더(pack.json, widgets/) 또는
 cli-arg-import-theme = 대상 테마: 기본 제공 테마 또는 라이브러리 테마를 ID나 이름으로 지정; 기본값: 기본 테마
 cli-arg-report = 가져오기 보고서(JSON)도 쓰기: 손실, 대체, 포함 범위
 cli-arg-no-system-fonts = hifi 모드에서 프레젠테이션의 글꼴을 시스템 글꼴에서 찾지 않음
+cli-arg-reference = hifi 모드에서 각 슬라이드를 프레젠테이션의 참조 렌더링과 비교(auto, 기본값)하거나 하지 않음(none)
```

`nl`:

```diff
--- a/crates/oku-slides-i18n/src/locales/nl/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/nl/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = kon de geïmporteerde presentatie { $path } niet schri
 cli-import-hifi-no-target = --pack en --theme horen bij de modus retrofit: import met hoge getrouwheid (--mode hifi) maakt een eigen pakket en thema
 cli-import-probe-mismatch = het gemaakte pakket komt bij het peilen niet overeen met de declaratie (widget { $widget }): een fout in de import
 cli-import-probe-limit = het gegenereerde pakket overschrijdt de grenzen van de controle (widget { $widget }): verhoog max_widgets, max_widget_kib of probe_budget_ms in de sectie [render] van de configuratie
+cli-import-reference-hifi-only = --reference hoort bij de hifi-modus: een retrofit-import wordt niet met een referentieweergave vergeleken
+cli-import-scored = Visueel verschil met { $witness }: gemiddeld { $mean }, { $above ->
+    [0] geen dia
+    [one] { $above } dia
+   *[other] { $above } dia’s
+} boven { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Oku Slides-presentaties weergeven en opslaan
 cli-arg-fonts = Extra lettertypemap (TTF/OTF); standaard: paths.fonts in de instellingen
@@ -83,3 +90,4 @@ cli-arg-pack = Doelpakket: id, pakketmap (pack.json, widgets/) of naam
 cli-arg-import-theme = Doelthema: een ingebouwd thema of een uit de bibliotheek, op id of naam; standaard: het standaardthema
 cli-arg-report = Ook het importrapport (JSON) schrijven: verliezen, vervangingen, dekking
 cli-arg-no-system-fonts = In modus hifi de lettertypen van de presentatie niet tussen de systeemlettertypen zoeken
+cli-arg-reference = In hifi-modus elke dia vergelijken met een referentieweergave van de presentatie (auto, standaard) of niet (none)
```

`pl`:

```diff
--- a/crates/oku-slides-i18n/src/locales/pl/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/pl/cli.ftl
@@ -61,6 +61,14 @@ cli-import-write-failed = nie udało się zapisać zaimportowanej prezentacji {
 cli-import-hifi-no-target = --pack i --theme dotyczą tylko trybu retrofit: import o wysokiej wierności (--mode hifi) tworzy własny pakiet i motyw
 cli-import-probe-mismatch = wygenerowany pakiet nie odpowiada swojej deklaracji przy sprawdzaniu (widżet { $widget }): to błąd importu
 cli-import-probe-limit = wygenerowany pakiet przekracza limity sondy (widżet { $widget }): zwiększ max_widgets, max_widget_kib lub probe_budget_ms w sekcji [render] konfiguracji
+cli-import-reference-hifi-only = --reference należy do trybu hifi: import retrofit nie jest porównywany z renderowaniem referencyjnym
+cli-import-scored = Różnica wizualna względem { $witness }: średnio { $mean }, { $above ->
+    [0] żaden slajd
+    [one] { $above } slajd
+    [few] { $above } slajdy
+   *[other] { $above } slajdów
+} powyżej { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Renderowanie i zapisywanie prezentacji Oku Slides
 cli-arg-fonts = Dodatkowy folder czcionek (TTF/OTF); domyślnie: paths.fonts w ustawieniach
@@ -91,3 +99,4 @@ cli-arg-pack = Pakiet docelowy: identyfikator, folder pakietu (pack.json, widget
 cli-arg-import-theme = Motyw docelowy: wbudowany lub z biblioteki, wskazany identyfikatorem lub nazwą; domyślnie: motyw domyślny
 cli-arg-report = Zapisz też raport importu (JSON): straty, zamienniki, pokrycie
 cli-arg-no-system-fonts = W trybie hifi nie szukaj czcionek prezentacji wśród czcionek systemowych
+cli-arg-reference = W trybie hifi porównać każdy slajd z renderowaniem referencyjnym prezentacji (auto, domyślnie) lub nie (none)
```

`pt`:

```diff
--- a/crates/oku-slides-i18n/src/locales/pt/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/pt/cli.ftl
@@ -53,6 +53,13 @@ cli-import-write-failed = não foi possível gravar a apresentação importada {
 cli-import-hifi-no-target = --pack e --theme são do modo retrofit: a importação de alta fidelidade (--mode hifi) gera o seu próprio pacote e tema
 cli-import-probe-mismatch = o pacote gerado não corresponde à sondagem como declarado (widget { $widget }): é um erro do importador
 cli-import-probe-limit = o pacote gerado excede os limites da sonda (widget { $widget }): aumente max_widgets, max_widget_kib ou probe_budget_ms na seção [render] da configuração
+cli-import-reference-hifi-only = --reference pertence ao modo hifi: uma importação retrofit não é comparada com uma renderização de referência
+cli-import-scored = Diferença visual para { $witness }: { $mean } em média, { $above ->
+    [0] nenhum slide
+    [one] { $above } slide
+   *[other] { $above } slides
+} acima de { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Renderiza e armazena apresentações do Oku Slides
 cli-arg-fonts = Pasta de tipos de letra extra (TTF/OTF); padrão: paths.fonts nas configurações
@@ -83,3 +90,4 @@ cli-arg-pack = Pacote de destino: id, pasta do pacote (pack.json, widgets/) ou n
 cli-arg-import-theme = Tema de destino: um tema incluído ou da biblioteca, pelo id ou nome; padrão: o tema padrão
 cli-arg-report = Gravar também o relatório de importação (JSON): perdas, substituições, cobertura
 cli-arg-no-system-fonts = No modo hifi, não procurar as fontes da apresentação entre as fontes do sistema
+cli-arg-reference = No modo hifi, comparar cada slide com uma renderização de referência da apresentação (auto, o padrão) ou não (none)
```

`ru`:

```diff
--- a/crates/oku-slides-i18n/src/locales/ru/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ru/cli.ftl
@@ -61,6 +61,14 @@ cli-import-write-failed = не удалось записать импортир
 cli-import-hifi-no-target = --pack и --theme относятся к режиму retrofit: импорт с высокой точностью (--mode hifi) создаёт собственный пакет и тему
 cli-import-probe-mismatch = созданный пакет не прошёл проверку в соответствии с объявлением (виджет { $widget }): это ошибка импорта
 cli-import-probe-limit = созданный пакет превышает ограничения проверки (виджет { $widget }): увеличьте max_widgets, max_widget_kib или probe_budget_ms в разделе [render] конфигурации
+cli-import-reference-hifi-only = --reference относится к режиму hifi: импорт retrofit не сравнивается с эталонным рендерингом
+cli-import-scored = Визуальное отличие от { $witness }: в среднем { $mean }, { $above ->
+    [0] ни одного слайда
+    [one] { $above } слайд
+    [few] { $above } слайда
+   *[other] { $above } слайдов
+} выше { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Отрисовка и хранение презентаций Oku Slides
 cli-arg-fonts = Дополнительная папка шрифтов (TTF/OTF); по умолчанию: paths.fonts в настройках
@@ -91,3 +99,4 @@ cli-arg-pack = Целевой пакет: идентификатор, папка
 cli-arg-import-theme = Целевая тема: встроенная или из библиотеки, по идентификатору или названию; по умолчанию: тема по умолчанию
 cli-arg-report = Также записать отчёт об импорте (JSON): потери, замены, охват
 cli-arg-no-system-fonts = В режиме hifi не искать шрифты презентации среди системных шрифтов
+cli-arg-reference = В режиме hifi сравнивать каждый слайд с эталонным рендерингом презентации (auto, по умолчанию) или нет (none)
```

`th`:

```diff
--- a/crates/oku-slides-i18n/src/locales/th/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/th/cli.ftl
@@ -41,6 +41,11 @@ cli-import-write-failed = เขียนเด็คที่นำเข้า
 cli-import-hifi-no-target = --pack และ --theme ใช้กับโหมด retrofit เท่านั้น: การนำเข้าแบบเที่ยงตรงสูง (--mode hifi) สร้างแพ็กและธีมของตัวเอง
 cli-import-probe-mismatch = แพ็กที่สร้างขึ้นตรวจสอบได้ไม่ตรงกับที่ประกาศ (วิดเจ็ต { $widget }): เป็นข้อบกพร่องของตัวนำเข้า
 cli-import-probe-limit = แพ็กที่สร้างเกินขีดจำกัดของการตรวจ (วิดเจ็ต { $widget }): เพิ่ม max_widgets, max_widget_kib หรือ probe_budget_ms ในส่วน [render] ของการตั้งค่า
+cli-import-reference-hifi-only = --reference เป็นของโหมด hifi: การนำเข้าแบบ retrofit ไม่ถูกเปรียบเทียบกับการเรนเดอร์อ้างอิง
+cli-import-scored = ความต่างทางภาพจาก { $witness }: เฉลี่ย { $mean }, สไลด์ที่เกิน { $threshold } { $above ->
+   *[other] { $above } แผ่น
+}
+cli-import-unscored = { $reason }
 
 cli-about = เรนเดอร์และจัดเก็บงานนำเสนอ Oku Slides
 cli-arg-fonts = โฟลเดอร์ฟอนต์เพิ่มเติม (TTF/OTF); ค่าเริ่มต้น: paths.fonts ในการตั้งค่า
@@ -71,3 +76,4 @@ cli-arg-pack = แพ็กเทมเพลตเป้าหมาย: รห
 cli-arg-import-theme = ธีมเป้าหมาย: ธีมในตัวหรือจากคลัง ระบุด้วยรหัสหรือชื่อ; ค่าเริ่มต้น: ธีมเริ่มต้น
 cli-arg-report = เขียนรายงานการนำเข้า (JSON) ด้วย: การสูญเสีย ทางเลือกสำรอง ความครอบคลุม
 cli-arg-no-system-fonts = ในโหมด hifi ไม่ต้องค้นหาฟอนต์ของงานนำเสนอในฟอนต์ของระบบ
+cli-arg-reference = ในโหมด hifi ให้เปรียบเทียบแต่ละสไลด์กับการเรนเดอร์อ้างอิงของงานนำเสนอ (auto ค่าเริ่มต้น) หรือไม่ (none)
```

`tr`:

```diff
--- a/crates/oku-slides-i18n/src/locales/tr/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/tr/cli.ftl
@@ -53,6 +53,12 @@ cli-import-write-failed = içe aktarılan sunu { $path } yazılamadı
 cli-import-hifi-no-target = --pack ve --theme yalnızca retrofit modu içindir: yüksek doğruluklu içe aktarma (--mode hifi) kendi paketini ve temasını üretir
 cli-import-probe-mismatch = üretilen paket bildirildiği gibi yoklanmadı (widget { $widget }): içe aktarıcıda bir hata
 cli-import-probe-limit = oluşturulan paket denetim sınırlarını aşıyor (widget { $widget }): yapılandırmanın [render] bölümünde max_widgets, max_widget_kib veya probe_budget_ms değerini artırın
+cli-import-reference-hifi-only = --reference hifi moduna aittir: retrofit içe aktarma bir referans oluşturmayla karşılaştırılmaz
+cli-import-scored = { $witness } ile görsel fark: ortalama { $mean }, { $threshold } üzerinde { $above ->
+    [0] hiç slayt yok
+   *[other] { $above } slayt
+}
+cli-import-unscored = { $reason }
 
 cli-about = Oku Slides sunularını oluşturur ve saklar
 cli-arg-fonts = Ek yazı tipi klasörü (TTF/OTF); varsayılan: ayarlardaki paths.fonts
@@ -83,3 +89,4 @@ cli-arg-pack = Hedef şablon paketi: kimlik, paket klasörü (pack.json, widgets
 cli-arg-import-theme = Hedef tema: yerleşik ya da kitaplıktaki bir tema, kimliği ya da adıyla; varsayılan: varsayılan tema
 cli-arg-report = İçe aktarma raporunu da (JSON) yaz: kayıplar, yedekler, kapsam
 cli-arg-no-system-fonts = hifi modunda sununun yazı tiplerini sistem yazı tipleri arasında arama
+cli-arg-reference = hifi modunda her slaytı sununun referans oluşturmasıyla karşılaştır (auto, varsayılan) ya da karşılaştırma (none)
```

`uk`:

```diff
--- a/crates/oku-slides-i18n/src/locales/uk/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/uk/cli.ftl
@@ -61,6 +61,14 @@ cli-import-write-failed = не вдалося записати імпортов
 cli-import-hifi-no-target = --pack і --theme стосуються режиму retrofit: імпорт із високою точністю (--mode hifi) створює власний пакет і тему
 cli-import-probe-mismatch = створений пакет не пройшов перевірку відповідно до оголошення (віджет { $widget }): це помилка імпорту
 cli-import-probe-limit = створений пакет перевищує обмеження перевірки (віджет { $widget }): збільште max_widgets, max_widget_kib або probe_budget_ms у розділі [render] конфігурації
+cli-import-reference-hifi-only = --reference належить до режиму hifi: імпорт retrofit не порівнюється з еталонним рендерингом
+cli-import-scored = Візуальна відмінність від { $witness }: у середньому { $mean }, { $above ->
+    [0] жодного слайда
+    [one] { $above } слайд
+    [few] { $above } слайди
+   *[other] { $above } слайдів
+} вище { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Відтворення та зберігання презентацій Oku Slides
 cli-arg-fonts = Додаткова тека шрифтів (TTF/OTF); типово: paths.fonts у налаштуваннях
@@ -91,3 +99,4 @@ cli-arg-pack = Цільовий пакет: ідентифікатор, тека
 cli-arg-import-theme = Цільова тема: вбудована або з бібліотеки, за ідентифікатором чи назвою; типово: тема за замовчуванням
 cli-arg-report = Також записати звіт про імпорт (JSON): втрати, заміни, охоплення
 cli-arg-no-system-fonts = У режимі hifi не шукати шрифти презентації серед системних шрифтів
+cli-arg-reference = У режимі hifi порівнювати кожен слайд з еталонним рендерингом презентації (auto, типово) чи ні (none)
```

`vi`:

```diff
--- a/crates/oku-slides-i18n/src/locales/vi/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/vi/cli.ftl
@@ -41,6 +41,12 @@ cli-import-write-failed = không thể ghi bản trình bày đã nhập { $path
 cli-import-hifi-no-target = --pack và --theme chỉ dùng cho chế độ retrofit: chế độ nhập độ trung thực cao (--mode hifi) tự tạo gói và chủ đề riêng
 cli-import-probe-mismatch = gói được tạo không khớp với khai báo khi kiểm tra (widget { $widget }): đây là lỗi của trình nhập
 cli-import-probe-limit = gói được tạo vượt giới hạn kiểm tra (widget { $widget }): hãy tăng max_widgets, max_widget_kib hoặc probe_budget_ms trong mục [render] của cấu hình
+cli-import-reference-hifi-only = --reference thuộc chế độ hifi: bản nhập retrofit không được so sánh với bản kết xuất tham chiếu
+cli-import-scored = Khác biệt hình ảnh so với { $witness }: trung bình { $mean }, { $above ->
+    [0] không trang chiếu nào
+   *[other] { $above } trang chiếu
+} vượt quá { $threshold }
+cli-import-unscored = { $reason }
 
 cli-about = Kết xuất và lưu trữ bản trình bày Oku Slides
 cli-arg-fonts = Thư mục phông chữ bổ sung (TTF/OTF); mặc định: paths.fonts trong cài đặt
@@ -71,3 +77,4 @@ cli-arg-pack = Gói mẫu đích: mã, thư mục gói (pack.json, widgets/) ho
 cli-arg-import-theme = Chủ đề đích: chủ đề có sẵn hoặc trong thư viện, theo mã hoặc tên; mặc định: chủ đề mặc định
 cli-arg-report = Ghi thêm báo cáo nhập (JSON): mất mát, phương án thay thế, độ bao phủ
 cli-arg-no-system-fonts = Ở chế độ hifi, không tìm phông chữ của bản trình bày trong phông chữ hệ thống
+cli-arg-reference = Ở chế độ hifi, so sánh từng trang chiếu với bản kết xuất tham chiếu của bản trình bày (auto, mặc định) hoặc không (none)
```

`zh-Hans`:

```diff
--- a/crates/oku-slides-i18n/src/locales/zh-Hans/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/zh-Hans/cli.ftl
@@ -41,6 +41,11 @@ cli-import-write-failed = 无法写入导入的演示文稿 { $path }
 cli-import-hifi-no-target = --pack 和 --theme 仅用于 retrofit 模式：高保真导入（--mode hifi）会生成自己的模板包和主题
 cli-import-probe-mismatch = 生成的模板包探测结果与声明不符（组件 { $widget }）：这是导入程序的缺陷
 cli-import-probe-limit = 生成的包超出检查限制（组件 { $widget }）：请在配置的 [render] 部分提高 max_widgets、max_widget_kib 或 probe_budget_ms
+cli-import-reference-hifi-only = --reference 属于 hifi 模式：retrofit 导入不与参考渲染比较
+cli-import-scored = 与 { $witness } 的视觉差异：平均 { $mean }，超过 { $threshold } 的幻灯片 { $above ->
+   *[other] { $above }张
+}
+cli-import-unscored = { $reason }
 
 cli-about = 渲染并存储 Oku Slides 演示文稿
 cli-arg-fonts = 额外字体文件夹（TTF/OTF）；默认：设置中的 paths.fonts
@@ -71,3 +76,4 @@ cli-arg-pack = 目标模板包：ID、模板包文件夹（pack.json、widgets/
 cli-arg-import-theme = 目标主题：内置主题或库中的主题，按 ID 或名称指定；默认：默认主题
 cli-arg-report = 同时写出导入报告（JSON）：损失、替代、覆盖率
 cli-arg-no-system-fonts = 在 hifi 模式下，不在系统字体中查找演示文稿的字体
+cli-arg-reference = 在 hifi 模式下，将每张幻灯片与演示文稿的参考渲染比较（auto，默认）或不比较（none）
```

`zh-Hant`:

```diff
--- a/crates/oku-slides-i18n/src/locales/zh-Hant/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/zh-Hant/cli.ftl
@@ -41,6 +41,11 @@ cli-import-write-failed = 無法寫入匯入的簡報 { $path }
 cli-import-hifi-no-target = --pack 與 --theme 僅用於 retrofit 模式：高保真匯入（--mode hifi）會產生自己的範本套件與主題
 cli-import-probe-mismatch = 產生的範本套件探測結果與宣告不符（元件 { $widget }）：這是匯入程式的缺陷
 cli-import-probe-limit = 產生的套件超出檢查限制（元件 { $widget }）：請在設定的 [render] 區段提高 max_widgets、max_widget_kib 或 probe_budget_ms
+cli-import-reference-hifi-only = --reference 屬於 hifi 模式：retrofit 匯入不與參考轉譯比較
+cli-import-scored = 與 { $witness } 的視覺差異：平均 { $mean }，超過 { $threshold } 的投影片 { $above ->
+   *[other] { $above }張
+}
+cli-import-unscored = { $reason }
 
 cli-about = 渲染並儲存 Oku Slides 簡報
 cli-arg-fonts = 額外字型資料夾（TTF/OTF）；預設：設定中的 paths.fonts
@@ -71,3 +76,4 @@ cli-arg-pack = 目標範本套件：ID、套件資料夾（pack.json、widgets/
 cli-arg-import-theme = 目標主題：內建主題或資料庫中的主題，以 ID 或名稱指定；預設：預設主題
 cli-arg-report = 同時寫出匯入報告（JSON）：損失、替代、涵蓋率
 cli-arg-no-system-fonts = 在 hifi 模式下，不在系統字型中尋找簡報的字型
+cli-arg-reference = 在 hifi 模式下，將每張投影片與簡報的參考轉譯比較（auto，預設）或不比較（none）
```

The `ar` file is given as fixed by draft commit `dc0dd55` (the first draft also rewrote the `[one]` variant of `cli-imported` by mistake): when extracting, take `crates/oku-slides-i18n/src/locales/ar/cli.ftl` from `dc0dd55`, not from `4982645`.

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-platform -p oku-slides-import -p oku-slides-i18n -p oku-slides-cli`
Expected: all pass, including the 8 tests of `import_reference` (alto takes about 15 s per test in debug), the 5 `import_overflow_tests`, `source_rect_of_a_4_3_source_is_the_centered_band` and the two settings tests. The i18n tests check that the 20 locales carry the same keys.

Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-cli -p oku-slides-import -p oku-slides-platform --all-targets -- -D warnings`
Expected: no warning.

Run: `cargo fmt -p oku-slides-cli -p oku-slides-import -p oku-slides-platform -- --check`
Expected: no diff.

Manual check (optional): create two fresh directories with `mktemp -d`, then run `OKU_SLIDES_PATHS__DATA=<first dir> OKU_SLIDES_PATHS__DECKS=<second dir> /Users/matthieu/Projects/slider/target/debug/oku-slides --fonts assets/fonts import fixtures/import/libreoffice.pptx --mode hifi --no-system-fonts -o <second dir>/out.oku-slides.pdf`. The output has two lines, the second starting with `Visual gap to office2pdf 0.8.1: `.

- [ ] **Step 5: Commit**

```bash
git add Cargo.lock crates/oku-slides-cli/Cargo.toml crates/oku-slides-cli/src/import.rs crates/oku-slides-cli/src/import_overflow.rs crates/oku-slides-cli/src/import_overflow_tests.rs crates/oku-slides-cli/src/import_reference.rs crates/oku-slides-cli/src/main.rs crates/oku-slides-cli/src/pack_render.rs crates/oku-slides-cli/tests/import_reference.rs crates/oku-slides-import/src/hifi/mod.rs crates/oku-slides-import/src/hifi/page.rs crates/oku-slides-import/src/hifi/page_tests.rs crates/oku-slides-platform/src/config.default.toml crates/oku-slides-platform/src/config.rs crates/oku-slides-platform/src/config_tests.rs crates/oku-slides-i18n/src/locales/ar/cli.ftl crates/oku-slides-i18n/src/locales/bn/cli.ftl crates/oku-slides-i18n/src/locales/de/cli.ftl crates/oku-slides-i18n/src/locales/en/cli.ftl crates/oku-slides-i18n/src/locales/es/cli.ftl crates/oku-slides-i18n/src/locales/fr/cli.ftl crates/oku-slides-i18n/src/locales/hi/cli.ftl crates/oku-slides-i18n/src/locales/it/cli.ftl crates/oku-slides-i18n/src/locales/ja/cli.ftl crates/oku-slides-i18n/src/locales/ko/cli.ftl crates/oku-slides-i18n/src/locales/nl/cli.ftl crates/oku-slides-i18n/src/locales/pl/cli.ftl crates/oku-slides-i18n/src/locales/pt/cli.ftl crates/oku-slides-i18n/src/locales/ru/cli.ftl crates/oku-slides-i18n/src/locales/th/cli.ftl crates/oku-slides-i18n/src/locales/tr/cli.ftl crates/oku-slides-i18n/src/locales/uk/cli.ftl crates/oku-slides-i18n/src/locales/vi/cli.ftl crates/oku-slides-i18n/src/locales/zh-Hans/cli.ftl crates/oku-slides-i18n/src/locales/zh-Hant/cli.ftl
```

Write the message to a file (for example `/private/tmp/claude-501/-Users-matthieu-Projects-slider/d50cb8db-f8d6-4853-8a19-42f8d216aa16/scratchpad/commit-msg-t6.txt`) with this single line, without attribution:

```
feat(cli) : oku-slides import --reference auto|none, écart visuel par slide et débordements au rapport (t-692)
```

then commit as a separate command:

```bash
git commit -F /private/tmp/claude-501/-Users-matthieu-Projects-slider/d50cb8db-f8d6-4853-8a19-42f8d216aa16/scratchpad/commit-msg-t6.txt
```

**Extracting from the draft:** the implementer may take the files of this task from the draft commit `4982645` instead of retyping them, with an explicit list (never a directory), from the draft worktree:

```bash
git checkout 4982645 -- crates/oku-slides-cli/Cargo.toml crates/oku-slides-cli/src/import.rs crates/oku-slides-cli/src/import_overflow.rs crates/oku-slides-cli/src/import_overflow_tests.rs crates/oku-slides-cli/src/import_reference.rs crates/oku-slides-cli/src/main.rs crates/oku-slides-cli/src/pack_render.rs crates/oku-slides-cli/tests/import_reference.rs crates/oku-slides-import/src/hifi/mod.rs crates/oku-slides-import/src/hifi/page.rs crates/oku-slides-import/src/hifi/page_tests.rs crates/oku-slides-platform/src/config.default.toml crates/oku-slides-platform/src/config.rs crates/oku-slides-platform/src/config_tests.rs crates/oku-slides-i18n/src/locales/ar/cli.ftl crates/oku-slides-i18n/src/locales/bn/cli.ftl crates/oku-slides-i18n/src/locales/de/cli.ftl crates/oku-slides-i18n/src/locales/en/cli.ftl crates/oku-slides-i18n/src/locales/es/cli.ftl crates/oku-slides-i18n/src/locales/fr/cli.ftl crates/oku-slides-i18n/src/locales/hi/cli.ftl crates/oku-slides-i18n/src/locales/it/cli.ftl crates/oku-slides-i18n/src/locales/ja/cli.ftl crates/oku-slides-i18n/src/locales/ko/cli.ftl crates/oku-slides-i18n/src/locales/nl/cli.ftl crates/oku-slides-i18n/src/locales/pl/cli.ftl crates/oku-slides-i18n/src/locales/pt/cli.ftl crates/oku-slides-i18n/src/locales/ru/cli.ftl crates/oku-slides-i18n/src/locales/th/cli.ftl crates/oku-slides-i18n/src/locales/tr/cli.ftl crates/oku-slides-i18n/src/locales/uk/cli.ftl crates/oku-slides-i18n/src/locales/vi/cli.ftl crates/oku-slides-i18n/src/locales/zh-Hans/cli.ftl crates/oku-slides-i18n/src/locales/zh-Hant/cli.ftl
```

Only `Cargo.lock` of this task is also changed on `main` since the draft base `53454df2` (compare with `git diff --name-only 53454df2 main`): do not check it out; let cargo update it as described in Step 3 (or apply `git show 4982645 -- Cargo.lock | git apply --3way`). The file list above is `git diff --name-only 4982645~1 4982645` minus `Cargo.lock`; the other files of this task are unchanged on `main`.

### Task 7: Documents: spec decisions, ARCHITECTURE.md in English, AGENTS.md rules

This task delivers the documentation of lot 4: the spec section "Decisions from the lot 4 plan (2026-10-14)" and the spec sections it supersedes (witness instead of LibreOffice), the English `ARCHITECTURE.md` of the import, CLI and render-worker crates, the new `crates/oku-slides-reference/AGENTS.md`, the AGENTS.md rules and map entry, the README crate table and the fixtures README. No code changes. The diffs below are computed against `main` (not against the draft parent), because the draft merged `main` (commit 6d7037a) before this commit, so `main` already holds the English `ARCHITECTURE.md` baseline for the import and CLI crates. The render-worker `ARCHITECTURE.md` diff is the whole English version. `crates/oku-slides-reference/ARCHITECTURE.md` is created by Task 1 and is not repeated here.

**Files:**
- Create: `crates/oku-slides-reference/AGENTS.md` (agent rules of the new crate)
- Modify: `AGENTS.md` (Harness map entry for the reference crate)
- Modify: `README.md` (crate table: reference crate, import row mentions visual gap)
- Modify: `crates/oku-slides-cli/AGENTS.md` (testing rule for `--fonts assets/fonts`)
- Modify: `crates/oku-slides-cli/ARCHITECTURE.md` (`write_rendered`, `import_overflow.rs`, `import_reference.rs`)
- Modify: `crates/oku-slides-import/AGENTS.md` (settled decision on the witness)
- Modify: `crates/oku-slides-import/ARCHITECTURE.md` (`hifi::source_rect`, `ReferenceReport`, `metrics`)
- Modify: `crates/oku-slides-render-worker/ARCHITECTURE.md` (English version, `RemoteReference`, 46 lines)
- Modify: `docs/superpowers/specs/2026-10-09-import-pptx-design.md` (lot 4 decisions, reference/metrics sections, CLI, fixtures, lots list)
- Modify: `fixtures/import/README.md` (English, `alto-wealth.pdf` is no longer the lot 4 target)
- Test: `just agents-lint`, and `wc -l` on each `ARCHITECTURE.md` (at most 50 lines)

**Interfaces:**
- Consumes: nothing new (the names quoted in the documents come from Tasks 1 to 6).
- Produces: nothing for later tasks.

- [ ] **Step 1: Apply the document diffs**

Every `ARCHITECTURE.md` stays at most 50 lines: import (see Step 2), CLI, render-worker (46 lines), reference (Task 1).

`AGENTS.md`: Harness map entry.

```diff
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -118,6 +118,7 @@ rule sections are linted. `just agents-lint` checks all this; catch-up: `docs/ha
 - `crates/oku-slides-import/AGENTS.md` — Read before changing PPTX import, the retrofit projector or its tests
 - `crates/oku-slides-mcp/AGENTS.md` — Read before changing MCP tools, SurfaceHub turns or the tool server
 - `crates/oku-slides-platform/AGENTS.md` — Read before changing Storage, config defaults or agent presets.
+- `crates/oku-slides-reference/AGENTS.md` — Read before changing the reference witness or page rasterization.
 - `crates/oku-slides-render/AGENTS.md` — Read before touching Typst rendering, widget probes, packs or tables.
 - `crates/oku-slides-render-worker/AGENTS.md` — Read before touching the sandboxed Typst worker or its watchdog.
 - `crates/oku-slides-session/AGENTS.md` — Read before editing session, workspace, resume, undo or agent code
```

`README.md`: crate table.

```diff
--- a/README.md
+++ b/README.md
@@ -21,7 +21,8 @@ output are in French.
 | `crates/oku-slides-render-worker` | Typst rendering in a worker process, bounded in time and memory |
 | `crates/oku-slides-chart` | Charts and tables: ECharts option subset, SVG via charts-rs, table model, data operations |
 | `crates/oku-slides-pdf` | Source blob inside the PDF (`/PieceInfo`): write, read, strip |
-| `crates/oku-slides-import` | PPTX import: `SourceDeck` intermediate representation, reader, retrofit and high-fidelity projectors, font resolution, report and content coverage |
+| `crates/oku-slides-import` | PPTX import: `SourceDeck` intermediate representation, reader, retrofit and high-fidelity projectors, font resolution, report, content coverage and visual gap |
+| `crates/oku-slides-reference` | Reference rendering of an imported presentation (`office2pdf` witness, run by the render worker) and `hayro` rasterization of its pages |
 | `crates/oku-slides-session` | Editing session (open deck, history, rendering, file), shared by the app and the dev server |
 | `crates/oku-slides-platform` | Foundation: logging, settings, storage, languages; bounded accept loop for local HTTP servers |
 | `crates/oku-slides-i18n` | Fluent messages (20 languages, English as the reference) and language selection |
```

`crates/oku-slides-cli/AGENTS.md`: testing rule.

```diff
--- a/crates/oku-slides-cli/AGENTS.md
+++ b/crates/oku-slides-cli/AGENTS.md
@@ -20,6 +20,7 @@
 
 - In CLI import tests, build pptx zips in memory and never launch LibreOffice or the app. (plan:2026-10-10-import-pptx-lot2)
 - Run CLI tests through the oku_slides() helper so the data directory points at a temporary dir. (plan:2026-10-10-import-pptx-lot2)
+- In reference tests, pass --fonts assets/fonts so Oku and the witness render with the app fonts. (t-692)
 
 ## Settled decisions
 
```

`crates/oku-slides-cli/ARCHITECTURE.md`: CLI architecture.

```diff
--- a/crates/oku-slides-cli/ARCHITECTURE.md
+++ b/crates/oku-slides-cli/ARCHITECTURE.md
@@ -9,7 +9,7 @@ without Tauri. Results go to stdout, logs to stderr, help in the user's language
 - `main.rs`: `Cli` and `Cmd` (clap), with help text from Fluent `cli-cmd-…` and
   `cli-arg-…` keys (`command`); `run` dispatches each `Cmd`. Shared helpers load a deck
   from JSON or `.oku-slides.pdf` (`load_bundle`), embed its pack and theme, and write the
-  document (`write_document`).
+  document (`write_document`, `write_rendered`).
 - `out.rs`: `Out`, localized command output on stdout; a closed pipe ends the command.
 - `library.rs`: packs and themes found by id, folder or name (`resolve_pack`,
   `embed_theme`, `pack_library`, `theme_library`, `beside_deck`).
@@ -17,6 +17,9 @@ without Tauri. Results go to stdout, logs to stderr, help in the user's language
   worker (in up to `PROBE_ROUNDS` calls); `render` renders in memory or in the worker.
 - `import.rs`: `oku-slides import`, `ImportMode` `retrofit` (onto an existing pack and
   theme) or `hifi`; one transaction, then saved like `save`, with an optional report.
+- `import_overflow.rs`: text fields rendered past their page become `Overflow` losses.
+- `import_reference.rs`: hi-fi `--reference auto`: the witness in the render worker, each Oku
+  page cropped to the source page at 960 px, `metrics::score` per slide into the report.
 - `import_hifi.rs`: hifi preparation: generated pack and theme with an identity from the
   `.pptx` bytes, font catalog, probe check against the declared schemas.
 - `system_fonts.rs`: copies the system faces of the families a presentation asks for.
```

`crates/oku-slides-import/AGENTS.md`: settled decision.

```diff
--- a/crates/oku-slides-import/AGENTS.md
+++ b/crates/oku-slides-import/AGENTS.md
@@ -43,6 +43,7 @@ Read `ARCHITECTURE.md` first for what this code is. This file is how to work on
 
 ## Settled decisions
 
+- Measure the hi-fi visual gap against the reference witness, never against fixtures/import/alto-wealth.pdf. (t-692)
 - Map source colors to target theme tokens or drop them; never emit raw hex colors or token overrides. (spec:2026-10-09-import-pptx-design)
 - Fail only on unreadable package, non-presentation, protection, no master or limit; report every other loss. (spec:2026-10-09-import-pptx-design)
 - Do not flat re-export retrofit from the import lib.rs; re-export only ImportPlan, ImportReport and RetrofitTarget. (plan:2026-10-10-import-pptx-lot2)
```

`crates/oku-slides-import/ARCHITECTURE.md`: import architecture.

```diff
--- a/crates/oku-slides-import/ARCHITECTURE.md
+++ b/crates/oku-slides-import/ARCHITECTURE.md
@@ -10,11 +10,13 @@ Oku ops. Spec: `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.
 - `retrofit::project(&SourceDeck, &RetrofitTarget) -> ImportPlan` : the content laid out on an
   existing pack and theme (templates, registry, `ColorTokens`, `PageTokens`).
 - `hifi::project(&SourceDeck, &HifiTarget) -> ImportPlan` : the source reproduced on a generated
-  theme and pack; `hifi::sought_fonts` lists the families it will look for.
+  theme and pack; `hifi::sought_fonts`, the families it looks for; `hifi::source_rect`, the page.
 - `ImportPlan` (`plan`) : ops, assets, `WidgetRequest`s, report; in hi-fi also the generated
   theme and pack (`PackFiles`), declared schemas and fonts; `report_json` is the `--report` form.
-- `report` : `ImportReport`, `Loss`/`LossKind`, warnings, `Coverage`; `coverage::coverage`
-  measures what of the source survives in the resulting deck.
+- `report` : `ImportReport`, `Loss`/`LossKind`, warnings, `Coverage`, `ReferenceReport`, slide
+  `score`; `coverage::coverage` measures what of the source survives in the resulting deck.
+- `metrics` : a hi-fi slide's visual gap to the reference (mean OKLab ΔE, light blur) and its worst
+  regions (owned pixels, realigned up to `MAX_SHIFT` px, worst `TILE`). Pure: the CLI renders.
 - `fonts` : `FontCatalog`, `FontOrigin`, `FontPlan`, `SUBSTITUTES`, `FALLBACKS`.
 
 ## Layers
```

`crates/oku-slides-reference/AGENTS.md`: new file.

```diff
--- /dev/null
+++ b/crates/oku-slides-reference/AGENTS.md
@@ -0,0 +1,13 @@
+# crates/oku-slides-reference — agent rules
+
+Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.
+
+## Invariants
+
+- Run a reference witness only through RenderWorker::reference in the render worker, never in-process. (t-692, t-327)
+- Give the witness the app's fonts and the deck's embedded fonts, and render hidden slides. (t-692)
+
+## Pitfalls
+
+- Read a gap on titles, autofit text, charts or SmartArt as a possible witness error before changing the import. (t-692)
+- Do not align the import on office2pdf code without checking it against the OOXML or another witness. (t-692)
```

`crates/oku-slides-render-worker/ARCHITECTURE.md`: whole English version (replaces the French text).

```diff
--- a/crates/oku-slides-render-worker/ARCHITECTURE.md
+++ b/crates/oku-slides-render-worker/ARCHITECTURE.md
@@ -1,47 +1,46 @@
 # oku-slides-render-worker
 
-Rendu Typst dans un processus de travail, borné en temps et en mémoire : un
-rendu qui boucle ou qui consomme trop se tue sans emporter l'application.
+Typst rendering in a worker process, bounded in time and memory: a render that loops or uses
+too much memory is killed without taking the app down.
 
-## Types clés
+## Key types
 
-- `RenderWorker` (`client.rs`) : le lien partagé (`Arc<Mutex<Link>>`) ;
-  `is_faulty`, `slot`. Un pack qui fait échouer
-  le rendu (délai, mémoire, arrêt) devient fautif et n'est plus exécuté.
-- `probe_pack` (`pack_probe.rs`) : sonde par fichier, jamais d'accusation (un délai = un
-  diagnostic), gardée par `probe_key`, budget total `timeout_ms` par appel ; `trial_slot` : emplacement d'aperçu qui n'accuse aucun pack.
-- `RenderSlot` (`client.rs`) : un emplacement (`set_widgets`, `set_language`,
-  `render`) ; son `Drop` met `CloseSlot` en file.
-- `RemoteCompiled` (`client.rs`) : une génération ; `layout`, `page_hash` locaux,
-  `svg`/`png`/`pdf` par requête ; son `Drop` met `Release` en file. La file est
-  envoyée avant la requête suivante : un `Drop` ne prend jamais le verrou du lien.
-- `Link` (`link.rs`, interne) : transport (sous-processus ou fil), délai, mémoire
-  relevée toutes les 100 ms (`rss.rs` : `phys_footprint` sur macOS, `footprint.rs`, mémoire résidente ailleurs), relance et rejeu de `ReplayState`. Le fils reçoit
-  ses bornes (`--timeout-ms`, `--memory-bytes`) et `OKU_SLIDES_RENDER_WORKER=1` :
-  `Platform::load` y choisit `InProcess`, un oubli d'`exit_if_worker` ne relance pas en chaîne.
-- `serve` (`serve.rs`) : la boucle du travailleur ; `exit_if_worker` l'appelle
-  quand le binaire est lancé avec `--render-worker`, sous le chien de garde
-  (`watchdog.rs`) : requête > 2 × délai, mémoire > limite pendant 1 s, parent disparu → sortie 3.
-  Les trames partent sur une copie de stdout ; stdout pointe sur stderr (`stdio.rs`, unix).
-- `Request`/`Response` (`protocol.rs`), trames (`frame.rs`) : en-tête JSON et
-  charge brute, bornés des deux côtés (à l'envoi, refus `Frame` sans rien
-  écrire ; à la lecture, avant d'allouer) par `FrameLimits`, tirées de
-  `max_frame_mib` : un en-tête au plus un huitième, la charge d'un SVG un quart,
-  celle d'un PNG, d'un PDF, d'une police ou d'un asset la taille entière, les
-  autres réponses aucune. L'hôte pose les bornes de la réponse attendue avant
-  d'envoyer la requête ; le fil lecteur les lit au premier octet de la trame.
+- `RenderWorker` (`client.rs`): the shared link (`Arc<Mutex<Link>>`); `is_faulty`, `slot`. A
+  pack that makes rendering fail (timeout, memory, crash) becomes faulty and is not run again.
+- `probe_pack` (`pack_probe.rs`): per-file probe, never blaming (a timeout is a diagnostic),
+  keyed by `probe_key`, total budget `timeout_ms` per call; `trial_slot`: a preview slot
+  that blames no pack.
+- `RenderSlot` (`slot.rs`): a slot (`set_widgets`, `set_language`, `render`); its `Drop`
+  queues `CloseSlot`.
+- `RemoteCompiled` (`compiled.rs`): a generation; `layout`, `page_hash` local, `svg`/`png`/`pdf`
+  by request; its `Drop` queues `Release`. The queue is sent before the next request: a
+  `Drop` never takes the link lock.
+- `RemoteReference` (`reference.rs`): `RenderWorker::reference(pptx, fonts)` has the witness
+  (`oku_slides_reference::Office2Pdf`) render the presentation in the worker, under its bounds;
+  `witness`, `page_count`, `page_size` local, `png(page, width)` by request; its `Drop` queues
+  `ReleaseReference`. No pack is blamed; a reference from another epoch answers `Stale`.
+- `Link` (`link.rs`, internal): transport (child process or thread), timeout, memory sampled
+  every 100 ms (`rss.rs`: `phys_footprint` on macOS, `footprint.rs`, resident memory elsewhere),
+  restart and replay of `ReplayState`. The child gets its bounds (`--timeout-ms`,
+  `--memory-bytes`) and `OKU_SLIDES_RENDER_WORKER=1`: `Platform::load` picks `InProcess` there,
+  so a missing `exit_if_worker` does not respawn in a chain.
+- `serve` (`serve.rs`): the worker loop; `exit_if_worker` calls it when the binary runs with
+  `--render-worker`, under the watchdog (`watchdog.rs`): request > 2 × timeout, memory over the
+  limit for 1 s, parent gone → exit 3. Frames go to a copy of stdout; stdout points to stderr.
+- `Request`/`Response` (`protocol.rs`), frames (`frame.rs`): JSON header and raw payload,
+  bounded on both sides by `FrameLimits` from `max_frame_mib`: a header at most an eighth, an
+  SVG a quarter, a PNG (page or reference), PDF, font, asset or `.pptx` the whole size.
 
-## Point d'extension
+## Extension point
 
-Une nouvelle sortie d'une page : une requête dans `protocol.rs`, sa branche
-dans `serve.rs`, une méthode sur `RemoteCompiled`.
+A new output of a page: a request in `protocol.rs`, its branch in `serve.rs`, a method on
+`RemoteCompiled`.
 
-## Dépendances
+## Dependencies
 
-- `oku-slides-render` : `Renderer`, erreurs sérialisables, sonde du registre.
-- `oku-slides-core` (`Deck`, `WidgetRegistry`), `oku-slides-i18n` (langue).
-- `oku-slides-platform` : `RenderLaunch`, réglages `[render]`, journaux.
-- `serde`, `serde_json` (`float_roundtrip` : la mise en page au bit près) ;
-  `sha2`, `hex` : empreintes des polices et des packs.
-- `snafu` : erreurs ; `tracing` : journaux ; `sysinfo` : mémoire résidente du travailleur (hors macOS) ;
-  `libc` (unix) : `dup2` de stdout, `proc_pid_rusage` (macOS).
+- `oku-slides-render`: `Renderer`, serializable errors, registry probe; `oku-slides-reference`:
+  the witness and the rasterization of its pages.
+- `oku-slides-core` (`Deck`, `WidgetRegistry`), `oku-slides-i18n`, `oku-slides-platform`
+  (`RenderLaunch`, `[render]` settings, logs).
+- `serde`, `serde_json` (`float_roundtrip`), `sha2`, `hex`, `snafu`, `tracing`, `sysinfo`,
+  `libc` (unix: `dup2` of stdout, `proc_pid_rusage`).
```

`docs/superpowers/specs/2026-10-09-import-pptx-design.md`: spec.

```diff
--- a/docs/superpowers/specs/2026-10-09-import-pptx-design.md
+++ b/docs/superpowers/specs/2026-10-09-import-pptx-design.md
@@ -71,8 +71,8 @@ Decisions from the lot 2 plan (2026-10-10):
   lookup, substitution) arrives with hi-fi.
 - **Content coverage in lot 2.** `coverage.rs` measures, on the resulting
   deck, the recall of runs, the presence of images and the equality of
-  table and chart data; lot 4 keeps the LibreOffice
-  reference and the visual gap (`metrics.rs`).
+  table and chart data; lot 4 keeps the reference
+  and the visual gap (`metrics.rs`).
 
 Decisions from the lot 3 plan (2026-10-11):
 
@@ -155,6 +155,43 @@ Clarifications made along the way:
 - `component.list.marker` takes the master's body bullet;
   `marker-color` is nullable (`null`: the text color).
 
+Decisions from the lot 4 plan (2026-10-14):
+
+1. **Witness.** No embeddable PPTX renderer is a faithful reference, and the
+   user ruled out a local LibreOffice install: the reference comes from the
+   `office2pdf` crate (pure Rust on Typst 0.15, Apache-2.0), behind a
+   `ReferenceRenderer` trait in a new crate `oku-slides-reference` (PDF
+   rasterized by `hayro`). It gets the app's fonts and the deck's embedded
+   fonts first, and renders hidden slides. LibreOffice and PowerPoint
+   witnesses are backlog work, as other implementations of the trait.
+2. **Isolation.** The witness reads an untrusted file without bounds of its
+   own: it runs in the render worker (`Reference`, `ReferencePng`,
+   `ReleaseReference` requests; `RenderWorker::reference` →
+   `RemoteReference`), under `timeout_ms` and `memory_mib`. A missing,
+   failing or mismatched reference (`ReferenceCause`: `failed`, `timeout`,
+   `memory`, `pages`) is reported, never an error.
+3. **Metric** (`oku-slides-import/src/metrics.rs`, pure, calibrated on alto).
+   Slide score: mean OKLab ΔE ×100 over the source page at 960 px after a
+   σ = 1 px Gaussian blur. Region score: each pixel belongs to the topmost
+   item covering it; the region is realigned by the integer shift (at most
+   3 px per axis) that minimizes its mean gap, the shift is reported; its
+   score is the mean gap of its worst 16 px tile (tiles overlap by half);
+   below 100 own pixels, no score. The report keeps the 3 worst regions.
+4. **Threshold.** `[import] score_threshold = 3.0`: the fixtures stay at or
+   under 2.8. Tests run the witness live (pinned by the lockfile, no
+   versioned reference PDF) with per-fixture bounds (measurement + 25 %) and
+   designation checks on alto (p. 7 callout line break, p. 8 bullet indents)
+   and on the `libreoffice.pptx` table.
+5. **Retrofit.** No visual gap (another theme); `--reference` is refused there.
+   In both modes, a non-empty text field rendered past its page edge becomes an
+   `Overflow` loss on its slide (t-816 stays open for the fix).
+6. **Known witness limits.** It shrinks `normAutofit` text without
+   `fontScale`, draws charts roughly and SmartArt text too small, renders the
+   alto titles about 1 % wider than Oku and LibreOffice, and shares Typst's
+   line breaking with Oku (a PowerPoint line-break gap, t-863, is invisible
+   to it). A behavior copied from office2pdf into the import is no longer
+   measured by it.
+
 Defaults chosen without asking:
 
 - **Page size.** Hi-fi: 1920 × 1080, the source scaled and
@@ -205,13 +242,13 @@ Crate `crates/oku-slides-import`:
 | `retrofit/` | retrofit projector: template scoring, grid, colors, fallbacks |
 | `pattern.rs` | pattern signature (shared by retrofit / hi-fi grouping) |
 | `plan.rs` | `ImportPlan`, `WidgetRequest` |
-| `reference.rs` | `soffice` call, `hayro` rasterization |
 | `coverage.rs` | content coverage, measured on the resulting deck (lot 2) |
-| `metrics.rs` | visual gap (hi-fi, lot 4) |
+| `metrics.rs` | visual gap and worst regions (hi-fi, lot 4; images rendered by the CLI) |
 | `report.rs` | `ImportReport` |
 
-Dependencies: `zip` (existing), `roxmltree` (new, read-only DOM),
-`hayro` (already transitive, becomes direct for `reference.rs`). The crate
+Dependencies: `zip` (existing), `roxmltree` (new, read-only DOM). The
+reference witness and the rasterization live in `oku-slides-reference`
+(`office2pdf`, `hayro`), run by the render worker (lot 4). The crate
 depends neither on the session nor on the UI: it takes as input the widget
 registry, the pack's templates (`oku_slides_core::Template`) and the target
 theme's tokens, and returns a plan.
@@ -368,13 +405,13 @@ it does not see it; only the reference rendering (lot 4) will measure it.
 
 ## Reference rendering and metrics
 
-- **Reference** (`reference.rs`): `soffice --headless --convert-to pdf`,
-  path and timeout in config; PDF rasterized by `hayro`. Absent or
-  failing → warning in the report, no error.
-- **Hi-fi — visual gap**: Oku rendering (`Compiled::png`) and reference at the
-  same resolution (960 px wide), mean OKLab ΔE after a light blur; plus
-  a **per-region score** from the items' `frame`, to point at the
-  faulty element.
+- **Reference** (`oku-slides-reference`, lot 4 decisions 1 and 2): the
+  `office2pdf` witness in the render worker, PDF rasterized by `hayro`.
+  Absent or failing → reported, no error.
+- **Hi-fi — visual gap**: Oku rendering cropped to the source page and
+  reference at the same resolution (960 px wide), mean OKLab ΔE after a
+  light blur; plus a **per-region score** from the items' `frame`, to point
+  at the faulty element (lot 4 decision 3).
 - **Retrofit — content coverage**: text recall (each source run
   found), presence of images, equality of table and chart
   data. Target 100%; every gap is listed.
@@ -383,7 +420,8 @@ it does not see it; only the reference rendering (lot 4) will measure it.
 
 `ImportReport` (JSON): per slide, score (gap or coverage), fallbacks
 applied, `Opaque`, ignored effects; globally, substituted fonts
-(`fonts`: one decision per family, lot 3), reference available or not,
+(`fonts`: one decision per family, lot 3), reference (witness, threshold,
+cause when missing; `above_threshold` on each slide beyond it, lot 4),
 `widget_requests`. Messages through Fluent
 (`oku-slides-i18n`).
 
@@ -415,7 +453,9 @@ applied, `Opaque`, ignored effects; globally, substituted fonts
   `--pack` and `--theme` are rejected there (the pack and the theme are generated,
   embedded in the deck only); the presentation's fonts are
   looked up among the app's (`--fonts`) and, unless `--no-system-fonts`,
-  the system's. `--reference` arrives in lot 4.
+  the system's. `--reference auto|none` (lot 4): hi-fi only, `auto` by
+  default; a second output line gives the mean gap and the slides above the
+  threshold.
 - **App**: "Ouvrir" (Open) accepts `.pptx`; a dialog asks for the mode, and in
   retrofit the target pack and theme. The app creates a new deck from the
   plan, then launches the agent with the fine-tuning instructions.
@@ -447,11 +487,13 @@ entry in the report.
   `python-pptx` is not available and hand-written XML files
   read better.)*
 - **Real deck**: `fixtures/import/alto-wealth.pptx` (generated by PptxGenJS)
-  and its reference PDF `alto-wealth.pdf`, visual target of lots 4 and 6.
+  and `alto-wealth.pdf`, an HTML render without embedded fonts (text ~5 %
+  narrower), visual target of lot 6; lot 4 measures against the witness.
 - **`insta` snapshots** of `SourceDeck` and `ImportPlan` per fixture.
 - **Style cascade** table-tested (`rstest`).
-- **Hi-fi gold**: **versioned** reference PDFs (no LibreOffice in
-  CI); visual gap under the threshold.
+- **Hi-fi gold**: the witness runs live in CLI tests (no LibreOffice, no
+  versioned reference PDF); visual gap under a per-fixture bound, worst
+  regions pointing at known gaps (lot 4 decision 4).
 - **Retrofit**: 100% content coverage on the fixtures.
 - **CLI**: end-to-end integration test (pptx → reopenable
   `.oku-slides.pdf`).
@@ -468,7 +510,9 @@ One plan and its subtasks per lot, in this order:
 3. *(delivered, t-692)* Hi-fi projector: theme, Typst generator, pack,
    fonts (`fonts/`), extended base (paths, text anchoring); plan:
    `docs/superpowers/plans/2026-10-11-import-pptx-lot3.md`.
-4. Reference rendering + visual gap.
+4. *(delivered, t-692)* Reference rendering (`office2pdf` witness in the
+   render worker) + visual gap + overflow losses; plan:
+   `docs/superpowers/plans/2026-10-14-import-pptx-lot4.md`.
 5. App + agent fine-tuning (MCP tools, instructions, child pack of the
    widgets created for the `WidgetRequest`s).
 6. *(Phase 2, separate spec)* PDF reader → `SourceDeck` through
```

`fixtures/import/README.md`: English fixtures README.

```diff
--- a/fixtures/import/README.md
+++ b/fixtures/import/README.md
@@ -1,9 +1,11 @@
-# Fixtures d'import (t-692)
+# Import fixtures (t-692)
 
-- `minimal/`, `kitchen/` : OOXML décompressé écrit à la main, zippé à la
-  volée par `crates/oku-slides-import/tests/common`.
-- `libreoffice.pptx` : `kitchen` réécrit par LibreOffice (aller-retour).
-- `alto-wealth.pptx` : deck d'exemple fourni par Matthieu le 2026-10-09,
-  généré par PptxGenJS, versionné avec son accord pour les tests.
-  `alto-wealth.pdf` (LFS) : son rendu de référence, produit séparément
-  (« appifact kit ») ; cible visuelle des lots 4 et 6, lu par aucun test du lot 1.
+- `minimal/`, `kitchen/`: hand-written unzipped OOXML, zipped on the fly by
+  `crates/oku-slides-import/tests/common`.
+- `libreoffice.pptx`: `kitchen` rewritten by LibreOffice (round trip).
+- `alto-wealth.pptx`: sample deck provided by Matthieu on 2026-10-09,
+  generated by PptxGenJS, versioned with their consent for the tests.
+  `alto-wealth.pdf` (LFS): an HTML render of it, made separately
+  ("appifact kit"), without embedded fonts (text ~5 % narrower); visual target
+  of lot 6, read by no test. Lot 4 measures the hi-fi visual gap against the
+  `office2pdf` witness instead (`crates/oku-slides-cli/tests/import_reference.rs`).
```

- [ ] **Step 2: Run the lint and the line checks**

Run: `just agents-lint`
Expected: `✓ harnais AGENTS.md`

Run: `wc -l crates/oku-slides-import/ARCHITECTURE.md crates/oku-slides-cli/ARCHITECTURE.md crates/oku-slides-render-worker/ARCHITECTURE.md crates/oku-slides-reference/ARCHITECTURE.md`
Expected: each file at most 50 lines (no `total` line counts).

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md README.md crates/oku-slides-cli/AGENTS.md crates/oku-slides-cli/ARCHITECTURE.md crates/oku-slides-import/AGENTS.md crates/oku-slides-import/ARCHITECTURE.md crates/oku-slides-reference/AGENTS.md crates/oku-slides-render-worker/ARCHITECTURE.md docs/superpowers/specs/2026-10-09-import-pptx-design.md fixtures/import/README.md
```

Write this message to a file (for example `/tmp/commit-t7.txt`), then commit it:

```
docs(import) : lot 4 — décisions du plan, témoin office2pdf, ARCHITECTURE.md en anglais, règles AGENTS.md (t-692)
```

```bash
git commit -F /tmp/commit-t7.txt
```

**Extracting from the draft:** the implementer may take these files from the draft commit `f86a6c84` instead of retyping them: `git show f86a6c84:<file> > <file>` for each file listed under Files, never a directory, but only if the file did not change on `main` after the plan was committed; otherwise apply the diff of that file with `git apply --3way` (for example `git diff main f86a6c84 -- <file> | git apply --3way`). On `main` at plan time (`9bcc3b6c`), these files differ from the draft base `53454df2` (`git diff --name-only 53454df2 main`): `AGENTS.md`, `README.md`, `crates/oku-slides-cli/AGENTS.md`, `crates/oku-slides-cli/ARCHITECTURE.md`, `crates/oku-slides-import/AGENTS.md`, `crates/oku-slides-import/ARCHITECTURE.md`, `docs/superpowers/specs/2026-10-09-import-pptx-design.md`. The diffs above are against `main`, so they apply on top of those changes.

### Task 8: Review fixes, import: no panic on a negative frame, σ = 2, slides flagged by their worst region, bounded regions

A `<a:ext cx>` below zero, kept by the reader, made `metrics::bounds` return `x1 < x0` and `ownership` panic on a reversed slice; a negative frame now gives an empty rectangle and no score. The blur goes to σ = 2 (at σ = 1, alto titles aligned within 1 px ranked above real gaps). The slide mean does not separate a faithful slide from a damaged one (the broken table of `libreoffice.pptx` p. 1 scores 0.6), so a slide is now flagged by its worst region (`SlideScore::worst`, `SlideReport::is_above`, the comparison written once). At most `MAX_REGIONS` = 256 regions (the topmost) are scored, which bounds the host CPU; the tile accumulator is a dense grid instead of a `BTreeMap`. `ReferenceReport::is_available` is removed (dead code), and tests the review found weak are fixed.

**Files:**
- Create: `crates/oku-slides-import/tests/regressions/issue_692_negative_extent.rs` (regression: negative width, no panic)
- Modify: `crates/oku-slides-import/tests/regressions/main.rs` (declare the new module)
- Modify: `crates/oku-slides-import/src/metrics.rs` (empty rectangle for a negative frame, `BLUR_SIGMA` = 2.0, `MAX_REGIONS`, `SlideScore::worst`, dense tile grid)
- Modify: `crates/oku-slides-import/src/report.rs` (`SlideReport::is_above`, `above_threshold` and `to_json` use it, `Overflow` doc)
- Modify: `crates/oku-slides-import/src/report/reference.rs` (remove `is_available`, threshold doc)
- Test: `crates/oku-slides-import/src/metrics_tests.rs`, `crates/oku-slides-import/src/report_tests.rs`, `crates/oku-slides-import/src/report/reference_tests.rs`, `crates/oku-slides-import/tests/regressions/issue_692_negative_extent.rs`

**Interfaces:**
- Consumes: from Tasks 3–5, `metrics::{Image, Region, RegionScore, SlideScore, regions, score, REPORTED_REGIONS}`, `report::{SlideReport, ImportReport::above_threshold, ReferenceReport, ReferenceCause::localize}`, and the test helpers already in those test files (`plain`, `with_rect`, `region`, `image`, `items`, `scored`, `with_reference`, `english`).
- Produces: `pub const MAX_REGIONS: usize = 256` and `pub const BLUR_SIGMA: f64 = 2.0` in `oku_slides_import::metrics`; `SlideScore::worst(&self) -> Option<f64>`; `SlideReport::is_above(&self, threshold: f64) -> bool`. Removed: `ReferenceReport::is_available`. `ARCHITECTURE.md` is not touched by this task.

- [ ] **Step 1: Write the failing tests**

```diff
--- /dev/null
+++ b/crates/oku-slides-import/tests/regressions/issue_692_negative_extent.rs
@@ -0,0 +1,22 @@
+//! Régression (t-692, lot 4) : un `<a:ext cx>` négatif, gardé par le lecteur, donnait à
+//! `metrics::score` un rectangle à l'envers et faisait paniquer l'import hi-fi (tranche
+//! `owner[x0..x1]` avec `x1 < x0`).
+
+use oku_slides_import::metrics::{Image, regions, score};
+use oku_slides_import::read_pptx;
+use speculoos::prelude::*;
+
+use crate::common;
+
+#[test]
+fn score_of_a_shape_with_a_negative_width_skips_it_without_panicking() {
+    let slide = common::text("minimal/ppt/slides/slide3.xml").replace(r#"cx="3048000""#, r#"cx="-3048000""#);
+    let deck = read_pptx(&common::pptx_with("minimal", &[("ppt/slides/slide3.xml", Some(&slide))]), &common::limits())
+        .unwrap_or_else(|e| panic!("lecture : {e}"));
+    let page = deck.slides.iter().find(|s| s.number == 3).unwrap_or_else(|| panic!("pas de slide 3"));
+    assert_that!(page.items.iter().any(|i| i.frame.w < 0.0)).is_true();
+    let white: Vec<u8> = (0..960 * 540).flat_map(|_| [255, 255, 255, 255]).collect();
+    let img = Image { width: 960, height: 540, rgba: &white };
+    let s = score(img, img, &regions(page, 960.0 / deck.size.w)).unwrap_or_else(|| panic!("pas de score"));
+    assert_that!(s.regions.iter().all(|r| r.score == 0.0)).is_true();
+}
```

```diff
--- a/crates/oku-slides-import/tests/regressions/main.rs
+++ b/crates/oku-slides-import/tests/regressions/main.rs
@@ -6,5 +6,6 @@ mod issue_692_bomb_rereads;
 mod issue_692_invisible_chars;
 mod issue_692_layout_dedup;
 mod issue_692_merged_cell_edges;
+mod issue_692_negative_extent;
 mod issue_692_xml_depth;
 mod issue_692_xml_nodes_limit;
```

```diff
--- a/crates/oku-slides-import/src/metrics_tests.rs
+++ b/crates/oku-slides-import/src/metrics_tests.rs
@@ -50,9 +50,10 @@ fn score_of_white_against_black_is_one_hundred() {
 fn score_designates_the_region_that_differs() {
     let white = plain(100, 50, [255, 255, 255]);
     let marked = with_rect(white.clone(), 100, (60, 20, 12, 12), [255, 0, 0]);
-    let regions = [region("gauche", (0.0, 0.0, 50.0, 50.0)), region("droite", (50.0, 0.0, 50.0, 50.0))];
+    // Les noms vont à rebours de l'ordre alphabétique, qui départage les égalités.
+    let regions = [region("a-gauche", (0.0, 0.0, 50.0, 50.0)), region("z-droite", (50.0, 0.0, 50.0, 50.0))];
     let s = score(image(100, 50, &white), image(100, 50, &marked), &regions).unwrap();
-    assert_that!(items(&s)).is_equal_to(vec!["droite", "gauche"]);
+    assert_that!(items(&s)).is_equal_to(vec!["z-droite", "a-gauche"]);
 }
 
 #[test]
@@ -102,11 +103,43 @@ fn score_skips_a_region_with_too_few_pixels_of_its_own() {
 
 #[test]
 fn score_keeps_the_worst_regions_only() {
-    let white = plain(100, 20, [255, 255, 255]);
+    let white = plain(200, 40, [255, 255, 255]);
+    // r1, r3 et r4 portent un carré noir, de plus en plus grand ; r0 et r2 sont identiques.
+    let mut marked = white.clone();
+    for (i, side) in [(1, 8), (3, 12), (4, 16)] {
+        marked = with_rect(marked, 200, (i * 40 + 4, 4, side, side), [0, 0, 0]);
+    }
     let regions: Vec<Region> =
-        (0..5).map(|i| region(&format!("r{i}"), (f64::from(i) * 20.0, 0.0, 20.0, 20.0))).collect();
-    let s = score(image(100, 20, &white), image(100, 20, &white), &regions).unwrap();
-    assert_that!(s.regions.len()).is_equal_to(REPORTED_REGIONS);
+        (0..5).map(|i| region(&format!("r{i}"), (f64::from(i) * 40.0, 0.0, 40.0, 40.0))).collect();
+    let s = score(image(200, 40, &white), image(200, 40, &marked), &regions).unwrap();
+    assert_that!(items(&s)).is_equal_to(vec!["r4", "r3", "r1"]);
+}
+
+#[test]
+fn score_of_a_region_with_a_negative_width_is_skipped() {
+    let white = plain(40, 20, [255, 255, 255]);
+    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("envers", (30.0, 0.0, -20.0, 20.0))]).unwrap();
+    assert_that!(s.regions).is_empty();
+}
+
+#[test]
+fn score_counts_the_topmost_max_regions_only() {
+    let white = plain(64, 64, [255, 255, 255]);
+    let marked = with_rect(white.clone(), 64, (0, 0, 16, 16), [0, 0, 0]);
+    // La région du dessous, seule à différer, passe sous la borne : aucune ne la porte.
+    let mut regions = vec![region("dessous", (0.0, 0.0, 16.0, 16.0))];
+    regions.extend((0..MAX_REGIONS).map(|i| region(&format!("r{i}"), (32.0, 32.0, 32.0, 32.0))));
+    let s = score(image(64, 64, &white), image(64, 64, &marked), &regions).unwrap();
+    assert_that!(s.regions.iter().all(|r| r.item != "dessous" && r.score == 0.0)).is_true();
+}
+
+#[test]
+fn worst_is_the_score_of_the_first_region() {
+    let white = plain(100, 50, [255, 255, 255]);
+    let marked = with_rect(white.clone(), 100, (60, 20, 12, 12), [255, 0, 0]);
+    let regions = [region("a", (0.0, 0.0, 50.0, 50.0)), region("b", (50.0, 0.0, 50.0, 50.0))];
+    let s = score(image(100, 50, &white), image(100, 50, &marked), &regions).unwrap();
+    assert_that!(s.worst()).is_equal_to(Some(s.regions[0].score));
 }
 
 #[test]
```

```diff
--- a/crates/oku-slides-import/src/report_tests.rs
+++ b/crates/oku-slides-import/src/report_tests.rs
@@ -381,8 +381,9 @@ fn to_json_without_score_writes_no_score() {
 
 #[test]
 fn to_json_explains_a_missing_reference() {
-    let v = with_reference(vec![], Some(ReferenceCause::Timeout)).to_json(english());
-    assert_that!(v["reference"]["message"].as_str().unwrap()).contains("too long");
+    let expected = ReferenceCause::Timeout.localize(&Localizer::new(Some("fr")));
+    let v = with_reference(vec![], Some(ReferenceCause::Timeout)).to_json(&Localizer::new(Some("fr")));
+    assert_that!(v["reference"]["message"].as_str()).is_equal_to(Some(expected.as_str()));
 }
 
 #[test]
@@ -391,6 +392,14 @@ fn to_json_without_reference_writes_none() {
     assert_that!(v.get("reference")).is_none();
 }
 
+#[test]
+fn above_threshold_follows_the_worst_region_not_the_mean() {
+    let regions = vec![crate::metrics::RegionScore { item: "s1/1".into(), score: 9.0, shift: [0, 0] }];
+    let slide = SlideReport { score: Some(SlideScore { score: 1.0, regions }), ..scored(1, 1.0) };
+    let r = with_reference(vec![slide], None);
+    assert_that!(r.above_threshold().count()).is_equal_to(1);
+}
+
 #[test]
 fn above_threshold_lists_the_slides_beyond_it() {
     let r = with_reference(vec![scored(1, 2.0), scored(2, 7.5), scored(3, 5.0)], None);
```

```diff
--- a/crates/oku-slides-import/src/report/reference_tests.rs
+++ b/crates/oku-slides-import/src/report/reference_tests.rs
@@ -17,13 +17,21 @@ fn cause_localize_in_every_locale_formats_its_own_message(#[case] cause: Referen
 }
 
 #[test]
-fn cause_pages_gives_both_counts() {
+fn cause_pages_gives_both_counts_in_order() {
     let msg = ReferenceCause::Pages { reference: 7, deck: 8 }.localize(english());
-    assert_that!((msg.contains('7'), msg.contains('8'))).is_equal_to((true, true));
+    let (seven, eight) = (msg.find('7'), msg.find('8'));
+    assert_that!(seven.zip(eight).is_some_and(|(s, e)| s < e)).is_true();
 }
 
 #[test]
-fn reference_without_cause_is_available() {
-    let r = ReferenceReport { witness: "office2pdf 0.8.1".into(), threshold: 5.0, unavailable: None };
-    assert_that!(r.is_available()).is_true();
+fn causes_give_four_distinct_messages_in_french() {
+    let fr = Localizer::new(Some("fr"));
+    let causes = [
+        ReferenceCause::Failed,
+        ReferenceCause::Timeout,
+        ReferenceCause::Memory,
+        ReferenceCause::Pages { reference: 7, deck: 8 },
+    ];
+    let messages: std::collections::BTreeSet<String> = causes.iter().map(|c| c.localize(&fr)).collect();
+    assert_that!(messages.len()).is_equal_to(4);
 }
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import`
Expected: compile error in the unit tests (`MAX_REGIONS` and `SlideScore::worst` not found); once those are stubbed, `score_of_a_shape_with_a_negative_width_skips_it_without_panicking` panics on the reversed slice, `score_of_a_region_with_a_negative_width_is_skipped` panics, and `above_threshold_follows_the_worst_region_not_the_mean` fails.

- [ ] **Step 3: Implement**

`metrics.rs`: an empty rectangle for a negative frame, σ = 2, the `MAX_REGIONS` bound (topmost regions), `SlideScore::worst`, and a dense tile grid sized from the region frame.

```diff
--- a/crates/oku-slides-import/src/metrics.rs
+++ b/crates/oku-slides-import/src/metrics.rs
@@ -3,7 +3,9 @@
 //! métriques »).
 //!
 //! Les deux images passent en OKLab et sont adoucies par un flou gaussien
-//! ([`BLUR_SIGMA`] pixels : l'anticrénelage compte peu) ; l'écart d'un pixel
+//! ([`BLUR_SIGMA`] pixels : l'anticrénelage et le pixel de décalage d'un
+//! glyphe comptent peu ; à σ = 1, les titres d'alto, alignés à 1 px près,
+//! passaient devant les vrais écarts) ; l'écart d'un pixel
 //! est le ΔE OKLab (×100, celui de `retrofit::color`). Le score d'une slide est
 //! l'écart moyen sur la page.
 //!
@@ -18,12 +20,19 @@
 //! - son score est l'écart moyen de sa pire tuile de [`TILE`] pixels : un mot
 //!   déplacé dans un grand cadre presque vide compte pleinement ;
 //! - une région de moins de [`MIN_PIXELS`] pixels à elle n'a pas de score
-//!   (ses pixels sont surtout des bords anticrénelés).
+//!   (ses pixels sont surtout des bords anticrénelés) ; un cadre de taille
+//!   négative (le lecteur la garde) n'en a aucun ;
+//! - au plus [`MAX_REGIONS`] régions comptent, les plus hautes : le coût reste
+//!   borné quel que soit le nombre d'items d'une slide.
+//!
+//! Une slide est signalée par sa pire région ([`SlideScore::worst`]) : la
+//! moyenne de la page, dominée par les fonds identiques, ne distingue pas une
+//! slide fidèle d'une slide abîmée.
 //!
 //! Le rapport garde les [`REPORTED_REGIONS`] pires. Pur : les images viennent
 //! de l'appelant.
 
-use std::collections::{BTreeMap, HashMap};
+use std::collections::HashMap;
 
 use serde::Serialize;
 
@@ -31,7 +40,7 @@ use crate::retrofit::color::{Oklab, delta_e, oklab};
 use crate::source::{Item, ItemKind, Rgba, SourceSlide};
 
 /// Écart-type du flou gaussien, en pixels (à 960 px de large).
-pub const BLUR_SIGMA: f64 = 1.0;
+pub const BLUR_SIGMA: f64 = 2.0;
 
 /// Décalage maximal, en pixels et par axe, du recalage d'une région.
 pub const MAX_SHIFT: i64 = 3;
@@ -45,6 +54,9 @@ pub const TILE: usize = 16;
 /// Pixels propres en deçà desquels une région n'a pas de score.
 pub const MIN_PIXELS: usize = 100;
 
+/// Régions comptées par slide, au plus : les dernières dessinées, au-dessus.
+pub const MAX_REGIONS: usize = 256;
+
 /// Nombre de régions gardées par slide, les pires d'abord.
 pub const REPORTED_REGIONS: usize = 3;
 
@@ -95,9 +107,17 @@ pub struct SlideScore {
     pub regions: Vec<RegionScore>,
 }
 
-/// Les régions de `slide` : le cadre de chaque item, les groupes ouverts
-/// (leurs enfants seulement), à `scale` pixels par pt de la page source, dans
-/// l'ordre de dessin (la dernière est au-dessus).
+impl SlideScore {
+    /// Le score de sa pire région ; `None` sans région notée.
+    pub fn worst(&self) -> Option<f64> {
+        self.regions.first().map(|r| r.score)
+    }
+}
+
+/// Le cadre de chaque item de `slide`, groupes ouverts, à `scale` px par pt.
+///
+/// Seuls les enfants d'un groupe comptent ; l'ordre est celui du dessin (la
+/// dernière région est au-dessus).
 pub fn regions(slide: &SourceSlide, scale: f64) -> Vec<Region> {
     fn walk(items: &[Item], scale: f64, out: &mut Vec<Region>) {
         for item in items {
@@ -121,8 +141,10 @@ pub fn regions(slide: &SourceSlide, scale: f64) -> Vec<Region> {
     out
 }
 
-/// L'écart de `oku` à `reference` ; `None` si les images n'ont pas la même
-/// taille, sont vides ou que leurs octets ne la font pas.
+/// L'écart de `oku` à `reference`, et ses pires régions.
+///
+/// `None` si les images n'ont pas la même taille, sont vides ou que leurs
+/// octets ne la font pas.
 pub fn score(reference: Image<'_>, oku: Image<'_>, regions: &[Region]) -> Option<SlideScore> {
     let (w, h) = (reference.width as usize, reference.height as usize);
     let valid = |i: Image<'_>| i.rgba.len() == w * h * 4;
@@ -132,11 +154,13 @@ pub fn score(reference: Image<'_>, oku: Image<'_>, regions: &[Region]) -> Option
     }
     let pair = Pair { a: blurred(reference), b: blurred(oku), w, h };
     let mean = (0..w * h).map(|i| delta_e(pair.a[i], pair.b[i])).sum::<f64>() / (w * h) as f64;
+    let regions = &regions[regions.len().saturating_sub(MAX_REGIONS)..];
     let mut scored: Vec<RegionScore> = ownership(regions, w, h)
         .iter()
         .zip(regions)
         .filter_map(|(pixels, r)| {
-            pair.region(pixels).map(|(score, shift)| RegionScore { item: r.item.clone(), score: round2(score), shift })
+            let (score, shift) = pair.region(pixels, bounds(r, w, h))?;
+            Some(RegionScore { item: r.item.clone(), score: round2(score), shift })
         })
         .collect();
     scored.sort_by(|x, y| y.score.total_cmp(&x.score).then_with(|| x.item.cmp(&y.item)));
@@ -164,9 +188,13 @@ fn ownership(regions: &[Region], w: usize, h: usize) -> Vec<Vec<usize>> {
 }
 
 /// Le rectangle `r` en pixels entiers, borné à l'image : `x0, y0, x1, y1`.
+///
+/// Un cadre de taille négative ou non finie est vide (`x1 = x0` ou `y1 = y0`).
 fn bounds(r: &Region, w: usize, h: usize) -> (usize, usize, usize, usize) {
     let clamp = |v: f64, max: usize| (v.max(0.0) as usize).min(max);
-    (clamp(r.x.floor(), w), clamp(r.y.floor(), h), clamp((r.x + r.w).ceil(), w), clamp((r.y + r.h).ceil(), h))
+    let (x0, y0) = (clamp(r.x.floor(), w), clamp(r.y.floor(), h));
+    let (x1, y1) = (clamp((r.x + r.w).ceil(), w), clamp((r.y + r.h).ceil(), h));
+    (x0, y0, x1.max(x0), y1.max(y0))
 }
 
 /// La référence et le rendu Oku en OKLab adouci, de même taille.
@@ -186,9 +214,9 @@ impl Pair {
         delta_e(self.a[p], self.b[sy * self.w + sx])
     }
 
-    /// Le score d'une région de pixels propres `pixels` et son recalage ;
-    /// `None` sous [`MIN_PIXELS`].
-    fn region(&self, pixels: &[usize]) -> Option<(f64, [i64; 2])> {
+    /// Le score d'une région de pixels propres `pixels`, dans le cadre
+    /// `x0, y0, x1, y1`, et son recalage ; `None` sous [`MIN_PIXELS`].
+    fn region(&self, pixels: &[usize], (x0, y0, x1, y1): (usize, usize, usize, usize)) -> Option<(f64, [i64; 2])> {
         if pixels.len() < MIN_PIXELS {
             return None;
         }
@@ -206,14 +234,16 @@ impl Pair {
             .map_or((0, 0), |(_, s)| s);
         // Tuiles de TILE pixels au pas de TILE / 2 : un écart de la taille d'une
         // tuile tombe entier dans l'une d'elles, où qu'il soit.
+        // Grille dense sur le cadre de la région : une case par tuile.
         let half = TILE / 2;
-        let mut tiles: BTreeMap<(usize, usize), (f64, usize)> = BTreeMap::new();
+        let cols = (x1 - x0) / half + 1;
+        let mut tiles = vec![(0.0, 0usize); cols * ((y1 - y0) / half + 1)];
         for &p in pixels {
-            let (x, y) = (p % self.w / half, p / self.w / half);
+            let (x, y) = ((p % self.w - x0) / half, (p / self.w - y0) / half);
             let d = self.at(p, dx, dy);
             for tx in x.saturating_sub(1)..=x {
                 for ty in y.saturating_sub(1)..=y {
-                    let t = tiles.entry((tx, ty)).or_default();
+                    let t = &mut tiles[ty * cols + tx];
                     t.0 += d;
                     t.1 += 1;
                 }
@@ -224,7 +254,7 @@ impl Pair {
         let full = |n: usize| n * 4 >= TILE * TILE;
         let mean = || pixels.iter().map(|&p| self.at(p, dx, dy)).sum::<f64>() / pixels.len() as f64;
         let peak =
-            tiles.values().filter(|(_, n)| full(*n)).map(|(s, n)| s / *n as f64).reduce(f64::max).unwrap_or_else(mean);
+            tiles.iter().filter(|(_, n)| full(*n)).map(|(s, n)| s / *n as f64).reduce(f64::max).unwrap_or_else(mean);
         Some((peak, [dx, dy]))
     }
 }
```

`report.rs`: one comparison, `SlideReport::is_above`, used by `above_threshold` and `to_json`.

```diff
--- a/crates/oku-slides-import/src/report.rs
+++ b/crates/oku-slides-import/src/report.rs
@@ -159,10 +159,10 @@ pub enum LossKind {
     /// largeurs de colonnes ou tailles de texte que le widget `table` ne rend
     /// pas, ou un style autre que celui du thème généré (le premier tableau).
     TableStyle,
-    /// Élément ou champ rendu au-delà du bord de sa slide (relevé sur le rendu,
+    /// Champ de texte rendu au-delà du bord de sa slide (relevé sur le rendu,
     /// dans les deux modes) : un contenu plus haut que sa place (t-816).
     Overflow {
-        /// Id de l'élément Oku, ou `<élément>/<champ>` pour un champ.
+        /// Le champ : `<id de l'élément ou de la slide Oku>/<paramètre>`.
         element: String,
     },
 }
@@ -347,6 +347,13 @@ pub struct SlideReport {
     pub score: Option<SlideScore>,
 }
 
+impl SlideReport {
+    /// Sa pire région dépasse-t-elle `threshold` ?
+    pub fn is_above(&self, threshold: f64) -> bool {
+        self.score.as_ref().and_then(SlideScore::worst).is_some_and(|w| w > threshold)
+    }
+}
+
 /// Une proportion : `found` sur `total`.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
 pub struct Ratio {
@@ -442,10 +449,10 @@ impl ImportReport {
         self.slides.iter().map(|s| s.losses.len()).sum()
     }
 
-    /// Les slides dont l'écart visuel dépasse le seuil de la référence.
+    /// Les slides dont la pire région dépasse le seuil de la référence.
     pub fn above_threshold(&self) -> impl Iterator<Item = &SlideReport> {
         let threshold = self.reference.as_ref().map(|r| r.threshold);
-        self.slides.iter().filter(move |s| s.score.as_ref().zip(threshold).is_some_and(|(sc, t)| sc.score > t))
+        self.slides.iter().filter(move |s| threshold.is_some_and(|t| s.is_above(t)))
     }
 
     /// L'écart visuel moyen des slides qui en ont un.
@@ -457,7 +464,7 @@ impl ImportReport {
     /// La forme écrite : la sérialisation, plus un `message` localisé à côté
     /// de chaque perte, de chaque avertissement, de chaque police remplacée,
     /// absente ou aux droits restreints, et de la référence absente ;
-    /// `above_threshold` sur chaque slide dont l'écart dépasse le seuil.
+    /// `above_threshold` sur chaque slide dont la pire région dépasse le seuil.
     pub fn to_json(&self, l: &Localizer) -> Value {
         // Ces types se sérialisent toujours : clés textuelles, nombres finis.
         let mut v = serde_json::to_value(self).unwrap_or_default();
@@ -466,7 +473,7 @@ impl ImportReport {
         if let Some(slides) = v.get_mut("slides").and_then(Value::as_array_mut) {
             for (slide, report) in slides.iter_mut().zip(&self.slides) {
                 add_messages(slide, "losses", &report.losses, l);
-                let above = report.score.as_ref().zip(threshold).is_some_and(|(s, t)| s.score > t);
+                let above = threshold.is_some_and(|t| report.is_above(t));
                 if let (Some(o), true) = (slide.as_object_mut(), above) {
                     o.insert("above_threshold".to_owned(), Value::Bool(true));
                 }
```

`report/reference.rs`: the dead `is_available` is removed; the threshold now applies to the worst region.

```diff
--- a/crates/oku-slides-import/src/report/reference.rs
+++ b/crates/oku-slides-import/src/report/reference.rs
@@ -9,20 +9,13 @@ use serde::Serialize;
 pub struct ReferenceReport {
     /// Nom et version du témoin (`office2pdf 0.8.1`).
     pub witness: String,
-    /// Écart (ΔE OKLab ×100) au-delà duquel une slide est signalée.
+    /// Écart (ΔE OKLab ×100) de sa pire région au-delà duquel une slide est signalée.
     pub threshold: f64,
     /// Pourquoi les slides n'ont pas de score ; `None` : elles en ont un.
     #[serde(skip_serializing_if = "Option::is_none")]
     pub unavailable: Option<ReferenceCause>,
 }
 
-impl ReferenceReport {
-    /// Les slides ont-elles un score ?
-    pub fn is_available(&self) -> bool {
-        self.unavailable.is_none()
-    }
-}
-
 /// Pourquoi la référence manque.
 #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
 #[serde(rename_all = "snake_case", tag = "type")]
```

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-import`
Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-import --all-targets -- -D warnings`
Run: `cargo fmt -p oku-slides-import -- --check`
Expected: all tests pass (the new regression test and the negative-width unit test fail if the `bounds` fix is reverted), clippy clean, fmt clean. Any other crate calling `ReferenceReport::is_available` or reading `SlideScore.score` for the threshold would fail to compile: run `grep -rn "is_available" crates src-tauri` and expect no match on `ReferenceReport`.

- [ ] **Step 5: Commit**
```bash
git add crates/oku-slides-import/src/metrics.rs crates/oku-slides-import/src/metrics_tests.rs crates/oku-slides-import/src/report.rs crates/oku-slides-import/src/report_tests.rs crates/oku-slides-import/src/report/reference.rs crates/oku-slides-import/src/report/reference_tests.rs crates/oku-slides-import/tests/regressions/issue_692_negative_extent.rs crates/oku-slides-import/tests/regressions/main.rs
```
Write this message to a file (for example `/tmp/commit-msg-task-8.txt`), then run `git commit -F /tmp/commit-msg-task-8.txt`:
```
fix(import) : score sans panique sur un cadre négatif, flou σ = 2, slide signalée par sa pire région, régions bornées (t-692)
```

**Extracting from the draft:** the implementer may take the files of this task from the draft commit instead of retyping them: `git checkout eaeb7002 -- crates/oku-slides-import/src/metrics.rs crates/oku-slides-import/src/metrics_tests.rs crates/oku-slides-import/src/report.rs crates/oku-slides-import/src/report_tests.rs crates/oku-slides-import/src/report/reference.rs crates/oku-slides-import/src/report/reference_tests.rs crates/oku-slides-import/tests/regressions/issue_692_negative_extent.rs crates/oku-slides-import/tests/regressions/main.rs` (the list is `git diff --name-only eaeb7002~1 eaeb7002`; never a directory). None of these files changed on `main` since the draft base `53454df2` (checked against `git diff --name-only 53454df2 main`), so no 3-way apply is needed.

### Task 9: Review fixes, witness and worker: pinned witness, slide spans, fonts handed over and stripped from the pptx, no panicking raster, NCSA licence exception

This task applies the plan-review fixes to the reference witness (Tasks 1–2) and its worker transport. `office2pdf` is pinned to exactly `=0.8.1` and the name lives in one `WITNESS` constant (a test reads the workspace `Cargo.toml`). `ReferenceRenderer::render` takes the fonts by value and an optional `SlideSpan` (maps to office2pdf `slide_range`; source positions, hidden slides counted), because one whole-deck request in the worker reached 3.27 GB at 240 slides and held two copies of the fonts. The deck's embedded `ppt/fonts/` parts are stripped before conversion: office2pdf writes them to the system temp dir and only removes them on Drop, so a killed worker would leave licensed fonts on disk (the caller already passes them in memory). `Raster::new` becomes `pub(crate)` and returns `Result` instead of panicking; the per-page sizes in the protocol were dead, so `pages` is a count and `page_size` is removed from `ReferencePdf` and `RemoteReference`. The CLI call site passes `None` to keep the build green (Task 10 slices). A duplicate dev-dependency is removed and `deny.toml` gets a scoped NCSA exception for `libfuzzer-sys` (via office2pdf → image avif → ravif → rav1e, never built).

**Files:**
- Modify: `Cargo.toml` (office2pdf pinned `=0.8.1`, comment)
- Modify: `deny.toml` (NCSA exception for `libfuzzer-sys`)
- Modify: `crates/oku-slides-reference/Cargo.toml` (`zip.workspace = true`)
- Modify: `crates/oku-slides-reference/src/lib.rs` (`SlideSpan`, new `render` signature, `WITNESS` re-export)
- Modify: `crates/oku-slides-reference/src/office2pdf.rs` (`WITNESS`, slide range, `without_fonts`)
- Modify: `crates/oku-slides-reference/src/pdf.rs` (drop `page_size`, `Raster::new` returns `Result`)
- Modify: `crates/oku-slides-reference/src/raster.rs` (`Raster::new` is `pub(crate)` and fallible)
- Modify: `crates/oku-slides-render-worker/Cargo.toml` (remove duplicate dev-dependency)
- Modify: `crates/oku-slides-render-worker/src/protocol.rs` (`slides` in `Reference`, `pages: usize`)
- Modify: `crates/oku-slides-render-worker/src/reference.rs` (`reference(pptx, fonts, slides)`, no `page_size`)
- Modify: `crates/oku-slides-render-worker/src/serve.rs` (fonts handed over, span, page count)
- Modify: `crates/oku-slides-cli/src/import_reference.rs` (`WITNESS`, `None` slide span)
- Test: `crates/oku-slides-reference/src/office2pdf_tests.rs`, `raster_tests.rs`, `pdf_tests.rs`
- Test: `crates/oku-slides-render-worker/src/reference_tests.rs`, `serve_tests.rs`, `client_tests.rs`

`Cargo.lock` does not change (the lock already holds `office2pdf 0.8.1`; `zip` is already a workspace dependency).

**Interfaces:**
- Consumes: `ReferenceRenderer`, `Office2Pdf`, `ReferencePdf`, `Raster` (Tasks 1–2); `RenderWorker::reference`, `RemoteReference` (worker reference transport).
- Produces:
  - `oku_slides_reference::SlideSpan { pub first: u32, pub last: u32 }` (`Debug, Clone, Copy, PartialEq, Eq`; source numbers 1.., inclusive, hidden slides counted)
  - `oku_slides_reference::WITNESS: &str = "office2pdf 0.8.1"`
  - `ReferenceRenderer::render(&self, pptx: &[u8], fonts: Vec<Vec<u8>>, slides: Option<SlideSpan>) -> Result<ReferencePdf, ReferenceError>`
  - `RenderWorker::reference(&self, pptx: &[u8], fonts: &Fonts, slides: Option<SlideSpan>) -> Result<RemoteReference, RenderError>`
  - `Raster::new` is `pub(crate)` and returns `Result<Raster, ReferenceError>` (`Size` kind on a wrong byte count)
  - Removed: `ReferencePdf::page_size`, `RemoteReference::page_size`

- [ ] **Step 1: Write the failing tests**

Test hunks (the new tests are `witness_names_the_version_pinned_in_the_workspace`, `render_of_a_slide_span_gives_its_pages_only`, `without_fonts_*`, `render_of_a_presentation_with_embedded_fonts_still_renders`, `new_with_the_wrong_byte_count_is_a_size_error`, `reference_of_a_slide_span_has_its_pages_only`; the others adapt call sites):

```diff
--- a/crates/oku-slides-reference/src/office2pdf_tests.rs
+++ b/crates/oku-slides-reference/src/office2pdf_tests.rs
@@ -5,7 +5,7 @@ use speculoos::prelude::*;
 use zip::write::SimpleFileOptions;
 
 use super::*;
-use crate::ReferenceErrorKind;
+use crate::{ReferenceErrorKind, SlideSpan};
 
 fn fixture(name: &str) -> Vec<u8> {
     let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import").join(name);
@@ -21,6 +21,24 @@ fn fonts() -> Vec<Vec<u8>> {
         .collect()
 }
 
+/// Les noms des parts de `pptx`.
+fn names(pptx: &[u8]) -> Vec<String> {
+    let zip = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
+    zip.file_names().map(str::to_owned).collect()
+}
+
+/// `pptx` avec une part de plus, `name`.
+fn with_part(pptx: &[u8], name: &str, bytes: &[u8]) -> Vec<u8> {
+    let mut src = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
+    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
+    for i in 0..src.len() {
+        out.raw_copy_file(src.by_index_raw(i).unwrap()).unwrap();
+    }
+    out.start_file(name, SimpleFileOptions::default()).unwrap();
+    out.write_all(bytes).unwrap();
+    out.finish().unwrap().into_inner()
+}
+
 /// `pptx` avec la slide `part` masquée (`show="0"`).
 fn hide(pptx: &[u8], part: &str) -> Vec<u8> {
     let mut src = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
@@ -42,24 +60,54 @@ fn hide(pptx: &[u8], part: &str) -> Vec<u8> {
 
 #[test]
 fn render_libreoffice_fixture_gives_one_page_per_slide() {
-    let pdf = Office2Pdf.render(&fixture("libreoffice.pptx"), &fonts()).unwrap();
+    let pdf = Office2Pdf.render(&fixture("libreoffice.pptx"), fonts(), None).unwrap();
     assert_that!(pdf.page_count()).is_equal_to(3);
 }
 
 #[test]
 fn render_hidden_slide_keeps_its_page() {
     let pptx = hide(&fixture("libreoffice.pptx"), "ppt/slides/slide2.xml");
-    let pdf = Office2Pdf.render(&pptx, &fonts()).unwrap();
+    let pdf = Office2Pdf.render(&pptx, fonts(), None).unwrap();
     assert_that!(pdf.page_count()).is_equal_to(3);
 }
 
 #[test]
 fn render_garbage_is_a_render_error() {
-    let err = Office2Pdf.render(b"not a zip", &[]).unwrap_err();
+    let err = Office2Pdf.render(b"not a zip", vec![], None).unwrap_err();
     assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
 }
 
 #[test]
-fn witness_names_the_crate_version() {
-    assert_that!(Office2Pdf.witness()).is_equal_to("office2pdf 0.8.1");
+fn witness_names_the_version_pinned_in_the_workspace() {
+    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml")).unwrap();
+    let pin = manifest.lines().find(|l| l.starts_with("office2pdf = ")).unwrap_or_default().to_owned();
+    let version = WITNESS.trim_start_matches("office2pdf ");
+    assert_that!(pin).contains(format!(r#"version = "={version}""#));
+}
+
+#[test]
+fn render_of_a_slide_span_gives_its_pages_only() {
+    let span = SlideSpan { first: 2, last: 3 };
+    let pdf = Office2Pdf.render(&fixture("libreoffice.pptx"), fonts(), Some(span)).unwrap();
+    assert_that!(pdf.page_count()).is_equal_to(2);
+}
+
+#[test]
+fn without_fonts_drops_the_embedded_font_parts() {
+    let pptx = with_part(&fixture("libreoffice.pptx"), "ppt/fonts/font1.fntdata", b"police");
+    let stripped = without_fonts(&pptx).unwrap();
+    assert_that!(names(&stripped).iter().any(|n| n.starts_with("ppt/fonts/"))).is_false();
+}
+
+#[test]
+fn without_fonts_keeps_every_other_part() {
+    let pptx = fixture("libreoffice.pptx");
+    let with = with_part(&pptx, "ppt/fonts/font1.fntdata", b"police");
+    assert_that!(names(&without_fonts(&with).unwrap())).is_equal_to(names(&pptx));
+}
+
+#[test]
+fn render_of_a_presentation_with_embedded_fonts_still_renders() {
+    let pptx = with_part(&fixture("libreoffice.pptx"), "ppt/fonts/font1.fntdata", b"police");
+    assert_that!(Office2Pdf.render(&pptx, fonts(), None).map(|p| p.page_count()).ok()).is_equal_to(Some(3));
 }
--- a/crates/oku-slides-reference/src/pdf_tests.rs
+++ b/crates/oku-slides-reference/src/pdf_tests.rs
@@ -10,7 +10,7 @@ fn libreoffice() -> ReferencePdf {
     let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
     let pptx = std::fs::read(root.join("fixtures/import/libreoffice.pptx")).unwrap();
     let font = std::fs::read(root.join("assets/fonts/carlito-400.ttf")).unwrap();
-    Office2Pdf.render(&pptx, &[font]).unwrap()
+    Office2Pdf.render(&pptx, vec![font], None).unwrap()
 }
 
 #[test]
@@ -44,12 +44,6 @@ fn raster_too_wide_is_a_size_error() {
     assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Size);
 }
 
-#[test]
-fn page_size_reads_the_points() {
-    let (w, h) = libreoffice().page_size(0).unwrap();
-    assert_that!((w.round(), h.round())).is_equal_to((960.0, 540.0));
-}
-
 #[test]
 fn new_without_pdf_is_a_pdf_error() {
     let err = ReferencePdf::new(b"%PDF-garbage".to_vec()).unwrap_err();
--- a/crates/oku-slides-reference/src/raster_tests.rs
+++ b/crates/oku-slides-reference/src/raster_tests.rs
@@ -5,7 +5,13 @@ use crate::ReferenceErrorKind;
 
 /// Une image 3 × 2 : chaque pixel porte son rang dans sa composante rouge.
 fn ramp() -> Raster {
-    Raster::new(3, 2, (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect())
+    Raster::new(3, 2, (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect()).unwrap()
+}
+
+#[test]
+fn new_with_the_wrong_byte_count_is_a_size_error() {
+    let err = Raster::new(3, 2, vec![0; 23]).unwrap_err();
+    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Size);
 }
 
 #[test]
--- a/crates/oku-slides-render-worker/src/client_tests.rs
+++ b/crates/oku-slides-render-worker/src/client_tests.rs
@@ -125,7 +125,7 @@ fn remote_compiled_of_previous_worker_is_stale_and_its_release_is_dropped() {
 fn remote_reference_of_previous_worker_is_stale() {
     let w = worker(3_000);
     let pptx = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/libreoffice.pptx")).unwrap();
-    let old = w.reference(&pptx, &Fonts::default()).unwrap();
+    let old = w.reference(&pptx, &Fonts::default(), None).unwrap();
     force_restart(&w, "boucle-r");
     assert_that!(old.png(0, 480).unwrap_err().kind()).is_equal_to(RenderErrorKind::Stale);
 }
--- a/crates/oku-slides-render-worker/src/reference_tests.rs
+++ b/crates/oku-slides-render-worker/src/reference_tests.rs
@@ -35,34 +35,27 @@ fn fonts() -> Fonts {
 
 #[test]
 fn reference_gives_the_witness_and_its_pages() {
-    let r = worker(30_000).reference(&pptx(), &fonts()).unwrap();
+    let r = worker(30_000).reference(&pptx(), &fonts(), None).unwrap();
     assert_that!((r.witness(), r.page_count())).is_equal_to(("office2pdf 0.8.1", 3));
 }
 
 #[test]
 fn reference_png_has_the_requested_width() {
-    let r = worker(30_000).reference(&pptx(), &fonts()).unwrap();
+    let r = worker(30_000).reference(&pptx(), &fonts(), None).unwrap();
     let png = oku_slides_reference::Raster::from_png(&r.png(0, 960).unwrap()).unwrap();
     assert_that!((png.width(), png.height())).is_equal_to((960, 540));
 }
 
-#[test]
-fn reference_page_size_is_in_points() {
-    let r = worker(30_000).reference(&pptx(), &fonts()).unwrap();
-    let (w, h) = r.page_size(0).unwrap();
-    assert_that!((w.round(), h.round())).is_equal_to((960.0, 540.0));
-}
-
 #[test]
 fn reference_of_garbage_is_an_error_and_the_worker_goes_on() {
     let w = worker(30_000);
-    assert_that!(w.reference(b"not a pptx", &fonts()).is_err()).is_true();
-    assert_that!(w.reference(&pptx(), &fonts()).is_ok()).is_true();
+    assert_that!(w.reference(b"not a pptx", &fonts(), None).is_err()).is_true();
+    assert_that!(w.reference(&pptx(), &fonts(), None).is_ok()).is_true();
 }
 
 #[test]
 fn reference_beyond_the_timeout_is_a_timeout() {
-    let err = worker(1).reference(&pptx(), &fonts()).unwrap_err();
+    let err = worker(1).reference(&pptx(), &fonts(), None).unwrap_err();
     assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
 }
 
@@ -71,3 +64,10 @@ fn remote_reference_is_send() {
     fn send<T: Send>() {}
     send::<crate::RemoteReference>();
 }
+
+#[test]
+fn reference_of_a_slide_span_has_its_pages_only() {
+    let span = oku_slides_reference::SlideSpan { first: 2, last: 2 };
+    let r = worker(30_000).reference(&pptx(), &fonts(), Some(span)).unwrap();
+    assert_that!(r.page_count()).is_equal_to(1);
+}
--- a/crates/oku-slides-render-worker/src/serve_tests.rs
+++ b/crates/oku-slides-render-worker/src/serve_tests.rs
@@ -237,9 +237,9 @@ fn libreoffice_pptx() -> Vec<u8> {
 #[test]
 fn serve_reference_then_png_returns_a_png() {
     let mut h = Harness::new();
-    let reference = match h.call(&Request::Reference { fonts: vec![] }, &libreoffice_pptx()).0 {
+    let reference = match h.call(&Request::Reference { fonts: vec![], slides: None }, &libreoffice_pptx()).0 {
         Response::Reference { reference, pages, .. } => {
-            assert_that!(pages.len()).is_equal_to(3);
+            assert_that!(pages).is_equal_to(3);
             reference
         }
         other => panic!("{other:?}"),
@@ -252,7 +252,8 @@ fn serve_reference_then_png_returns_a_png() {
 #[test]
 fn serve_png_of_released_reference_returns_stale() {
     let mut h = Harness::new();
-    let Response::Reference { reference, .. } = h.call(&Request::Reference { fonts: vec![] }, &libreoffice_pptx()).0
+    let Response::Reference { reference, .. } =
+        h.call(&Request::Reference { fonts: vec![], slides: None }, &libreoffice_pptx()).0
     else {
         panic!("pas de référence")
     };
@@ -264,6 +265,6 @@ fn serve_png_of_released_reference_returns_stale() {
 #[test]
 fn serve_reference_with_unknown_font_returns_setup() {
     let mut h = Harness::new();
-    let (resp, _) = h.call(&Request::Reference { fonts: vec!["inconnue".into()] }, &libreoffice_pptx());
+    let (resp, _) = h.call(&Request::Reference { fonts: vec!["inconnue".into()], slides: None }, &libreoffice_pptx());
     assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
 }
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-reference -p oku-slides-render-worker`
Expected: compile errors naming the missing items (`SlideSpan`, `WITNESS`, `without_fonts`, the three-argument `render` and `reference`, `Raster::new` returning a `Result`).

- [ ] **Step 3: Implement**

Pin the witness (it must equal the `WITNESS` constant). If this hunk does not apply cleanly because `Cargo.toml` changed on main (version bump), apply it with `git apply --3way`.

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -77,8 +77,9 @@ typst-layout = "0.15.1"
 # Aperçu des widgets recadré sur leur dessin (t-392) ; déjà tiré par typst-render.
 png = "0.18"
 # Témoin du rendu de référence de l'import (t-692, lot 4) : PPTX → PDF en Rust pur, sur Typst 0.15 ;
-# sans les polices embarquées de typst-kit (l'appelant passe les siennes).
-office2pdf = { version = "0.8.1", default-features = false }
+# sans les polices embarquées de typst-kit (l'appelant passe les siennes). Épinglé : le rapport nomme
+# la version (`oku_slides_reference::WITNESS`) et les bornes des tests sont mesurées avec elle.
+office2pdf = { version = "=0.8.1", default-features = false }
 # Rastérisation des pages du PDF de référence ; déjà tiré par typst-render.
 hayro = { version = "0.7.1", default-features = false }
 # Serveur de développement (`oku-slides serve`, t-419) ; déjà tiré par rust-mcp-axum.
```

Scoped licence exception (`cargo deny check licenses` failed on `libfuzzer-sys`, which is never built):

```diff
--- a/deny.toml
+++ b/deny.toml
@@ -34,6 +34,11 @@ allow = [
     "Zlib",
 ]
 confidence-threshold = 0.9
+# libfuzzer-sys ((MIT OR Apache-2.0) AND NCSA) : tiré par office2pdf → image (avif) → ravif → rav1e,
+# qui ne le compile que pour son fuzzing ; jamais construit ni livré (t-692, lot 4).
+exceptions = [
+    { crate = "libfuzzer-sys", allow = ["NCSA"] },
+]
 
 [licenses.private]
 # Les crates du workspace ne sont pas publiées.
```

Reference crate: the `zip` dependency, `SlideSpan`, the new `render` signature and `WITNESS`, the span and font stripping, no `page_size`, a fallible `Raster::new`.

```diff
--- a/crates/oku-slides-reference/Cargo.toml
+++ b/crates/oku-slides-reference/Cargo.toml
@@ -12,6 +12,7 @@ office2pdf.workspace = true
 hayro.workspace = true
 png.workspace = true
 snafu.workspace = true
+zip.workspace = true
 
 [dev-dependencies]
 speculoos.workspace = true
--- a/crates/oku-slides-reference/src/lib.rs
+++ b/crates/oku-slides-reference/src/lib.rs
@@ -2,12 +2,14 @@
 //! d'Oku rend le `.pptx` en PDF, dont les pages sont rastérisées pour être
 //! comparées au rendu Oku (écart visuel du hi-fi, `oku_slides_import::metrics`).
 //!
-//! - [`ReferenceRenderer`] : un témoin (`.pptx` + polices → PDF) ;
-//!   [`Office2Pdf`] est le seul livré (crate `office2pdf`, Rust pur, sur Typst).
+//! - [`ReferenceRenderer`] : un témoin (`.pptx` + polices → PDF, une tranche
+//!   de slides [`SlideSpan`] ou toutes) ; [`Office2Pdf`] est le seul livré
+//!   (crate `office2pdf`, Rust pur, sur Typst ; [`WITNESS`], son nom).
 //! - [`ReferencePdf`] : le PDF rendu, ses pages, rastérisées par `hayro`
 //!   ([`ReferencePdf::raster`]) à une largeur donnée.
 //! - [`Raster`] : une image RGBA 8 bits opaque ; [`Raster::from_png`],
-//!   [`Raster::to_png`] (le tube du processus de rendu transporte des PNG).
+//!   [`Raster::to_png`] (le tube du processus de rendu transporte des PNG),
+//!   [`Raster::crop`].
 //! - [`ReferenceError`], [`ReferenceErrorKind`] : opaques ; jamais fatals pour
 //!   l'import, qui dit au rapport que la référence manque.
 //!
@@ -21,7 +23,7 @@ mod pdf;
 mod raster;
 
 pub use error::{ReferenceError, ReferenceErrorKind};
-pub use office2pdf::Office2Pdf;
+pub use office2pdf::{Office2Pdf, WITNESS};
 pub use pdf::{MAX_RASTER_SIDE, ReferencePdf};
 pub use raster::Raster;
 
@@ -35,6 +37,25 @@ pub trait ReferenceRenderer: std::fmt::Debug + Send + Sync {
     /// Nom et version du témoin, tel que le rapport l'écrit (`office2pdf 0.8.1`).
     fn witness(&self) -> &'static str;
 
-    /// Le PDF de `pptx`, rendu avec `fonts` (octets de fichiers de police).
-    fn render(&self, pptx: &[u8], fonts: &[Vec<u8>]) -> Result<ReferencePdf, ReferenceError>;
+    /// Le PDF de `pptx` rendu avec `fonts`, ses slides `slides` ou toutes.
+    ///
+    /// `fonts` : octets de fichiers de police, cédés au témoin (il les garde le
+    /// temps du rendu). Une page par slide de la tranche, dans l'ordre.
+    fn render(
+        &self,
+        pptx: &[u8],
+        fonts: Vec<Vec<u8>>,
+        slides: Option<SlideSpan>,
+    ) -> Result<ReferencePdf, ReferenceError>;
+}
+
+/// Une tranche de slides : numéros source de la première et de la dernière (1.., inclus).
+///
+/// Les slides masquées comptent : la tranche suit les numéros de la présentation.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct SlideSpan {
+    /// Numéro de la première slide (1..).
+    pub first: u32,
+    /// Numéro de la dernière slide, incluse.
+    pub last: u32,
 }
--- a/crates/oku-slides-reference/src/office2pdf.rs
+++ b/crates/oku-slides-reference/src/office2pdf.rs
@@ -4,10 +4,25 @@
 //! Les polices fournies passent avant toutes les autres ; celles du système
 //! restent un repli (le crate les cherche toujours, hors WASM). Les slides
 //! masquées sont rendues : l'import les reprend visibles, une page par slide.
+//! Les polices embarquées du `.pptx` (`ppt/fonts/`) sont ôtées avant la
+//! conversion : `office2pdf` les écrirait dans le dossier temporaire du
+//! système, et un processus de rendu tué les y laisserait ; l'appelant les
+//! fournit déjà en mémoire.
+
+use std::io::Cursor;
+
+use office2pdf::config::{ConvertOptions, Format, SlideRange};
 
 use crate::error::RenderSnafu;
-use crate::{ReferenceError, ReferencePdf, ReferenceRenderer};
-use office2pdf::config::{ConvertOptions, Format};
+use crate::{ReferenceError, ReferencePdf, ReferenceRenderer, SlideSpan};
+
+/// Nom et version du témoin `office2pdf`, tels que le rapport les écrit.
+///
+/// La version est celle que `Cargo.toml` épingle (`=0.8.1`) : un test le vérifie.
+pub const WITNESS: &str = "office2pdf 0.8.1";
+
+/// Le dossier des polices embarquées d'un `.pptx`.
+const FONTS_DIR: &str = "ppt/fonts/";
 
 /// Le témoin `office2pdf`.
 #[derive(Debug, Clone, Copy, Default)]
@@ -15,17 +30,45 @@ pub struct Office2Pdf;
 
 impl ReferenceRenderer for Office2Pdf {
     fn witness(&self) -> &'static str {
-        "office2pdf 0.8.1"
+        WITNESS
     }
 
-    fn render(&self, pptx: &[u8], fonts: &[Vec<u8>]) -> Result<ReferencePdf, ReferenceError> {
-        let options = ConvertOptions { include_hidden_slides: true, font_bytes: fonts.to_vec(), ..Default::default() };
-        let converted = office2pdf::convert_bytes(pptx, Format::Pptx, &options)
-            .map_err(|e| ReferenceError::from(RenderSnafu { message: e.to_string() }.build()))?;
+    fn render(
+        &self,
+        pptx: &[u8],
+        fonts: Vec<Vec<u8>>,
+        slides: Option<SlideSpan>,
+    ) -> Result<ReferencePdf, ReferenceError> {
+        let render_err = |message: String| ReferenceError::from(RenderSnafu { message }.build());
+        let pptx = without_fonts(pptx).map_err(|e| render_err(e.to_string()))?;
+        let options = ConvertOptions {
+            include_hidden_slides: true,
+            font_bytes: fonts,
+            slide_range: slides.map(|s| SlideRange::new(s.first, s.last)),
+            ..Default::default()
+        };
+        let converted =
+            office2pdf::convert_bytes(&pptx, Format::Pptx, &options).map_err(|e| render_err(e.to_string()))?;
         ReferencePdf::new(converted.pdf)
     }
 }
 
+/// `pptx` sans ses parts de `ppt/fonts/`, les autres copiées telles quelles.
+fn without_fonts(pptx: &[u8]) -> Result<Vec<u8>, zip::result::ZipError> {
+    let mut source = zip::ZipArchive::new(Cursor::new(pptx))?;
+    if !source.file_names().any(|n| n.starts_with(FONTS_DIR)) {
+        return Ok(pptx.to_vec());
+    }
+    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
+    for i in 0..source.len() {
+        let entry = source.by_index_raw(i)?;
+        if !entry.name().starts_with(FONTS_DIR) {
+            out.raw_copy_file(entry)?;
+        }
+    }
+    Ok(out.finish()?.into_inner())
+}
+
 #[cfg(test)]
 #[path = "office2pdf_tests.rs"]
 mod office2pdf_tests;
--- a/crates/oku-slides-reference/src/pdf.rs
+++ b/crates/oku-slides-reference/src/pdf.rs
@@ -39,12 +39,6 @@ impl ReferencePdf {
         self.pdf.pages().len()
     }
 
-    /// Taille de la page `index` (à partir de 0), en pt : largeur, hauteur.
-    pub fn page_size(&self, index: usize) -> Result<(f32, f32), ReferenceError> {
-        let page = self.pdf.pages().get(index).ok_or_else(|| NoPageSnafu { index }.build())?;
-        Ok(page.render_dimensions())
-    }
-
     /// La page `index` sur fond blanc, à `width` pixels de large (la hauteur
     /// suit le ratio de la page, arrondie).
     pub fn raster(&self, index: usize, width: u32) -> Result<Raster, ReferenceError> {
@@ -64,7 +58,7 @@ impl ReferencePdf {
         };
         let pixmap = hayro::render(page, &RenderCache::new(), &InterpreterSettings::default(), &settings);
         // Fond blanc opaque : les composantes prémultipliées sont les composantes droites.
-        Ok(Raster::new(width, height, pixmap.data_as_u8_slice().to_vec()))
+        Raster::new(width, height, pixmap.data_as_u8_slice().to_vec())
     }
 }
 
--- a/crates/oku-slides-reference/src/raster.rs
+++ b/crates/oku-slides-reference/src/raster.rs
@@ -20,14 +20,12 @@ impl std::fmt::Debug for Raster {
 }
 
 impl Raster {
-    /// Une image de `width` × `height` pixels ; `rgba` en a exactement 4 octets par pixel.
+    /// Une image de `width` × `height` pixels, `rgba` à 4 octets par pixel.
     ///
-    /// # Panics
-    ///
-    /// Si `rgba` n'a pas la taille de l'image (un bogue de l'appelant).
-    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
-        assert_eq!(rgba.len(), width as usize * height as usize * 4, "taille RGBA");
-        Self { width, height, rgba }
+    /// Des octets d'une autre taille sont une erreur `Size`.
+    pub(crate) fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ReferenceError> {
+        ensure!(rgba.len() == width as usize * height as usize * 4, SizeSnafu { width, height });
+        Ok(Self { width, height, rgba })
     }
 
     /// Largeur, en pixels.
```

Worker: the `slides` field and page count in the protocol, the span on `RenderWorker::reference`, one copy of the fonts handed to the witness in `serve.rs`, and the duplicate dev-dependency removed.

```diff
--- a/crates/oku-slides-render-worker/Cargo.toml
+++ b/crates/oku-slides-render-worker/Cargo.toml
@@ -35,7 +35,6 @@ libc.workspace = true
 [dev-dependencies]
 speculoos.workspace = true
 rstest.workspace = true
-oku-slides-reference.workspace = true
 
 [lints]
 workspace = true
--- a/crates/oku-slides-render-worker/src/protocol.rs
+++ b/crates/oku-slides-render-worker/src/protocol.rs
@@ -76,9 +76,12 @@ pub(crate) enum Request {
         slot: SlotId,
         generations: Vec<u64>,
     },
-    /// Rend la présentation de la charge (`.pptx`) par le témoin, avec ces polices.
+    /// Rend la présentation de la charge (`.pptx`) par le témoin, avec ces
+    /// polices ; `slides` : numéros source de la première et de la dernière slide.
     Reference {
         fonts: Vec<String>,
+        #[serde(default)]
+        slides: Option<(u32, u32)>,
     },
     /// La page `page` d'une référence, en PNG de `width` pixels de large.
     ReferencePng {
@@ -115,11 +118,11 @@ pub(crate) enum Response {
     Compiled(Box<CompiledHeader>),
     /// Charge : octets du résultat.
     Bytes,
-    /// Une référence rendue : son identifiant, le témoin et la taille de chaque page (pt).
+    /// Une référence rendue : son identifiant, le témoin et son nombre de pages.
     Reference {
         reference: u64,
         witness: String,
-        pages: Vec<(f32, f32)>,
+        pages: usize,
     },
     Error {
         error: oku_slides_render::WireError,
--- a/crates/oku-slides-render-worker/src/reference.rs
+++ b/crates/oku-slides-render-worker/src/reference.rs
@@ -6,6 +6,7 @@
 //! une référence d'une autre époque répond `Stale` sans appel ; le `Drop` met
 //! `ReleaseReference` en file.
 
+use oku_slides_reference::SlideSpan;
 use oku_slides_render::{Fonts, RenderError};
 
 use crate::client::{Deferred, RenderWorker, put_fonts, unexpected};
@@ -18,7 +19,7 @@ pub struct RemoteReference {
     /// Époque du travailleur qui l'a rendu.
     epoch: u64,
     witness: String,
-    pages: Vec<(f32, f32)>,
+    pages: usize,
 }
 
 impl std::fmt::Debug for RemoteReference {
@@ -26,19 +27,27 @@ impl std::fmt::Debug for RemoteReference {
         f.debug_struct("RemoteReference")
             .field("reference", &self.reference)
             .field("witness", &self.witness)
-            .field("pages", &self.pages.len())
+            .field("pages", &self.pages)
             .finish_non_exhaustive()
     }
 }
 
 impl RenderWorker {
     /// Le rendu de référence de `pptx` par le témoin, avec `fonts` en priorité.
-    pub fn reference(&self, pptx: &[u8], fonts: &Fonts) -> Result<RemoteReference, RenderError> {
+    ///
+    /// `slides` : la tranche à rendre (numéros source, masquées comprises), ou toutes.
+    pub fn reference(
+        &self,
+        pptx: &[u8],
+        fonts: &Fonts,
+        slides: Option<SlideSpan>,
+    ) -> Result<RemoteReference, RenderError> {
         let mut link = self.link();
         self.prepare(&mut link)?;
         let ids = put_fonts(&mut link, fonts)?;
         let epoch = link.restarts;
-        match link.call(&Request::Reference { fonts: ids }, pptx)? {
+        let slides = slides.map(|s| (s.first, s.last));
+        match link.call(&Request::Reference { fonts: ids, slides }, pptx)? {
             (Response::Reference { reference, witness, pages }, _) => {
                 Ok(RemoteReference { worker: self.clone(), reference, epoch, witness, pages })
             }
@@ -56,12 +65,7 @@ impl RemoteReference {
 
     /// Nombre de pages.
     pub fn page_count(&self) -> usize {
-        self.pages.len()
-    }
-
-    /// Taille de la page `index`, en pt : largeur, hauteur.
-    pub fn page_size(&self, index: usize) -> Option<(f32, f32)> {
-        self.pages.get(index).copied()
+        self.pages
     }
 
     /// PNG de la page `index` (0 = première), à `width` pixels de large.
--- a/crates/oku-slides-render-worker/src/serve.rs
+++ b/crates/oku-slides-render-worker/src/serve.rs
@@ -17,7 +17,7 @@
 //! - `Svg`, `Png` et `Pdf` d'une génération absente répondent `Error Stale`.
 //! - `Release` retire les générations nommées ; `CloseSlot` l'emplacement.
 //! - `Reference` rend la charge (un `.pptx`) par le témoin (`Office2Pdf`) avec
-//!   les polices nommées, sous un nouvel identifiant (même compteur que les
+//!   les polices nommées, une tranche de slides ou toutes, sous un nouvel identifiant (même compteur que les
 //!   générations) ; `ReferencePng` en rastérise une page, `ReleaseReference`
 //!   l'oublie. Une référence absente répond `Error Stale` ; un échec du témoin,
 //!   `Error Setup` (`reference: …`).
@@ -32,7 +32,7 @@ use std::time::Instant;
 
 use oku_slides_core::WidgetFile;
 use oku_slides_i18n::Localizer;
-use oku_slides_reference::{Office2Pdf, ReferenceError, ReferencePdf, ReferenceRenderer};
+use oku_slides_reference::{Office2Pdf, ReferenceError, ReferencePdf, ReferenceRenderer, SlideSpan};
 use oku_slides_render::{Compiled, Fonts, PackWidgets, Registries, RenderError, Renderer, WireError};
 use serde_json::{Map, Value};
 use snafu::Snafu;
@@ -161,7 +161,7 @@ impl Worker {
                 }
                 ok()
             }
-            Request::Reference { fonts } => self.reference(&fonts, &payload),
+            Request::Reference { fonts, slides } => self.reference(&fonts, slides, &payload),
             Request::ReferencePng { reference, page, width } => self.reference_png(reference, page, width),
             Request::ReleaseReference { reference } => {
                 self.references.remove(&reference);
@@ -171,12 +171,16 @@ impl Worker {
         result.unwrap_or_else(error_reply)
     }
 
-    fn reference(&mut self, fonts: &[String], pptx: &[u8]) -> Result<Reply, WireError> {
-        let files = self.fonts_of(fonts)?.files().map(<[u8]>::to_vec).collect::<Vec<_>>();
+    fn reference(&mut self, fonts: &[String], slides: Option<(u32, u32)>, pptx: &[u8]) -> Result<Reply, WireError> {
+        // Une seule copie des polices : le témoin les garde le temps du rendu.
+        let files = fonts
+            .iter()
+            .map(|id| self.fonts.get(id).cloned().ok_or_else(|| setup(format!("unknown font {id}"))))
+            .collect::<Result<Vec<_>, _>>()?;
         let witness = Office2Pdf;
-        let pdf = witness.render(pptx, &files).map_err(|e| reference_error(&e))?;
-        let pages = (0..pdf.page_count()).map(|i| pdf.page_size(i)).collect::<Result<Vec<_>, _>>();
-        let pages = pages.map_err(|e| reference_error(&e))?;
+        let span = slides.map(|(first, last)| SlideSpan { first, last });
+        let pdf = witness.render(pptx, files, span).map_err(|e| reference_error(&e))?;
+        let pages = pdf.page_count();
         self.next_generation += 1;
         let reference = self.next_generation;
         self.references.insert(reference, pdf);
```

CLI call site: `WITNESS` and no slice yet (Task 10 slices):

```diff
--- a/crates/oku-slides-cli/src/import_reference.rs
+++ b/crates/oku-slides-cli/src/import_reference.rs
@@ -10,7 +10,7 @@ use oku_slides_import::metrics::{self, Image};
 use oku_slides_import::report::{ReferenceCause, ReferenceReport};
 use oku_slides_import::source::SourceDeck;
 use oku_slides_import::{ImportReport, hifi};
-use oku_slides_reference::{Office2Pdf, Raster, ReferenceRenderer};
+use oku_slides_reference::{Raster, WITNESS};
 use oku_slides_render::{Fonts, RenderError, RenderErrorKind};
 use oku_slides_render_worker::{RemoteReference, RenderWorker};
 
@@ -35,7 +35,7 @@ pub(crate) struct Scoring<'a> {
 
 /// Pose dans `report` la référence et l'écart de chaque slide de `doc`.
 pub(crate) fn score(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport) {
-    let result = s.worker.reference(s.pptx, s.fonts).map_err(|e| cause(&e)).and_then(|reference| {
+    let result = s.worker.reference(s.pptx, s.fonts, None).map_err(|e| cause(&e)).and_then(|reference| {
         let (pages, deck) = (reference.page_count(), doc.page_count());
         if pages != deck || pages != report.slides.len() {
             let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
@@ -49,7 +49,7 @@ pub(crate) fn score(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport)
             for slide in &mut report.slides {
                 slide.score = None;
             }
-            (Office2Pdf.witness().to_owned(), Some(cause))
+            (WITNESS.to_owned(), Some(cause))
         }
     };
     report.reference = Some(ReferenceReport { witness, threshold: s.threshold, unavailable });
```

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-reference -p oku-slides-render-worker -p oku-slides-cli`
Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-reference -p oku-slides-render-worker -p oku-slides-cli --all-targets -- -D warnings`
Run: `cargo fmt -p oku-slides-reference -p oku-slides-render-worker -p oku-slides-cli -- --check`
Run: `cargo deny check licenses`
Expected: all tests pass, clippy and fmt clean, licences ok (no `NCSA` failure); `git diff Cargo.lock` is empty.

- [ ] **Step 5: Commit**
```bash
git add Cargo.toml deny.toml crates/oku-slides-cli/src/import_reference.rs crates/oku-slides-reference/Cargo.toml crates/oku-slides-reference/src/lib.rs crates/oku-slides-reference/src/office2pdf.rs crates/oku-slides-reference/src/office2pdf_tests.rs crates/oku-slides-reference/src/pdf.rs crates/oku-slides-reference/src/pdf_tests.rs crates/oku-slides-reference/src/raster.rs crates/oku-slides-reference/src/raster_tests.rs crates/oku-slides-render-worker/Cargo.toml crates/oku-slides-render-worker/src/client_tests.rs crates/oku-slides-render-worker/src/protocol.rs crates/oku-slides-render-worker/src/reference.rs crates/oku-slides-render-worker/src/reference_tests.rs crates/oku-slides-render-worker/src/serve.rs crates/oku-slides-render-worker/src/serve_tests.rs
```
Write the message to a file (for example `/tmp/commit-task-9.txt`) with this subject, then run `git commit -F /tmp/commit-task-9.txt`:
```
fix(reference) : témoin épinglé (=0.8.1, WITNESS), tranches de slides, polices cédées sans copie et ôtées du pptx, Raster sans panique, licence NCSA de libfuzzer-sys (t-692)
```

**Extracting from the draft:** the implementer may take the files of this task from the draft commit `211ded7` instead of retyping them: `git checkout 211ded7 -- deny.toml crates/oku-slides-cli/src/import_reference.rs crates/oku-slides-reference/Cargo.toml crates/oku-slides-reference/src/lib.rs crates/oku-slides-reference/src/office2pdf.rs crates/oku-slides-reference/src/office2pdf_tests.rs crates/oku-slides-reference/src/pdf.rs crates/oku-slides-reference/src/pdf_tests.rs crates/oku-slides-reference/src/raster.rs crates/oku-slides-reference/src/raster_tests.rs crates/oku-slides-render-worker/Cargo.toml crates/oku-slides-render-worker/src/client_tests.rs crates/oku-slides-render-worker/src/protocol.rs crates/oku-slides-render-worker/src/reference.rs crates/oku-slides-render-worker/src/reference_tests.rs crates/oku-slides-render-worker/src/serve.rs crates/oku-slides-render-worker/src/serve_tests.rs` (never a directory; the list is `git diff --name-only 211ded7~1 211ded7` minus `Cargo.toml`). `Cargo.toml` was also changed on `main` since the draft base `53454df2` (version bump), so apply its hunk with `git show 211ded7 -- Cargo.toml | git apply --3way` instead of checking it out. No other file of this task changed on `main`.

### Task 10: Review fixes, CLI: the witness in its own render worker, 20 slides at a time, no silent fallback; worst-region threshold, validated; overflow tested end to end

This task applies the plan-review fixes to the hi-fi visual gap in the CLI. The witness now runs in a render worker of its own (bounded by the new `[import] reference_timeout_ms = 30000`, which also makes the failure path testable), renders `SPAN` = 20 slides per request (`spans`, unit-tested) and is paired with the Oku slides by source number. Silent fallbacks are removed: a missing Oku page, a crop size mismatch or an unscorable slide yields `ReferenceCause::Failed` with the detail logged through `tracing::warn!`, and then no slide keeps a score; crops are cut to the common size and the result line no longer prints a mean of 0. `[import] score_threshold` now compares the worst region of each slide (default 10.0, measured on alto with office2pdf 0.8.1, σ = 2: titles aligned within 1 px score 7.7–8.3, real gaps 8.7–22.5, witness errors up to 46) and is validated (finite, >= 0) with the new message `cli-import-bad-threshold`. `DeckWidgets::worker` (dead after this change) is removed, `ImportSettings` becomes `#[non_exhaustive]`, and the ko `cli-import-scored` message is fixed (an i18n test forbids a space between a number and the following Hangul). The existing hi-fi CLI tests pass `--reference none`; new tests cover the reference timeout, the refused negative threshold (French) and a retrofit overflow end to end.

**Files:**
- Create: `crates/oku-slides-cli/src/import_reference_tests.rs` (unit tests of `spans` and `cause`)
- Modify: `crates/oku-slides-cli/src/import.rs` (own worker with the reference timeout, threshold validation, result line without a fake mean)
- Modify: `crates/oku-slides-cli/src/import_reference.rs` (slices of `SPAN`, pairing by source number, no silent fallback, `tracing::warn!`)
- Modify: `crates/oku-slides-cli/src/pack_render.rs` (remove the dead `DeckWidgets::worker`)
- Modify: `crates/oku-slides-platform/src/config.rs` (`#[non_exhaustive]`, `reference_timeout_ms`, doc of `score_threshold`)
- Modify: `crates/oku-slides-platform/src/config.default.toml` (`score_threshold = 10.0`, `reference_timeout_ms = 30000`)
- Modify: `crates/oku-slides-i18n/src/locales/{ar,bn,de,en,es,fr,hi,it,ja,ko,nl,pl,pt,ru,th,tr,uk,vi,zh-Hans,zh-Hant}/cli.ftl` (new `cli-import-bad-threshold`; ko `cli-import-scored` fixed)
- Test: `crates/oku-slides-cli/src/import_reference_tests.rs`, `crates/oku-slides-cli/tests/import.rs`, `crates/oku-slides-cli/tests/import_reference.rs`, `crates/oku-slides-platform/src/config_tests.rs`

`crates/oku-slides-import/ARCHITECTURE.md` and the other `ARCHITECTURE.md` files: this task touches none.

**Interfaces:**
- Consumes: `oku_slides_reference::{Raster, SlideSpan { first: u32, last: u32 }, WITNESS}` (Task 6); `RenderWorker::reference(&self, pptx: &[u8], fonts: &Fonts, span: Option<SlideSpan>) -> Result<RemoteReference, RenderError>` with `RemoteReference::{page_count, png, witness}` (Tasks 8 and 9); `Raster::{from_png, crop, width, height, rgba}`; `metrics::{score, regions, Image}`; `ReferenceCause::{Timeout, Memory, Pages { reference, deck }, Failed}` and `ReferenceReport`; `Localize::localize` for `ReferenceCause`; `Rendered::{layout, png, page_count}`; `Layout::page_of(&str) -> Option<usize>` (1-based pages); `ImportReport::{mean_score, above_threshold}`.
- Produces: `pub(crate) const SPAN: usize = 20` and `pub(crate) fn spans(numbers: &[u32], size: usize) -> Vec<SlideSpan>` in `oku-slides-cli`; `Scoring` gains `l: &'a Localizer` and `worker` is now the witness' own worker; `ImportSettings.reference_timeout_ms: u64` (`[import] reference_timeout_ms`, env `OKU_SLIDES_IMPORT__REFERENCE_TIMEOUT_MS`) and `score_threshold` default 10.0 compared with the worst region; Fluent key `cli-import-bad-threshold` (argument `$value`) in the 20 locales; `DeckWidgets::worker` no longer exists.

- [ ] **Step 1: Write the failing tests**

New unit tests, in a sibling file included by `import_reference.rs` (the `#[cfg(test)] #[path]` hunk is in Step 3):

```diff
new file mode 100644
--- /dev/null
+++ b/crates/oku-slides-cli/src/import_reference_tests.rs
@@ -0,0 +1,33 @@
+use oku_slides_render::RenderError;
+use rstest::rstest;
+use speculoos::prelude::*;
+
+use super::*;
+
+fn span(first: u32, last: u32) -> SlideSpan {
+    SlideSpan { first, last }
+}
+
+#[test]
+fn spans_cut_the_slides_in_groups_of_size() {
+    assert_that!(spans(&[1, 2, 3, 4, 5], 2)).is_equal_to(vec![span(1, 2), span(3, 4), span(5, 5)]);
+}
+
+#[test]
+fn spans_cover_the_slides_the_import_omitted_in_between() {
+    assert_that!(spans(&[1, 2, 5, 6], 3)).is_equal_to(vec![span(1, 5), span(6, 6)]);
+}
+
+#[test]
+fn spans_of_no_slide_are_empty() {
+    assert_that!(spans(&[], SPAN)).is_empty();
+}
+
+#[rstest]
+#[case(RenderError::timeout(10), ReferenceCause::Timeout)]
+#[case(RenderError::out_of_memory(2048), ReferenceCause::Memory)]
+#[case(RenderError::worker_lost("fermé"), ReferenceCause::Failed)]
+#[case(RenderError::stale(), ReferenceCause::Failed)]
+fn cause_of_a_render_error_names_the_bound(#[case] e: RenderError, #[case] expected: ReferenceCause) {
+    assert_that!(cause(&e)).is_equal_to(expected);
+}
```

Integration tests of the CLI (the hi-fi helper skips the visual gap; the overflow scenario edits the minimal fixture in place):

```diff
--- a/crates/oku-slides-cli/tests/import.rs
+++ b/crates/oku-slides-cli/tests/import.rs
@@ -104,6 +104,53 @@ fn import_minimal_on_essentiel_and_pro_writes_a_reopenable_deck() {
     assert_that!(deck.slides[1].notes.starts_with("Dire bonjour.")).is_true();
 }
 
+/// `pptx` avec la part `name` remplacée par `content`.
+fn replaced(pptx: &Path, name: &str, content: &str) {
+    let bytes = std::fs::read(pptx).unwrap_or_else(|e| panic!("{e}"));
+    let mut src = zip::ZipArchive::new(Cursor::new(bytes)).unwrap_or_else(|e| panic!("zip : {e}"));
+    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
+    for i in 0..src.len() {
+        let entry = src.by_index_raw(i).unwrap_or_else(|e| panic!("zip : {e}"));
+        if entry.name() == name {
+            continue;
+        }
+        w.raw_copy_file(entry).unwrap_or_else(|e| panic!("zip : {e}"));
+    }
+    w.start_file(name, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
+    w.write_all(content.as_bytes()).unwrap_or_else(|e| panic!("zip : {e}"));
+    let out = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
+    std::fs::write(pptx, out).unwrap_or_else(|e| panic!("{e}"));
+}
+
+#[test]
+fn import_retrofit_of_a_text_too_long_for_its_place_reports_an_overflow() {
+    let dir = workdir("import-overflow");
+    let input = pptx(&dir, "minimal");
+    let slide =
+        std::fs::read_to_string(fixtures().join("minimal/ppt/slides/slide3.xml")).unwrap_or_else(|e| panic!("{e}"));
+    let paragraphs: String = (0..80)
+        .map(|i| format!(r#"<a:p><a:r><a:rPr lang="fr-FR"/><a:t>Ligne {i} d'un texte trop long</a:t></a:r></a:p>"#))
+        .collect();
+    let long =
+        slide.replace(r#"<a:p><a:r><a:rPr lang="fr-FR"/><a:t>Texte sans couleur</a:t></a:r></a:p>"#, &paragraphs);
+    assert_that!(long.len()).is_greater_than(slide.len());
+    replaced(&input, "ppt/slides/slide3.xml", &long);
+    let report = dir.join("r.json");
+    let out = run(oku_slides(&dir, "en")
+        .arg("import")
+        .arg(&input)
+        .args(["--pack", "essentiel", "-o"])
+        .arg(dir.join("o.oku-slides.pdf"))
+        .arg("--report")
+        .arg(&report));
+    assert_success("import", &out, &[]);
+    let r: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|e| panic!("{e}")))
+        .unwrap_or_else(|e| panic!("rapport : {e}"));
+    let kinds: Vec<&str> =
+        r["slides"][2]["losses"].as_array().into_iter().flatten().filter_map(|l| l["kind"]["type"].as_str()).collect();
+    assert_that!(kinds).contains("overflow");
+}
+
 #[test]
 fn import_kitchen_on_grilles_writes_its_report_with_coverage_and_messages() {
     let dir = workdir("import-kitchen");
@@ -174,7 +221,8 @@ fn import_hifi(dir: &Path, input: &Path, stem: &str) -> Output {
     run(oku_slides(dir, "en")
         .arg("import")
         .arg(input)
-        .args(["--mode", "hifi", "--no-system-fonts", "-o"])
+        // L'écart visuel a ses propres tests (`import_reference.rs`) : ceux-ci s'en passent.
+        .args(["--mode", "hifi", "--no-system-fonts", "--reference", "none", "-o"])
         .arg(dir.join(format!("{stem}.oku-slides.pdf")))
         .arg("--report")
         .arg(dir.join(format!("{stem}.json"))))
```

Reference tests: recalibrated bounds (alto 1.57 -> 2.0, libreoffice 2.52 -> 3.2), the p. 7 callout now checked among the 3 worst regions (title and callout bar precede it), the timeout test (the timeout is set to 1 ms through the environment) and the refused negative threshold (French):

```diff
--- a/crates/oku-slides-cli/tests/import_reference.rs
+++ b/crates/oku-slides-cli/tests/import_reference.rs
@@ -21,8 +21,14 @@ fn root() -> PathBuf {
 
 /// Importe la fixture `name` en hi-fi, avec `extra` ; la sortie et le rapport.
 fn import(dir: &Path, name: &str, extra: &[&str]) -> (Output, Value) {
+    import_with(dir, name, extra, &[])
+}
+
+/// [`import`], avec les variables d'environnement `env`.
+fn import_with(dir: &Path, name: &str, extra: &[&str], env: &[(&str, &str)]) -> (Output, Value) {
     let report = dir.join("r.json");
     let out = oku_slides(dir, "en")
+        .envs(env.iter().copied())
         .arg("--fonts")
         .arg(root().join("assets/fonts"))
         .arg("import")
@@ -56,8 +62,9 @@ fn max_score(report: &Value) -> f64 {
 }
 
 #[rstest]
-// Mesures du 2026-10-14 (office2pdf 0.8.1), plus 25 % : alto 1,90, libreoffice 2,58.
-#[case("alto-wealth.pptx", 2.4)]
+// Écart moyen de la pire slide, mesuré le 2026-10-14 (office2pdf 0.8.1, σ = 2), plus 25 % :
+// alto 1,57, libreoffice 2,52.
+#[case("alto-wealth.pptx", 2.0)]
 #[case("libreoffice.pptx", 3.2)]
 fn import_hifi_scores_every_slide_under_its_threshold(#[case] name: &str, #[case] threshold: f64) {
     let dir = workdir("import-reference");
@@ -70,8 +77,9 @@ fn import_hifi_scores_every_slide_under_its_threshold(#[case] name: &str, #[case
 fn import_hifi_of_alto_designates_the_cut_line_of_the_callout_on_page_7() {
     let dir = workdir("import-reference-p7");
     let (_, report) = import(dir.as_ref(), "alto-wealth.pptx", &[]);
-    // Le titre le précède : le témoin dessine les titres d'alto plus larges (limite connue).
-    assert_that!(worst(&report, 7)[..2].to_vec()).contains("s7/21".to_owned());
+    // Le titre et le filet de l'encadré le précèdent : le témoin dessine les titres d'alto
+    // plus larges (limite connue).
+    assert_that!(worst(&report, 7)).contains("s7/21".to_owned());
 }
 
 #[test]
@@ -120,3 +128,30 @@ fn import_retrofit_with_reference_is_refused_in_french() {
     assert_that!(out.status.success()).is_false();
     assert_that!(String::from_utf8_lossy(&out.stderr).into_owned()).contains("--reference appartient au mode hifi");
 }
+
+#[test]
+fn import_hifi_whose_reference_times_out_still_writes_the_deck_without_scores() {
+    let dir = workdir("import-reference-timeout");
+    let (out, report) =
+        import_with(dir.as_ref(), "libreoffice.pptx", &[], &[("OKU_SLIDES_IMPORT__REFERENCE_TIMEOUT_MS", "1")]);
+    assert_that!(report["reference"]["unavailable"]["type"].as_str()).is_equal_to(Some("timeout"));
+    let scored = report["slides"].as_array().into_iter().flatten().filter(|s| s.get("score").is_some()).count();
+    assert_that!(scored).is_equal_to(0);
+    let second = String::from_utf8_lossy(&out.stdout).lines().nth(1).unwrap_or_default().to_owned();
+    assert_that!(second.starts_with("Visual gap")).is_false();
+}
+
+#[test]
+fn import_hifi_with_a_negative_threshold_is_refused_in_french() {
+    let dir = workdir("import-reference-threshold");
+    let out = oku_slides(dir.as_ref(), "fr")
+        .env("OKU_SLIDES_IMPORT__SCORE_THRESHOLD", "-1")
+        .arg("import")
+        .arg(root().join("fixtures/import/libreoffice.pptx"))
+        .args(["--mode", "hifi", "-o"])
+        .arg(dir.as_ref().join("out.oku-slides.pdf"))
+        .output()
+        .unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"));
+    assert_that!(out.status.success()).is_false();
+    assert_that!(String::from_utf8_lossy(&out.stderr).into_owned()).contains("doit être un nombre positif");
+}
```

Defaults of the configuration:

```diff
--- a/crates/oku-slides-platform/src/config_tests.rs
+++ b/crates/oku-slides-platform/src/config_tests.rs
@@ -305,7 +305,12 @@ fn render_memory_and_frame_env_overrides_change_the_settings() {
 
 #[test]
 fn settings_default_import_threshold_matches_config_defaults() {
-    assert_that!(Settings::defaults().unwrap().import.score_threshold).is_equal_to(3.0);
+    assert_that!(Settings::defaults().unwrap().import.score_threshold).is_equal_to(10.0);
+}
+
+#[test]
+fn settings_default_reference_timeout_matches_config_defaults() {
+    assert_that!(Settings::defaults().unwrap().import.reference_timeout_ms).is_equal_to(30_000);
 }
 
 #[test]
```

- [ ] **Step 2: Run them to see them fail**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-platform -p oku-slides-cli --no-fail-fast`
Expected: compile errors naming the missing items: `no field reference_timeout_ms on type ImportSettings` (platform tests) and `cannot find function spans in this scope` / `cannot find type SlideSpan` / `cannot find value SPAN` in `import_reference_tests.rs` (the `#[path]` module is declared in Step 3, so the unit tests only compile after it; the integration tests fail on the old threshold and the missing timeout/validation behaviour once the crate builds).

- [ ] **Step 3: Implement**

Configuration: `ImportSettings` is `#[non_exhaustive]`, gains `reference_timeout_ms`, and `score_threshold` is documented as the worst-region threshold.

```diff
--- a/crates/oku-slides-platform/src/config.rs
+++ b/crates/oku-slides-platform/src/config.rs
@@ -110,10 +110,15 @@ pub struct RenderSettings {
 
 /// Import d'une présentation (`oku-slides import`).
 #[derive(Debug, Clone, PartialEq, Deserialize)]
+#[non_exhaustive]
 pub struct ImportSettings {
-    /// Hi-fi : écart visuel (ΔE OKLab ×100, moyen sur la slide) au rendu de
-    /// référence au-delà duquel le rapport signale une slide.
+    /// Hi-fi : écart (ΔE OKLab ×100) d'une région qui fait signaler sa slide.
+    ///
+    /// Comparé à la pire région de chaque slide, contre le rendu de référence ;
+    /// fini et positif (la CLI refuse un autre).
     pub score_threshold: f64,
+    /// Durée maximale d'une tranche du rendu de référence (ms), dans son processus de rendu.
+    pub reference_timeout_ms: u64,
 }
 
 impl SourcesSettings {
```

```diff
--- a/crates/oku-slides-platform/src/config.default.toml
+++ b/crates/oku-slides-platform/src/config.default.toml
@@ -53,10 +53,14 @@ max_widget_kib = 256
 max_widgets = 64
 
 [import]
-# Import hi-fi : écart visuel (ΔE OKLab ×100, moyen sur la slide) au rendu de
-# référence du témoin au-delà duquel le rapport signale une slide. Les fixtures
-# d'import, fidèles, restent sous 2,8 (le témoin dessine sommairement les graphiques).
-score_threshold = 3.0
+# Import hi-fi : écart (ΔE OKLab ×100) de la pire région d'une slide au rendu de
+# référence du témoin au-delà duquel le rapport signale la slide. Mesuré sur alto
+# (office2pdf 0.8.1) : un titre aligné à 1 px près vaut 7,7 à 8,3 (le bruit), les
+# vrais écarts 8,7 à 22,5, les erreurs du témoin jusqu'à 46.
+score_threshold = 10.0
+# Durée maximale d'une tranche de 20 slides du rendu de référence (ms), dans son
+# propre processus de rendu ; au-delà, la référence manque au rapport (« timeout »).
+reference_timeout_ms = 30000
 
 [serve]
 # Serveur de développement (`oku-slides serve`), sans authentification.
```

CLI: dead `DeckWidgets::worker` removed.

```diff
--- a/crates/oku-slides-cli/src/pack_render.rs
+++ b/crates/oku-slides-cli/src/pack_render.rs
@@ -115,11 +115,6 @@ pub(crate) struct DeckWidgets {
 }
 
 impl DeckWidgets {
-    /// Le processus de rendu du pack, s'il en a un.
-    pub(crate) fn worker(&self) -> Option<&RenderWorker> {
-        self.remote.as_ref().map(|(w, _)| w)
-    }
-
     /// Sonde le pack que `deck` référence, pris dans `packs`.
     ///
     /// Sans référence, ou sans widgets : la base. Version absente de `packs`
```

CLI: the threshold is validated before any work in hi-fi mode, the witness gets a worker of its own (timeout from `reference_timeout_ms`, other render limits unchanged), and the result line is printed only when there is a mean.

```diff
--- a/crates/oku-slides-cli/src/import.rs
+++ b/crates/oku-slides-cli/src/import.rs
@@ -31,6 +31,7 @@ use oku_slides_import::retrofit::{self, ColorTokens, PageTokens};
 use oku_slides_import::source::SourceDeck;
 use oku_slides_import::{ImportPlan, ImportReport, ReadLimits, RetrofitTarget, read_pptx};
 use oku_slides_platform::Platform;
+use oku_slides_platform::config::RenderSettings;
 use oku_slides_platform::storage::StorageErrorKind;
 use oku_slides_render::Fonts;
 use oku_slides_render_worker::RenderWorker;
@@ -110,7 +111,13 @@ pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args
         ImportMode::Hifi if args.pack.is_some() || args.theme.is_some() => {
             return Err(eyre!(l.tr("cli-import-hifi-no-target")));
         }
-        ImportMode::Hifi => None,
+        ImportMode::Hifi => {
+            let threshold = platform.settings.import.score_threshold;
+            if !(threshold.is_finite() && threshold >= 0.0) {
+                return Err(eyre!(l.tr_with("cli-import-bad-threshold", &[("value", threshold.to_string().into())])));
+            }
+            None
+        }
         ImportMode::Retrofit if args.reference.is_some() => {
             return Err(eyre!(l.tr("cli-import-reference-hifi-only")));
         }
@@ -207,16 +214,15 @@ fn finish(
     let written = crate::write_rendered(platform, &doc, &bundle, &args.output).wrap_err_with(failed)?;
     note_overflows(doc.layout(), &mut plan.report);
     if args.mode == ImportMode::Hifi && args.reference != Some(ReferenceMode::None) {
-        let own;
-        let worker = match widgets.worker() {
-            Some(w) => w,
-            None => {
-                own = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
-                &own
-            }
+        // Un processus de rendu à lui : le témoin n'ajoute pas sa mémoire à celle du rendu Oku.
+        let settings = RenderSettings {
+            timeout_ms: platform.settings.import.reference_timeout_ms,
+            ..platform.settings.render.clone()
         };
+        let worker = RenderWorker::new(platform.render_launch.clone(), settings);
         let threshold = platform.settings.import.score_threshold;
-        let scoring = Scoring { pptx: &input.bytes, source: &input.source, fonts: &fonts, worker, threshold };
+        let scoring =
+            Scoring { pptx: &input.bytes, source: &input.source, fonts: &fonts, worker: &worker, threshold, l };
         import_reference::score(&scoring, &doc, &mut plan.report);
     }
     if let Some(path) = &args.report {
@@ -238,20 +244,23 @@ fn finish(
     Ok(())
 }
 
-/// La ligne de l'écart visuel : moyenne et slides au-delà du seuil, ou pourquoi la référence manque.
+/// La ligne de l'écart visuel : moyenne et slides au-delà du seuil (par leur pire
+/// région), ou pourquoi la référence manque.
 fn reference_line(out: &mut Out<'_>, reference: &ReferenceReport, report: &ImportReport) -> Result<()> {
     let l = out.localizer();
     match (&reference.unavailable, report.mean_score()) {
         (Some(cause), _) => out.line("cli-import-unscored", &[("reason", cause.localize(l).into())])?,
-        (None, mean) => out.line(
+        (None, Some(mean)) => out.line(
             "cli-import-scored",
             &[
                 ("witness", reference.witness.as_str().into()),
-                ("mean", mean.unwrap_or_default().into()),
+                ("mean", mean.into()),
                 ("above", report.above_threshold().count().into()),
                 ("threshold", reference.threshold.into()),
             ],
         )?,
+        // Une présentation sans slide : rien à dire.
+        (None, None) => {}
     }
     Ok(())
 }
```

CLI: slices of `SPAN` slides, witness pages paired by source number, every failure is a `Failure` (cause for the report, detail for the log), and the unit-test module is declared.

```diff
--- a/crates/oku-slides-cli/src/import_reference.rs
+++ b/crates/oku-slides-cli/src/import_reference.rs
@@ -1,24 +1,33 @@
-//! L'écart visuel d'un import hi-fi (`--reference auto`, le défaut en hi-fi) :
-//! le témoin rend la présentation dans le processus de rendu
-//! ([`RenderWorker::reference`]), puis chaque slide Oku, rognée à la page
-//! source ([`hifi::source_rect`]), est comparée à sa page de référence à
-//! [`REFERENCE_WIDTH`] pixels de large ([`metrics::score`], régions des items
-//! source). Une référence absente, en échec ou d'un autre nombre de pages
-//! n'est jamais une erreur : le rapport dit pourquoi (`ReferenceCause`).
+//! L'écart visuel d'un import hi-fi (`--reference auto`, le défaut en hi-fi).
+//!
+//! Le témoin rend la présentation dans un processus de rendu à lui
+//! ([`RenderWorker::reference`], borné par `[import] reference_timeout_ms`),
+//! par tranches de [`SPAN`] slides : sa mémoire ne croît pas avec le deck.
+//! Chaque slide Oku, rognée à la page source ([`hifi::source_rect`]), est
+//! comparée à sa page de référence à [`REFERENCE_WIDTH`] pixels de large
+//! ([`metrics::score`], régions des items source) ; les slides se répondent
+//! par leur numéro source. Une référence en échec, d'un autre nombre de pages
+//! ou une slide qu'on ne peut noter n'est jamais une erreur : le rapport dit
+//! pourquoi (`ReferenceCause`), le journal dit le détail, et aucune slide ne
+//! garde de score.
 
+use oku_slides_i18n::{Localize, Localizer};
 use oku_slides_import::metrics::{self, Image};
 use oku_slides_import::report::{ReferenceCause, ReferenceReport};
 use oku_slides_import::source::SourceDeck;
 use oku_slides_import::{ImportReport, hifi};
-use oku_slides_reference::{Raster, WITNESS};
+use oku_slides_reference::{Raster, SlideSpan, WITNESS};
 use oku_slides_render::{Fonts, RenderError, RenderErrorKind};
-use oku_slides_render_worker::{RemoteReference, RenderWorker};
+use oku_slides_render_worker::RenderWorker;
 
 use crate::pack_render::Rendered;
 
 /// Largeur, en pixels, de la page source comparée (spec : 960 px).
 pub(crate) const REFERENCE_WIDTH: f64 = 960.0;
 
+/// Slides rendues par le témoin en une requête, au plus.
+pub(crate) const SPAN: usize = 20;
+
 /// Ce qu'il faut pour mesurer l'écart d'un import hi-fi.
 pub(crate) struct Scoring<'a> {
     /// Les octets du `.pptx`.
@@ -27,25 +36,23 @@ pub(crate) struct Scoring<'a> {
     pub(crate) source: &'a SourceDeck,
     /// Les polices du rendu Oku (celles de l'app et celles du deck), données au témoin.
     pub(crate) fonts: &'a Fonts,
-    /// Le processus de rendu.
+    /// Le processus de rendu du témoin, distinct de celui du rendu Oku.
     pub(crate) worker: &'a RenderWorker,
     /// Le seuil du rapport (`[import] score_threshold`).
     pub(crate) threshold: f64,
+    /// La langue du journal.
+    pub(crate) l: &'a Localizer,
 }
 
+/// Un échec : sa cause au rapport et son détail au journal.
+type Failure = (ReferenceCause, String);
+
 /// Pose dans `report` la référence et l'écart de chaque slide de `doc`.
 pub(crate) fn score(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport) {
-    let result = s.worker.reference(s.pptx, s.fonts, None).map_err(|e| cause(&e)).and_then(|reference| {
-        let (pages, deck) = (reference.page_count(), doc.page_count());
-        if pages != deck || pages != report.slides.len() {
-            let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
-            return Err(ReferenceCause::Pages { reference: count(pages), deck: count(deck) });
-        }
-        scores(s, &reference, doc, report).map(|()| reference.witness().to_owned())
-    });
-    let (witness, unavailable) = match result {
+    let (witness, unavailable) = match scores(s, doc, report) {
         Ok(witness) => (witness, None),
-        Err(cause) => {
+        Err((cause, detail)) => {
+            tracing::warn!(error = %detail, "{}", cause.localize(s.l));
             for slide in &mut report.slides {
                 slide.score = None;
             }
@@ -55,31 +62,52 @@ pub(crate) fn score(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport)
     report.reference = Some(ReferenceReport { witness, threshold: s.threshold, unavailable });
 }
 
-/// L'écart de chaque slide : sa page Oku rognée à la page source contre sa page de référence.
-fn scores(
-    s: &Scoring<'_>,
-    reference: &RemoteReference,
-    doc: &Rendered,
-    report: &mut ImportReport,
-) -> Result<(), ReferenceCause> {
+/// Les tranches de numéros source à rendre : [`SPAN`] slides du rapport au
+/// plus chacune, de la première à la dernière (les slides que l'import a
+/// omises entre deux comptent dans la tranche, le témoin les rend aussi).
+pub(crate) fn spans(numbers: &[u32], size: usize) -> Vec<SlideSpan> {
+    numbers.chunks(size.max(1)).filter_map(|c| Some(SlideSpan { first: *c.first()?, last: *c.last()? })).collect()
+}
+
+/// L'écart de chaque slide, tranche par tranche ; le nom du témoin.
+fn scores(s: &Scoring<'_>, doc: &Rendered, report: &mut ImportReport) -> Result<String, Failure> {
     let rect = hifi::source_rect(s.source);
     // Pixels par pt de la page Oku, pour que la page source y fasse REFERENCE_WIDTH pixels.
     let ppp = REFERENCE_WIDTH / rect.w;
     let px = |v: f64| (v * ppp).round().max(0.0) as u32;
     let width = px(rect.w);
     let scale = f64::from(width) / s.source.size.w;
-    for (index, (slide, src)) in report.slides.iter_mut().zip(&s.source.slides).enumerate() {
-        let page = doc.layout().page_of(&slide.slide).map_or(index, |p| p.saturating_sub(1));
-        let oku =
-            Raster::from_png(&doc.png(page, ppp as f32).map_err(|e| cause(&e))?).map_err(|_| ReferenceCause::Failed)?;
-        let refr = Raster::from_png(&reference.png(index, width).map_err(|e| cause(&e))?)
-            .map_err(|_| ReferenceCause::Failed)?;
-        let height = refr.height().min(px(rect.h));
-        let oku = oku.crop(px(rect.x), px(rect.y), width, height);
-        let refr = refr.crop(0, 0, width, height);
-        slide.score = metrics::score(image(&refr), image(&oku), &metrics::regions(src, scale));
+    let numbers: Vec<u32> = report.slides.iter().map(|r| r.source).collect();
+    let mut witness = WITNESS.to_owned();
+    for span in spans(&numbers, SPAN) {
+        let reference = s.worker.reference(s.pptx, s.fonts, Some(span)).map_err(|e| render(&e, "témoin"))?;
+        let expected = (span.last - span.first + 1) as usize;
+        if reference.page_count() != expected {
+            let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
+            let cause = ReferenceCause::Pages { reference: count(reference.page_count()), deck: count(expected) };
+            return Err((cause, format!("slides {}..={}", span.first, span.last)));
+        }
+        witness = reference.witness().to_owned();
+        for slide in report.slides.iter_mut().filter(|r| (span.first..=span.last).contains(&r.source)) {
+            let failed = |what: &str| (ReferenceCause::Failed, format!("slide {} : {what}", slide.source));
+            let src = s.source.slides.iter().find(|x| x.number == slide.source).ok_or_else(|| failed("source"))?;
+            let page = doc.layout().page_of(&slide.slide).and_then(|p| p.checked_sub(1));
+            let page = page.ok_or_else(|| failed("page Oku absente du rendu"))?;
+            let oku = doc.png(page, ppp as f32).map_err(|e| render(&e, "page Oku"))?;
+            let oku = Raster::from_png(&oku).map_err(|e| (ReferenceCause::Failed, format!("page Oku : {e}")))?;
+            let at = (slide.source - span.first) as usize;
+            let refr = reference.png(at, width).map_err(|e| render(&e, "page du témoin"))?;
+            let refr =
+                Raster::from_png(&refr).map_err(|e| (ReferenceCause::Failed, format!("page du témoin : {e}")))?;
+            // Rognées à leur taille commune : un arrondi d'un pixel ne prive pas la slide de son score.
+            let w = width.min(refr.width()).min(oku.width().saturating_sub(px(rect.x)));
+            let h = px(rect.h).min(refr.height()).min(oku.height().saturating_sub(px(rect.y)));
+            let (oku, refr) = (oku.crop(px(rect.x), px(rect.y), w, h), refr.crop(0, 0, w, h));
+            let scored = metrics::score(image(&refr), image(&oku), &metrics::regions(src, scale));
+            slide.score = Some(scored.ok_or_else(|| failed("images vides"))?);
+        }
     }
-    Ok(())
+    Ok(witness)
 }
 
 /// La vue de `r` que lit [`metrics::score`].
@@ -87,6 +115,11 @@ fn image(r: &Raster) -> Image<'_> {
     Image { width: r.width(), height: r.height(), rgba: r.rgba() }
 }
 
+/// Un échec du processus de rendu, `what` en disant l'objet.
+fn render(e: &RenderError, what: &str) -> Failure {
+    (cause(e), format!("{what} : {e}"))
+}
+
 /// Pourquoi un échec du processus de rendu prive l'import de sa référence.
 fn cause(e: &RenderError) -> ReferenceCause {
     match e.kind() {
@@ -95,3 +128,7 @@ fn cause(e: &RenderError) -> ReferenceCause {
         _ => ReferenceCause::Failed,
     }
 }
+
+#[cfg(test)]
+#[path = "import_reference_tests.rs"]
+mod import_reference_tests;
```

Translations: the English file first, then the 19 others (the ko hunk also fixes `cli-import-scored`: `{ $threshold }을 넘는`, no space between the number and the Hangul).

```diff
--- a/crates/oku-slides-i18n/src/locales/en/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/en/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack and --theme belong to retrofit mode: the high
 cli-import-probe-mismatch = the generated pack does not probe as declared (widget { $widget }): this is a bug in the importer
 cli-import-probe-limit = the generated pack exceeds the probe limits (widget { $widget }): raise max_widgets, max_widget_kib or probe_budget_ms in the [render] section of the configuration
 cli-import-reference-hifi-only = --reference belongs to hifi mode: a retrofit import is not compared with a reference render
+cli-import-bad-threshold = score_threshold in the [import] section must be a positive number, not { $value }
 cli-import-scored = Visual gap to { $witness }: { $mean } on average, { $above ->
     [0] no slide
     [one] { $above } slide
```

```diff
--- a/crates/oku-slides-i18n/src/locales/ar/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ar/cli.ftl
@@ -70,6 +70,7 @@ cli-import-hifi-no-target = --pack و--theme خاصّان بوضع retrofit: ا
 cli-import-probe-mismatch = الحزمة المولَّدة لا تُفحص كما أُعلنت (الأداة { $widget }): هذا خلل في المستورِد
 cli-import-probe-limit = تتجاوز الحزمة المولَّدة حدود الفحص (الأداة { $widget }): ارفع max_widgets أو max_widget_kib أو probe_budget_ms في القسم [render] من الإعدادات
 cli-import-reference-hifi-only = ‎--reference خاص بوضع hifi: لا يُقارَن استيراد retrofit بعرض مرجعي
+cli-import-bad-threshold = يجب أن يكون score_threshold في القسم [import] عددًا موجبًا، لا { $value }
 cli-import-scored = الفرق المرئي عن { $witness }: { $mean } في المتوسط، { $above ->
     [0] لا توجد شرائح
     [one] { $above } شريحة
```

```diff
--- a/crates/oku-slides-i18n/src/locales/bn/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/bn/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack ও --theme শুধু retrofit মোড
 cli-import-probe-mismatch = তৈরি করা প্যাক ঘোষণা অনুযায়ী যাচাই হয়নি (উইজেট { $widget }): এটি ইমপোর্টারের ত্রুটি
 cli-import-probe-limit = তৈরি প্যাকটি যাচাইয়ের সীমা ছাড়িয়েছে (উইজেট { $widget }): কনফিগারেশনের [render] অংশে max_widgets, max_widget_kib বা probe_budget_ms বাড়ান
 cli-import-reference-hifi-only = --reference hifi মোডের অংশ: retrofit আমদানি রেফারেন্স রেন্ডারের সঙ্গে তুলনা করা হয় না
+cli-import-bad-threshold = [import] অংশের score_threshold একটি ধনাত্মক সংখ্যা হতে হবে, { $value } নয়
 cli-import-scored = { $witness }-এর সঙ্গে দৃশ্যমান পার্থক্য: গড়ে { $mean }, { $threshold }-এর উপরে { $above ->
     [0] কোনো স্লাইড নেই
     [one] { $above }টি স্লাইড
```

```diff
--- a/crates/oku-slides-i18n/src/locales/de/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/de/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack und --theme gelten nur im Modus retrofit: der
 cli-import-probe-mismatch = das erzeugte Paket entspricht bei der Prüfung nicht seiner Deklaration (Widget { $widget }): ein Fehler des Imports
 cli-import-probe-limit = das erzeugte Paket überschreitet die Grenzen der Prüfung (Widget { $widget }): erhöhen Sie max_widgets, max_widget_kib oder probe_budget_ms im Abschnitt [render] der Konfiguration
 cli-import-reference-hifi-only = --reference gehört zum hifi-Modus: ein Retrofit-Import wird nicht mit einem Referenz-Rendering verglichen
+cli-import-bad-threshold = score_threshold im Abschnitt [import] muss eine positive Zahl sein, nicht { $value }
 cli-import-scored = Visuelle Abweichung zu { $witness }: { $mean } im Mittel, { $above ->
     [0] keine Folie
     [one] { $above } Folie
```

```diff
--- a/crates/oku-slides-i18n/src/locales/es/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/es/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack y --theme son del modo retrofit: la importaci
 cli-import-probe-mismatch = el paquete generado no se sondea como se declaró (widget { $widget }): es un error del importador
 cli-import-probe-limit = el paquete generado supera los límites de la sonda (widget { $widget }): aumente max_widgets, max_widget_kib o probe_budget_ms en la sección [render] de la configuración
 cli-import-reference-hifi-only = --reference pertenece al modo hifi: una importación retrofit no se compara con un renderizado de referencia
+cli-import-bad-threshold = score_threshold de la sección [import] debe ser un número positivo, no { $value }
 cli-import-scored = Diferencia visual con { $witness }: { $mean } de media, { $above ->
     [0] ninguna diapositiva
     [one] { $above } diapositiva
```

```diff
--- a/crates/oku-slides-i18n/src/locales/fr/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/fr/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack et --theme sont propres au mode retrofit : l'
 cli-import-probe-mismatch = le pack généré ne se sonde pas comme il est déclaré (widget { $widget }) : c'est un bogue de l'import
 cli-import-probe-limit = le pack généré dépasse les bornes de la sonde (widget { $widget }) : relevez max_widgets, max_widget_kib ou probe_budget_ms dans la section [render] de la configuration
 cli-import-reference-hifi-only = --reference appartient au mode hifi : un import retrofit n'est pas comparé à un rendu de référence
+cli-import-bad-threshold = score_threshold de la section [import] doit être un nombre positif, pas { $value }
 cli-import-scored = Écart visuel à { $witness } : { $mean } en moyenne, { $above ->
     [0] aucune slide
     [one] { $above } slide
```

```diff
--- a/crates/oku-slides-i18n/src/locales/hi/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/hi/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack और --theme केवल retrofit मो
 cli-import-probe-mismatch = बनाया गया पैक घोषणा के अनुसार जाँच में नहीं मिला (विजेट { $widget }): यह आयातक की त्रुटि है
 cli-import-probe-limit = बनाया गया पैक जाँच की सीमाएँ पार करता है (विजेट { $widget }): कॉन्फ़िगरेशन के [render] खंड में max_widgets, max_widget_kib या probe_budget_ms बढ़ाएँ
 cli-import-reference-hifi-only = --reference hifi मोड का हिस्सा है: retrofit आयात की तुलना संदर्भ रेंडर से नहीं होती
+cli-import-bad-threshold = [import] अनुभाग का score_threshold एक धनात्मक संख्या होना चाहिए, { $value } नहीं
 cli-import-scored = { $witness } से दृश्य अंतर: औसत { $mean }, { $threshold } से ऊपर { $above ->
     [0] कोई स्लाइड नहीं
     [one] { $above } स्लाइड
```

```diff
--- a/crates/oku-slides-i18n/src/locales/it/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/it/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack e --theme valgono solo in modalità retrofit:
 cli-import-probe-mismatch = il pacchetto generato non supera la sonda come dichiarato (widget { $widget }): è un errore dell'importazione
 cli-import-probe-limit = il pacchetto generato supera i limiti della sonda (widget { $widget }): aumenta max_widgets, max_widget_kib o probe_budget_ms nella sezione [render] della configurazione
 cli-import-reference-hifi-only = --reference appartiene alla modalità hifi: un'importazione retrofit non è confrontata con un rendering di riferimento
+cli-import-bad-threshold = score_threshold della sezione [import] deve essere un numero positivo, non { $value }
 cli-import-scored = Scarto visivo da { $witness }: { $mean } in media, { $above ->
     [0] nessuna slide
     [one] { $above } slide
```

```diff
--- a/crates/oku-slides-i18n/src/locales/ja/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ja/cli.ftl
@@ -42,6 +42,7 @@ cli-import-hifi-no-target = --pack と --theme は retrofit モード専用で
 cli-import-probe-mismatch = 生成されたパックが宣言どおりに検査されませんでした（ウィジェット { $widget }）。インポーターの不具合です
 cli-import-probe-limit = 生成したパックが検査の上限を超えています（ウィジェット { $widget }）：設定の [render] セクションで max_widgets、max_widget_kib、probe_budget_ms を引き上げてください
 cli-import-reference-hifi-only = --reference は hifi モード用です：retrofit インポートは参照レンダリングと比較されません
+cli-import-bad-threshold = [import] セクションの score_threshold は正の数である必要があります（{ $value } ではありません）
 cli-import-scored = { $witness } との見た目の差：平均 { $mean }、{ $threshold } を超えるスライド { $above ->
    *[other] { $above }枚
 }
```

```diff
--- a/crates/oku-slides-i18n/src/locales/ko/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ko/cli.ftl
@@ -42,7 +42,8 @@ cli-import-hifi-no-target = --pack과 --theme는 retrofit 모드 전용입니다
 cli-import-probe-mismatch = 생성된 팩이 선언대로 검사되지 않았습니다(위젯 { $widget }): 가져오기 기능의 버그입니다
 cli-import-probe-limit = 생성된 팩이 검사 한도를 넘습니다(위젯 { $widget }): 설정의 [render] 섹션에서 max_widgets, max_widget_kib 또는 probe_budget_ms를 늘리세요
 cli-import-reference-hifi-only = --reference는 hifi 모드용입니다: retrofit 가져오기는 참조 렌더링과 비교하지 않습니다
-cli-import-scored = { $witness }와의 시각 차이: 평균 { $mean }, { $threshold } 초과 슬라이드 { $above ->
+cli-import-bad-threshold = [import] 섹션의 score_threshold는 양수여야 합니다({ $value } 아님)
+cli-import-scored = { $witness }와의 시각 차이: 평균 { $mean }, { $threshold }을 넘는 슬라이드 { $above ->
    *[other] { $above }개
 }
 cli-import-unscored = { $reason }
```

```diff
--- a/crates/oku-slides-i18n/src/locales/nl/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/nl/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack en --theme horen bij de modus retrofit: impor
 cli-import-probe-mismatch = het gemaakte pakket komt bij het peilen niet overeen met de declaratie (widget { $widget }): een fout in de import
 cli-import-probe-limit = het gegenereerde pakket overschrijdt de grenzen van de controle (widget { $widget }): verhoog max_widgets, max_widget_kib of probe_budget_ms in de sectie [render] van de configuratie
 cli-import-reference-hifi-only = --reference hoort bij de hifi-modus: een retrofit-import wordt niet met een referentieweergave vergeleken
+cli-import-bad-threshold = score_threshold in de sectie [import] moet een positief getal zijn, niet { $value }
 cli-import-scored = Visueel verschil met { $witness }: gemiddeld { $mean }, { $above ->
     [0] geen dia
     [one] { $above } dia
```

```diff
--- a/crates/oku-slides-i18n/src/locales/pl/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/pl/cli.ftl
@@ -62,6 +62,7 @@ cli-import-hifi-no-target = --pack i --theme dotyczą tylko trybu retrofit: impo
 cli-import-probe-mismatch = wygenerowany pakiet nie odpowiada swojej deklaracji przy sprawdzaniu (widżet { $widget }): to błąd importu
 cli-import-probe-limit = wygenerowany pakiet przekracza limity sondy (widżet { $widget }): zwiększ max_widgets, max_widget_kib lub probe_budget_ms w sekcji [render] konfiguracji
 cli-import-reference-hifi-only = --reference należy do trybu hifi: import retrofit nie jest porównywany z renderowaniem referencyjnym
+cli-import-bad-threshold = score_threshold w sekcji [import] musi być liczbą dodatnią, a nie { $value }
 cli-import-scored = Różnica wizualna względem { $witness }: średnio { $mean }, { $above ->
     [0] żaden slajd
     [one] { $above } slajd
```

```diff
--- a/crates/oku-slides-i18n/src/locales/pt/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/pt/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack e --theme são do modo retrofit: a importaç
 cli-import-probe-mismatch = o pacote gerado não corresponde à sondagem como declarado (widget { $widget }): é um erro do importador
 cli-import-probe-limit = o pacote gerado excede os limites da sonda (widget { $widget }): aumente max_widgets, max_widget_kib ou probe_budget_ms na seção [render] da configuração
 cli-import-reference-hifi-only = --reference pertence ao modo hifi: uma importação retrofit não é comparada com uma renderização de referência
+cli-import-bad-threshold = score_threshold da seção [import] deve ser um número positivo, não { $value }
 cli-import-scored = Diferença visual para { $witness }: { $mean } em média, { $above ->
     [0] nenhum slide
     [one] { $above } slide
```

```diff
--- a/crates/oku-slides-i18n/src/locales/ru/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/ru/cli.ftl
@@ -62,6 +62,7 @@ cli-import-hifi-no-target = --pack и --theme относятся к режиму
 cli-import-probe-mismatch = созданный пакет не прошёл проверку в соответствии с объявлением (виджет { $widget }): это ошибка импорта
 cli-import-probe-limit = созданный пакет превышает ограничения проверки (виджет { $widget }): увеличьте max_widgets, max_widget_kib или probe_budget_ms в разделе [render] конфигурации
 cli-import-reference-hifi-only = --reference относится к режиму hifi: импорт retrofit не сравнивается с эталонным рендерингом
+cli-import-bad-threshold = score_threshold в разделе [import] должен быть положительным числом, а не { $value }
 cli-import-scored = Визуальное отличие от { $witness }: в среднем { $mean }, { $above ->
     [0] ни одного слайда
     [one] { $above } слайд
```

```diff
--- a/crates/oku-slides-i18n/src/locales/th/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/th/cli.ftl
@@ -42,6 +42,7 @@ cli-import-hifi-no-target = --pack และ --theme ใช้กับโหม
 cli-import-probe-mismatch = แพ็กที่สร้างขึ้นตรวจสอบได้ไม่ตรงกับที่ประกาศ (วิดเจ็ต { $widget }): เป็นข้อบกพร่องของตัวนำเข้า
 cli-import-probe-limit = แพ็กที่สร้างเกินขีดจำกัดของการตรวจ (วิดเจ็ต { $widget }): เพิ่ม max_widgets, max_widget_kib หรือ probe_budget_ms ในส่วน [render] ของการตั้งค่า
 cli-import-reference-hifi-only = --reference เป็นของโหมด hifi: การนำเข้าแบบ retrofit ไม่ถูกเปรียบเทียบกับการเรนเดอร์อ้างอิง
+cli-import-bad-threshold = score_threshold ในส่วน [import] ต้องเป็นจำนวนบวก ไม่ใช่ { $value }
 cli-import-scored = ความต่างทางภาพจาก { $witness }: เฉลี่ย { $mean }, สไลด์ที่เกิน { $threshold } { $above ->
    *[other] { $above } แผ่น
 }
```

```diff
--- a/crates/oku-slides-i18n/src/locales/tr/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/tr/cli.ftl
@@ -54,6 +54,7 @@ cli-import-hifi-no-target = --pack ve --theme yalnızca retrofit modu içindir:
 cli-import-probe-mismatch = üretilen paket bildirildiği gibi yoklanmadı (widget { $widget }): içe aktarıcıda bir hata
 cli-import-probe-limit = oluşturulan paket denetim sınırlarını aşıyor (widget { $widget }): yapılandırmanın [render] bölümünde max_widgets, max_widget_kib veya probe_budget_ms değerini artırın
 cli-import-reference-hifi-only = --reference hifi moduna aittir: retrofit içe aktarma bir referans oluşturmayla karşılaştırılmaz
+cli-import-bad-threshold = [import] bölümündeki score_threshold pozitif bir sayı olmalı, { $value } değil
 cli-import-scored = { $witness } ile görsel fark: ortalama { $mean }, { $threshold } üzerinde { $above ->
     [0] hiç slayt yok
    *[other] { $above } slayt
```

```diff
--- a/crates/oku-slides-i18n/src/locales/uk/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/uk/cli.ftl
@@ -62,6 +62,7 @@ cli-import-hifi-no-target = --pack і --theme стосуються режиму
 cli-import-probe-mismatch = створений пакет не пройшов перевірку відповідно до оголошення (віджет { $widget }): це помилка імпорту
 cli-import-probe-limit = створений пакет перевищує обмеження перевірки (віджет { $widget }): збільште max_widgets, max_widget_kib або probe_budget_ms у розділі [render] конфігурації
 cli-import-reference-hifi-only = --reference належить до режиму hifi: імпорт retrofit не порівнюється з еталонним рендерингом
+cli-import-bad-threshold = score_threshold у розділі [import] має бути додатним числом, а не { $value }
 cli-import-scored = Візуальна відмінність від { $witness }: у середньому { $mean }, { $above ->
     [0] жодного слайда
     [one] { $above } слайд
```

```diff
--- a/crates/oku-slides-i18n/src/locales/vi/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/vi/cli.ftl
@@ -42,6 +42,7 @@ cli-import-hifi-no-target = --pack và --theme chỉ dùng cho chế độ retro
 cli-import-probe-mismatch = gói được tạo không khớp với khai báo khi kiểm tra (widget { $widget }): đây là lỗi của trình nhập
 cli-import-probe-limit = gói được tạo vượt giới hạn kiểm tra (widget { $widget }): hãy tăng max_widgets, max_widget_kib hoặc probe_budget_ms trong mục [render] của cấu hình
 cli-import-reference-hifi-only = --reference thuộc chế độ hifi: bản nhập retrofit không được so sánh với bản kết xuất tham chiếu
+cli-import-bad-threshold = score_threshold trong mục [import] phải là một số dương, không phải { $value }
 cli-import-scored = Khác biệt hình ảnh so với { $witness }: trung bình { $mean }, { $above ->
     [0] không trang chiếu nào
    *[other] { $above } trang chiếu
```

```diff
--- a/crates/oku-slides-i18n/src/locales/zh-Hans/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/zh-Hans/cli.ftl
@@ -42,6 +42,7 @@ cli-import-hifi-no-target = --pack 和 --theme 仅用于 retrofit 模式：高
 cli-import-probe-mismatch = 生成的模板包探测结果与声明不符（组件 { $widget }）：这是导入程序的缺陷
 cli-import-probe-limit = 生成的包超出检查限制（组件 { $widget }）：请在配置的 [render] 部分提高 max_widgets、max_widget_kib 或 probe_budget_ms
 cli-import-reference-hifi-only = --reference 属于 hifi 模式：retrofit 导入不与参考渲染比较
+cli-import-bad-threshold = [import] 部分的 score_threshold 必须是正数，而不是 { $value }
 cli-import-scored = 与 { $witness } 的视觉差异：平均 { $mean }，超过 { $threshold } 的幻灯片 { $above ->
    *[other] { $above }张
 }
```

```diff
--- a/crates/oku-slides-i18n/src/locales/zh-Hant/cli.ftl
+++ b/crates/oku-slides-i18n/src/locales/zh-Hant/cli.ftl
@@ -42,6 +42,7 @@ cli-import-hifi-no-target = --pack 與 --theme 僅用於 retrofit 模式：高
 cli-import-probe-mismatch = 產生的範本套件探測結果與宣告不符（元件 { $widget }）：這是匯入程式的缺陷
 cli-import-probe-limit = 產生的套件超出檢查限制（元件 { $widget }）：請在設定的 [render] 區段提高 max_widgets、max_widget_kib 或 probe_budget_ms
 cli-import-reference-hifi-only = --reference 屬於 hifi 模式：retrofit 匯入不與參考轉譯比較
+cli-import-bad-threshold = [import] 區段的 score_threshold 必須是正數，而不是 { $value }
 cli-import-scored = 與 { $witness } 的視覺差異：平均 { $mean }，超過 { $threshold } 的投影片 { $above ->
    *[other] { $above }張
 }
```

- [ ] **Step 4: Run the tests and the checks**
Run: `touch crates/*/src/lib.rs && CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo nextest run -p oku-slides-platform -p oku-slides-i18n -p oku-slides-cli`
Run: `CARGO_TARGET_DIR=/Users/matthieu/Projects/slider/target cargo clippy -p oku-slides-platform -p oku-slides-i18n -p oku-slides-cli --all-targets -- -D warnings`
Run: `cargo fmt -p oku-slides-platform -p oku-slides-i18n -p oku-slides-cli -- --check`
Expected: all tests pass (the new ones: `spans_*` x3, `cause_of_a_render_error_names_the_bound` x4 cases, the two default-config tests, `import_retrofit_of_a_text_too_long_for_its_place_reports_an_overflow`, `import_hifi_whose_reference_times_out_still_writes_the_deck_without_scores`, `import_hifi_with_a_negative_threshold_is_refused_in_french`, and the recalibrated `import_hifi_scores_every_slide_under_its_threshold` and p. 7 test); clippy and fmt clean. The reference tests need the witness (office2pdf 0.8.1) and the fonts of `assets/fonts` (Git LFS). Any manual run of the `oku-slides` binary sets `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh `mktemp -d` directory (the tests do it through `workdir`).

- [ ] **Step 5: Commit**
```bash
git add crates/oku-slides-cli/src/import.rs crates/oku-slides-cli/src/import_reference.rs crates/oku-slides-cli/src/import_reference_tests.rs crates/oku-slides-cli/src/pack_render.rs crates/oku-slides-cli/tests/import.rs crates/oku-slides-cli/tests/import_reference.rs crates/oku-slides-platform/src/config.default.toml crates/oku-slides-platform/src/config.rs crates/oku-slides-platform/src/config_tests.rs crates/oku-slides-i18n/src/locales/ar/cli.ftl crates/oku-slides-i18n/src/locales/bn/cli.ftl crates/oku-slides-i18n/src/locales/de/cli.ftl crates/oku-slides-i18n/src/locales/en/cli.ftl crates/oku-slides-i18n/src/locales/es/cli.ftl crates/oku-slides-i18n/src/locales/fr/cli.ftl crates/oku-slides-i18n/src/locales/hi/cli.ftl crates/oku-slides-i18n/src/locales/it/cli.ftl crates/oku-slides-i18n/src/locales/ja/cli.ftl crates/oku-slides-i18n/src/locales/ko/cli.ftl crates/oku-slides-i18n/src/locales/nl/cli.ftl crates/oku-slides-i18n/src/locales/pl/cli.ftl crates/oku-slides-i18n/src/locales/pt/cli.ftl crates/oku-slides-i18n/src/locales/ru/cli.ftl crates/oku-slides-i18n/src/locales/th/cli.ftl crates/oku-slides-i18n/src/locales/tr/cli.ftl crates/oku-slides-i18n/src/locales/uk/cli.ftl crates/oku-slides-i18n/src/locales/vi/cli.ftl crates/oku-slides-i18n/src/locales/zh-Hans/cli.ftl crates/oku-slides-i18n/src/locales/zh-Hant/cli.ftl
```
Write the message to a file (for example `/tmp/commit-msg-t10.txt`) containing exactly this subject, then commit with it:
```
fix(cli) : référence dans un processus de rendu à elle, par tranches de 20 slides, sans repli silencieux ; seuil sur la pire région, validé ; débordement testé de bout en bout (t-692)
```
```bash
git commit -F /tmp/commit-msg-t10.txt
```

**Extracting from the draft:** the implementer may take the files of this task from the draft commit instead of retyping them: `git checkout a5546575 -- <file> …` with the explicit list from `git diff --name-only a5546575~1 a5546575` (the 29 files named in Step 5, never a directory). None of these files changed on `main` since the draft base `53454df2` (checked against `git diff --name-only 53454df2 main`), so a plain checkout is safe; were one to change, apply it instead with `git show a5546575 -- <file> | git apply --3way`.

### Task 11: Documents after the plan review: decisions, ARCHITECTURE.md, AGENTS.md, README

Documents only, no code. This task records what the plan review changed: the witness is pinned and its fonts stripped, it runs in a render worker of its own on 20-slide spans, the blur is σ = 2, a slide is flagged on its worst region (threshold 10.0, with the alto measurements), the witness limits found by the real render, and the app dependency cost deferred to lot 5. It updates the spec's lot 4 decisions, the four `ARCHITECTURE.md` files (the import one gets back its t-816 line), the reference crate `AGENTS.md` rules, and adds a README paragraph on `--reference`.

The diffs below are taken against the draft parent (`git show f96e0ef2`) and apply on top of the English documents written by Task 7 (and on top of `main`'s English `ARCHITECTURE.md` files). If a hunk does not apply cleanly because of a wording difference, apply it by hand: the intent of each hunk is given in its introduction.

**Files:**
- Modify: `README.md` (paragraph on `--reference` in hifi mode)
- Modify: `crates/oku-slides-cli/ARCHITECTURE.md` (`import_reference.rs` line)
- Modify: `crates/oku-slides-import/ARCHITECTURE.md` (shorter `metrics` line, t-816 known limit)
- Modify: `crates/oku-slides-reference/AGENTS.md` (own render worker, letter-spaced titles)
- Modify: `crates/oku-slides-reference/ARCHITECTURE.md` (span argument, `WITNESS`, font stripping, limits, cost)
- Modify: `crates/oku-slides-render-worker/ARCHITECTURE.md` (`RenderWorker::reference` with slides)
- Modify: `docs/superpowers/specs/2026-10-09-import-pptx-design.md` (lot 4 decisions 1-7, CLI output line)

**Interfaces:**
- Consumes: nothing new (the documents describe `ReferenceRenderer::render(pptx, fonts, slides: Option<SlideSpan>)`, `WITNESS`, `RenderWorker::reference(pptx, fonts, slides)` from earlier tasks).
- Produces: nothing new.

- [ ] **Step 1: Apply the diffs**

README: the `--reference` paragraph.

```diff
--- a/README.md
+++ b/README.md
@@ -75,6 +75,14 @@ a metric-compatible substitute (Calibri → Carlito, Cambria → Caladea, Arial,
 Times New Roman, Courier New → Liberation); presentation and system faces are
 embedded in the deck. The report adds the font decisions.
 
+In hifi mode, `--reference auto` (the default) also renders the presentation with an
+independent witness (`office2pdf`, in its own bounded render process, 20 slides at a
+time) and writes to the report a visual gap per slide (mean OKLab ΔE) with its three
+worst regions (the source items that differ most, after a realignment of up to 3 px);
+a slide whose worst region exceeds `[import] score_threshold` is flagged, and a second
+output line sums it up. `--reference none` skips it. In both modes, a text field
+rendered past its page becomes an `Overflow` loss.
+
 ## Running the app
 
 Everything runs from the repository root with [just](https://just.systems).
```

CLI architecture: the witness has a worker of its own and is asked 20 slides at a time.

```diff
--- a/crates/oku-slides-cli/ARCHITECTURE.md
+++ b/crates/oku-slides-cli/ARCHITECTURE.md
@@ -18,8 +18,8 @@ without Tauri. Results go to stdout, logs to stderr, help in the user's language
 - `import.rs`: `oku-slides import`, `ImportMode` `retrofit` (onto an existing pack and
   theme) or `hifi`; one transaction, then saved like `save`, with an optional report.
 - `import_overflow.rs`: text fields rendered past their page become `Overflow` losses.
-- `import_reference.rs`: hi-fi `--reference auto`: the witness in the render worker, each Oku
-  page cropped to the source page at 960 px, `metrics::score` per slide into the report.
+- `import_reference.rs`: hi-fi `--reference auto`: the witness in a render worker of its own, 20
+  slides at a time, each Oku page cropped to the source page at 960 px, scored into the report.
 - `import_hifi.rs`: hifi preparation: generated pack and theme with an identity from the
   `.pptx` bytes, font catalog, probe check against the declared schemas.
 - `system_fonts.rs`: copies the system faces of the families a presentation asks for.
```

Import architecture: shorter `metrics` line (keeps the file within 50 lines) and the t-816 known limit, which Task 7 dropped.

```diff
--- a/crates/oku-slides-import/ARCHITECTURE.md
+++ b/crates/oku-slides-import/ARCHITECTURE.md
@@ -15,8 +15,7 @@ Oku ops. Spec: `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.
   theme and pack (`PackFiles`), declared schemas and fonts; `report_json` is the `--report` form.
 - `report` : `ImportReport`, `Loss`/`LossKind`, warnings, `Coverage`, `ReferenceReport`, slide
   `score`; `coverage::coverage` measures what of the source survives in the resulting deck.
-- `metrics` : a hi-fi slide's visual gap to the reference (mean OKLab ΔE, light blur) and its worst
-  regions (owned pixels, realigned up to `MAX_SHIFT` px, worst `TILE`). Pure: the CLI renders.
+- `metrics` : a hi-fi slide's gap to the reference and its worst regions (pure; the CLI renders).
 - `fonts` : `FontCatalog`, `FontOrigin`, `FontPlan`, `SUBSTITUTES`, `FALLBACKS`.
 
 ## Layers
@@ -35,6 +34,7 @@ Oku ops. Spec: `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.
   `presetShapeDefinitions.xml`), slide items as elements (`slide`, `element`, `table`).
 - `pattern` : retrofit's signature that groups repeated item groups into one widget request.
 - `fonts/` : embedded font, then app or system catalog, then metric substitute, then `Missing`.
+- Known limit, t-816 (retrofit): flow content taller than its zone leaves the page (`Overflow`).
 
 ## Extension points
 
```

Reference crate rules (the second hunk of each pair is a pitfall reworded: "letter-spaced titles", "likely witness error").

```diff
--- a/crates/oku-slides-reference/AGENTS.md
+++ b/crates/oku-slides-reference/AGENTS.md
@@ -4,10 +4,10 @@ Read `ARCHITECTURE.md` first for what this code is. This file is how to work on
 
 ## Invariants
 
-- Run a reference witness only through RenderWorker::reference in the render worker, never in-process. (t-692, t-327)
+- Run a reference witness only through RenderWorker::reference, in a render worker of its own, never in-process. (t-692, t-327)
 - Give the witness the app's fonts and the deck's embedded fonts, and render hidden slides. (t-692)
 
 ## Pitfalls
 
-- Read a gap on titles, autofit text, charts or SmartArt as a possible witness error before changing the import. (t-692)
+- Read a gap on letter-spaced titles, autofit text, charts or SmartArt as a likely witness error before changing the import. (t-692)
 - Do not align the import on office2pdf code without checking it against the OOXML or another witness. (t-692)
```

Reference crate architecture: span argument, pinned `WITNESS`, font stripping, worker of its own, measured limits, dependency cost.

```diff
--- a/crates/oku-slides-reference/ARCHITECTURE.md
+++ b/crates/oku-slides-reference/ARCHITECTURE.md
@@ -7,28 +7,33 @@ gap, `oku_slides_import::metrics`, t-692 lot 4).
 ## API
 
 - `ReferenceRenderer` (trait): `witness()` (name and version written to the report),
-  `render(pptx, fonts) -> ReferencePdf`. `Office2Pdf` is the only witness shipped.
+  `render(pptx, fonts, slides: Option<SlideSpan>) -> ReferencePdf`. `Office2Pdf` is the only
+  witness shipped; `WITNESS` its name, pinned with the `=0.8.1` requirement (a test checks it).
+  It strips `ppt/fonts/` from the `.pptx` (office2pdf would write them to the temp dir).
 - `ReferencePdf`: `page_count`, `page_size(i)` (pt), `raster(i, width) -> Raster` through
   `hayro`, on white, sides at most `MAX_RASTER_SIDE`.
-- `Raster`: opaque 8-bit RGBA; `crop`, `from_png`, `to_png` (the render worker pipe carries PNGs).
+- `Raster`: opaque 8-bit RGBA; `crop`, `from_png`, `to_png` (the worker pipe carries PNGs).
 - `ReferenceError` opaque, `kind() -> ReferenceErrorKind` (`Render`, `Pdf`, `NoPage`, `Size`,
   `Png`): never fatal for the import, whose report says the reference is missing.
 
 ## Isolation
 
 The witness reads an untrusted file without bounds of its own: it runs in the render worker
-(`oku-slides-render-worker`, `Reference` and `ReferencePng` requests), bounded in time and memory.
+(`oku-slides-render-worker`, `Reference` and `ReferencePng` requests), bounded in time and memory;
+the CLI gives it a worker of its own and asks 20 slides at a time.
 
 ## Known limits (`office2pdf` witness)
 
-- It shrinks `normAutofit` text without `fontScale` (PowerPoint keeps 100 %), draws charts
-  roughly, SmartArt text too small, and the alto titles about 1 % wider than Oku and LibreOffice.
+- It shrinks `normAutofit` text without `fontScale` (PowerPoint keeps 100 %: the alto p. 2 title,
+  every footer), draws letter-spaced titles 2–5 px wider than Oku and LibreOffice, charts roughly
+  and SmartArt text too small: such regions often rank first.
 - It shares Typst's shaping and line breaking with Oku: a PowerPoint line-break gap (t-863)
   does not show.
 - The fonts given come first; the system fonts stay its fallback (outside WASM).
 - An import behavior copied from office2pdf (Apache-2.0, credited in the file) is no longer
   measured by it: check it against another witness or against the OOXML.
-- It also pulls the crate's DOCX and XLSX readers (`docx-rs`, `umya-spreadsheet`), never called.
+- It also pulls the crate's DOCX and XLSX readers (`docx-rs`, `umya-spreadsheet`), never called,
+  into the render worker and so the desktop app (~48 crates; lot 5 needs the witness there).
 
 ## Extension points
 
```

Render worker architecture: `RenderWorker::reference` takes the slide span.

```diff
--- a/crates/oku-slides-render-worker/ARCHITECTURE.md
+++ b/crates/oku-slides-render-worker/ARCHITECTURE.md
@@ -15,9 +15,9 @@ too much memory is killed without taking the app down.
 - `RemoteCompiled` (`compiled.rs`): a generation; `layout`, `page_hash` local, `svg`/`png`/`pdf`
   by request; its `Drop` queues `Release`. The queue is sent before the next request: a
   `Drop` never takes the link lock.
-- `RemoteReference` (`reference.rs`): `RenderWorker::reference(pptx, fonts)` has the witness
-  (`oku_slides_reference::Office2Pdf`) render the presentation in the worker, under its bounds;
-  `witness`, `page_count`, `page_size` local, `png(page, width)` by request; its `Drop` queues
+- `RemoteReference` (`reference.rs`): `RenderWorker::reference(pptx, fonts, slides)` has the
+  witness (`oku_slides_reference::Office2Pdf`) render the presentation, or a span of its slides, in
+  the worker, under its bounds; `witness`, `page_count` local, `png(page, width)` by request; `Drop` queues
   `ReleaseReference`. No pack is blamed; a reference from another epoch answers `Stale`.
 - `Link` (`link.rs`, internal): transport (child process or thread), timeout, memory sampled
   every 100 ms (`rss.rs`: `phys_footprint` on macOS, `footprint.rs`, resident memory elsewhere),
```

Spec: lot 4 decisions after the plan review (pinned witness and stripped fonts, own worker and 20-slide spans, σ = 2, worst-region threshold 10.0 with the alto measurements, witness limits, app dependency cost) and the CLI output line.

```diff
--- a/docs/superpowers/specs/2026-10-09-import-pptx-design.md
+++ b/docs/superpowers/specs/2026-10-09-import-pptx-design.md
@@ -161,36 +161,55 @@ Decisions from the lot 4 plan (2026-10-14):
    user ruled out a local LibreOffice install: the reference comes from the
    `office2pdf` crate (pure Rust on Typst 0.15, Apache-2.0), behind a
    `ReferenceRenderer` trait in a new crate `oku-slides-reference` (PDF
-   rasterized by `hayro`). It gets the app's fonts and the deck's embedded
-   fonts first, and renders hidden slides. LibreOffice and PowerPoint
-   witnesses are backlog work, as other implementations of the trait.
+   rasterized by `hayro`), pinned at `=0.8.1` (`WITNESS` names it in the
+   report). It gets the app's fonts and the deck's embedded fonts first (the
+   `.pptx` it reads has its `ppt/fonts/` parts removed: office2pdf would write
+   them to the system temp dir), and renders hidden slides. LibreOffice and
+   PowerPoint witnesses are backlog work, as other implementations of the trait.
 2. **Isolation.** The witness reads an untrusted file without bounds of its
-   own: it runs in the render worker (`Reference`, `ReferencePng`,
-   `ReleaseReference` requests; `RenderWorker::reference` →
-   `RemoteReference`), under `timeout_ms` and `memory_mib`. A missing,
-   failing or mismatched reference (`ReferenceCause`: `failed`, `timeout`,
-   `memory`, `pages`) is reported, never an error.
+   own: it runs in a render worker of its own, not the one holding the Oku
+   render (`Reference`, `ReferencePng`, `ReleaseReference` requests;
+   `RenderWorker::reference` → `RemoteReference`), under `[import]
+   reference_timeout_ms` (30 s) and `memory_mib`, 20 slides per request
+   (`slide_range`): memory does not grow with the deck. Slides pair with the
+   witness pages by their source number. A failing or mismatched reference, or
+   a slide that cannot be scored (`ReferenceCause`: `failed`, `timeout`,
+   `memory`, `pages`), is reported and logged, never an error, and leaves no
+   slide with a score.
 3. **Metric** (`oku-slides-import/src/metrics.rs`, pure, calibrated on alto).
    Slide score: mean OKLab ΔE ×100 over the source page at 960 px after a
-   σ = 1 px Gaussian blur. Region score: each pixel belongs to the topmost
-   item covering it; the region is realigned by the integer shift (at most
-   3 px per axis) that minimizes its mean gap, the shift is reported; its
-   score is the mean gap of its worst 16 px tile (tiles overlap by half);
-   below 100 own pixels, no score. The report keeps the 3 worst regions.
-4. **Threshold.** `[import] score_threshold = 3.0`: the fixtures stay at or
-   under 2.8. Tests run the witness live (pinned by the lockfile, no
-   versioned reference PDF) with per-fixture bounds (measurement + 25 %) and
-   designation checks on alto (p. 7 callout line break, p. 8 bullet indents)
-   and on the `libreoffice.pptx` table.
+   σ = 2 px Gaussian blur (at σ = 1, alto titles aligned within 1 px ranked
+   above the real gaps). Region score: each pixel belongs to the topmost
+   item covering it (at most 256 regions, the topmost); the region is
+   realigned by the integer shift (at most 3 px per axis) that minimizes its
+   mean gap, the shift is reported; its score is the mean gap of its worst
+   16 px tile (tiles overlap by half); below 100 own pixels or with a negative
+   frame, no score. The report keeps the 3 worst regions.
+4. **Threshold** (plan review). The slide mean does not separate a faithful
+   slide from a damaged one (the broken table of `libreoffice.pptx` p. 1
+   scores 0.6): a slide is flagged when its **worst region** exceeds
+   `[import] score_threshold = 10.0`. Measured on alto: a title aligned
+   within 1 px scores 7.7–8.3 (the noise floor), the real gaps 8.7–22.5, the
+   witness's own errors up to 46; the flag is a coarse alert, the regions
+   point at the item. Tests run the witness live (pinned, no versioned
+   reference PDF) with per-fixture bounds on the slide mean (measurement +
+   25 %) and designation checks: alto p. 8 bullets and p. 7 callout among
+   the 3 worst regions, the `libreoffice.pptx` table first.
 5. **Retrofit.** No visual gap (another theme); `--reference` is refused there.
    In both modes, a non-empty text field rendered past its page edge becomes an
    `Overflow` loss on its slide (t-816 stays open for the fix).
-6. **Known witness limits.** It shrinks `normAutofit` text without
-   `fontScale`, draws charts roughly and SmartArt text too small, renders the
-   alto titles about 1 % wider than Oku and LibreOffice, and shares Typst's
-   line breaking with Oku (a PowerPoint line-break gap, t-863, is invisible
-   to it). A behavior copied from office2pdf into the import is no longer
-   measured by it.
+6. **Known witness limits** (plan review, real render). It shrinks
+   `normAutofit` text without `fontScale` (the alto p. 2 title and every
+   footer), draws letter-spaced titles 2–5 px wider than Oku (which matches
+   HarfBuzz within 1 px) and LibreOffice, draws charts roughly and SmartArt
+   text too small: those regions often rank first. It shares Typst's line
+   breaking with Oku (a PowerPoint line-break gap, t-863, is invisible to
+   it), and office2pdf always keeps the system fonts as its fallback. A
+   behavior copied from office2pdf into the import is no longer measured by
+   it. An autofit-aware ranking is backlog work before lot 5.
+7. **Cost.** office2pdf brings its DOCX and XLSX readers into the render
+   worker, hence the desktop app (about 48 crates); no feature flag in lot 4,
+   since lot 5 needs the witness in the app.
 
 Defaults chosen without asking:
 
@@ -454,8 +473,8 @@ cause when missing; `above_threshold` on each slide beyond it, lot 4),
   embedded in the deck only); the presentation's fonts are
   looked up among the app's (`--fonts`) and, unless `--no-system-fonts`,
   the system's. `--reference auto|none` (lot 4): hi-fi only, `auto` by
-  default; a second output line gives the mean gap and the slides above the
-  threshold.
+  default; a second output line gives the mean gap and the slides whose worst
+  region is above the threshold, or why the reference is missing.
 - **App**: "Ouvrir" (Open) accepts `.pptx`; a dialog asks for the mode, and in
   retrofit the target pack and theme. The app creates a new deck from the
   plan, then launches the agent with the fine-tuning instructions.
```

- [ ] **Step 2: Run the lint and the line checks**

Run: `just agents-lint`
Expected: `✓ harnais AGENTS.md`

Run: `wc -l crates/oku-slides-import/ARCHITECTURE.md crates/oku-slides-cli/ARCHITECTURE.md crates/oku-slides-render-worker/ARCHITECTURE.md crates/oku-slides-reference/ARCHITECTURE.md`
Expected: each file at most 50 lines (the `total` line does not count).

- [ ] **Step 3: Commit**

```bash
git add README.md crates/oku-slides-cli/ARCHITECTURE.md crates/oku-slides-import/ARCHITECTURE.md crates/oku-slides-reference/AGENTS.md crates/oku-slides-reference/ARCHITECTURE.md crates/oku-slides-render-worker/ARCHITECTURE.md docs/superpowers/specs/2026-10-09-import-pptx-design.md
```

Write this message to a file (for example `/tmp/commit-t11.txt`), then commit it:

```
docs(import) : lot 4 — décisions après la revue du plan (σ = 2, pire région, processus et tranches du témoin, limites du témoin), README (t-692)
```

```bash
git commit -F /tmp/commit-t11.txt
```

**Extracting from the draft:** the implementer may take these files from the draft commit `f96e0ef2` instead of retyping them: `git checkout f96e0ef2 -- README.md crates/oku-slides-cli/ARCHITECTURE.md crates/oku-slides-import/ARCHITECTURE.md crates/oku-slides-reference/AGENTS.md crates/oku-slides-reference/ARCHITECTURE.md crates/oku-slides-render-worker/ARCHITECTURE.md docs/superpowers/specs/2026-10-09-import-pptx-design.md` (the list is `git diff --name-only f96e0ef2~1 f96e0ef2`), never a directory. A file also changed on `main` since the draft base `53454df2` (`git diff --name-only 53454df2 main`) must instead be applied with `git show f96e0ef2 -- <file> | git apply --3way`: of this task's files, these are `README.md`, `crates/oku-slides-cli/ARCHITECTURE.md`, `crates/oku-slides-import/ARCHITECTURE.md` and `docs/superpowers/specs/2026-10-09-import-pptx-design.md`. The two `ARCHITECTURE.md` files of the reference and render-worker crates and the reference `AGENTS.md` can be checked out directly only if Task 7 did not already rewrite them in English with different wording; if it did, use the `--3way` form for them as well.
