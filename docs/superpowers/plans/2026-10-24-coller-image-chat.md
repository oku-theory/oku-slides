# Pasting an image into the agent chat — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Paste an image into the agent draft; the agent sees it (ACP image block) and can place it on a slide, which adds it to the deck as an asset.

**Architecture:** Pasting sends the bytes to the host (`paste_image`); the tab session validates them and keeps them in an in-memory store under their `sha256:` asset id, and the host draft lists their refs. A prompt carries the ids; the turn sends a text block naming them, then image blocks when the agent advertises `promptCapabilities.image`. `Session::apply` adds a pasted image to the bundle the first time an op makes the deck cite it, and refuses any other newly cited unknown asset.

**Tech Stack:** Rust (oku-slides-core, -session, -agent, -a2ui, -platform, -i18n), ACP schema 1.9.1, React/TS (Bun, happy-dom), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-24-coller-image-chat-design.md` (amended in the same commit as this plan: see "Spec amendments").

**Task:** t-1161 (parent); one sub-task per plan task below, titles verbatim.

## Global Constraints

- Formats accepted on paste: `image/png`, `image/jpeg`, `image/gif`, `image/webp`, by sniffed magic bytes only.
- `[paste]` config defaults: `max_image_mib = 10`, `max_image_pixels = 67108864`, `max_draft_images = 8`, `max_kept_mib = 100`, `prompt_image_mib = 5`, `prompt_image_max_side = 8000`, `prompt_total_mib = 20`.
- An attachment id is `oku_slides_core::asset_id(bytes)` = `sha256:<hex>`.
- No `std::fs` in production code; no `unwrap`/`expect` outside tests; Snafu errors with a backtrace in every variant, opaque public types, Fluent-localized messages that never quote user data.
- `std::sync::Mutex` locked with `lock().unwrap_or_else(PoisonError::into_inner)`, never held across an await; session work from async code runs in `spawn_blocking`.
- Every new Fluent key goes in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), same key order, translated (never the English copied); English first.
- Every new `pub` item and module documented, first sentence under 15 words. Source files target 500 lines, never above 1000.
- Rust tests in sibling `x_tests.rs` (`#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`), speculoos asserts, rstest tables, names `unit_scenario_result`.
- `ComponentKind` and `ui/src/a2ui/turnComponents.tsx` change in the same commit; `CATALOG_ID` stays `urn:oku-slides:a2ui:catalog:v4`.
- Each `ARCHITECTURE.md` touched stays at most 50 lines and is committed with the code it describes.
- Commits: `type(scope) : description (t-N)`, message written to a scratchpad file and committed with `git commit -F <file>` (never a heredoc, never `$TMPDIR`); `git add` in its own call; no `Co-Authored-By` line.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Spec amendments

Found while reading the code; this plan follows them and the spec is updated with it:

1. **Store in memory only.** The agent draft itself is not persisted (`TabLink.draft`), so attachments are not written to the resume folder.
2. **Byte-capped store instead of turn-based pruning.** The tab store keeps at most `max_kept_mib`; past it, the oldest images not in the current draft leave. An evicted id is then refused like any unknown one.
3. **Commands `paste_image` and `remove_image`.** `attach` already names re-attaching the agent window in the UI. The host is the authority for the draft's images: every `draft` event carries them, `set_draft` still carries only text, and closing the draft (`text: null`) drops its images.
4. **Unknown assets by difference.** `apply` refuses an asset id that the new deck cites, the old deck did not, and neither the bundle nor the store holds (`roadmap.json` already cites a missing `sha256:0000`, which keeps working).
5. **20 locales**, not English and French only.
6. **Paste errors in the toast** (`say`), like every other host refusal in the draft, not in `draft-notice` (a status prop of the panel).

## File structure

| File | Responsibility |
|---|---|
| `crates/oku-slides-core/src/media.rs` (moved from import) | Sniff a MIME type, read raster dimensions from headers |
| `crates/oku-slides-core/src/validate.rs` | + `asset_refs`: every asset id a deck cites |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml` | `[paste]` limits (`PasteSettings`) |
| `crates/oku-slides-session/src/agent/attachments.rs` | `AttachmentRef`, the tab's `Attachments` store, `Session::paste_image`, `pasted_image`, `embed_op_assets` |
| `crates/oku-slides-agent/src/paste.rs` | `paste_image`, `remove_image`, `resolve_images` for the stack |
| `crates/oku-slides-agent/src/prompt_images.rs` | The prompt blocks of pasted images |
| `crates/oku-slides-a2ui/src/message.rs` | `PromptImage`, `ComponentKind::Prompt.images` |
| `ui/src/agent/pastedImages.ts` | Pure helpers: clipboard images, base64, sha256 id, chip label |
| `ui/src/agent/usePastedImages.ts` | Local thumbnails and the paste action of one view |
| `ui/src/DictationDraft.tsx` | Paste handler, chip row |
| `ui/e2e/260-coller-image.cjs` | End to end: paste, prompt blocks, image placed |

---

### Task 1: Move media sniffing into the core

**Files:**
- Move: `crates/oku-slides-import/src/pptx/media.rs` → `crates/oku-slides-core/src/media.rs`
- Move: `crates/oku-slides-import/src/pptx/media_tests.rs` → `crates/oku-slides-core/src/media_tests.rs`
- Modify: `crates/oku-slides-core/src/lib.rs` (module list and doc), `crates/oku-slides-core/ARCHITECTURE.md`
- Modify: `crates/oku-slides-import/src/pptx/mod.rs:14`, `crates/oku-slides-import/src/pptx/context.rs:14`, `crates/oku-slides-import/src/source/font.rs:86`

**Interfaces:**
- Produces: `oku_slides_core::media::{sniff(&[u8]) -> &'static str, dimensions(&[u8], &str) -> Option<(u32, u32)>, SVG, OCTET_STREAM}`.

This is a move: the tests already exist and must stay green in their new crate.

- [ ] **Step 1: Move the files with git**

```bash
git mv crates/oku-slides-import/src/pptx/media.rs crates/oku-slides-core/src/media.rs
git mv crates/oku-slides-import/src/pptx/media_tests.rs crates/oku-slides-core/src/media_tests.rs
```

- [ ] **Step 2: Make the items public**

In `crates/oku-slides-core/src/media.rs`, replace `pub(crate) const OCTET_STREAM`, `pub(crate) const SVG`, `pub(crate) fn sniff`, `pub(crate) fn dimensions` by `pub`. Change the module doc line to:

```rust
//! Type réel d'un média, lu dans ses premiers octets et jamais déduit de son extension.
//!
//! L'import d'un `.pptx` et le collage d'une image dans le brouillon de l'agent s'en servent.
```

In `crates/oku-slides-core/src/lib.rs`, add `pub mod media;` between `pub mod locate;` and `pub mod model;`, and add to the crate doc list, after the `kinds` line:

```rust
//! - [`media`] : type réel d'un média ([`media::sniff`]) et dimensions d'un raster lues dans son en-tête.
```

- [ ] **Step 3: Point the import at the core**

`crates/oku-slides-import/src/pptx/mod.rs`: delete the line `pub(crate) mod media;`.
`crates/oku-slides-import/src/pptx/context.rs:14`: replace `use crate::pptx::media::{SVG, dimensions, sniff};` by `use oku_slides_core::media::{SVG, dimensions, sniff};`.
`crates/oku-slides-import/src/source/font.rs:86`: in the doc comment replace `` `pptx::media::sniff` `` by `` `oku_slides_core::media::sniff` ``.

- [ ] **Step 4: Run the moved tests and the import tests**

Run: `cargo nextest run -p oku-slides-core media && cargo nextest run -p oku-slides-import`
Expected: PASS (same test count for `media_tests` as before the move).

- [ ] **Step 5: Update `crates/oku-slides-core/ARCHITECTURE.md`**

In the "Modules" list, on the line that starts `` - `tokens.rs`: ``, append: `` `media.rs`: `sniff` (magic bytes), raster `dimensions` from headers, used by the import and by pasting. `` Check the file is at most 50 lines (`wc -l`); if not, merge this into the `fields.rs, locate.rs, color.rs` line.

- [ ] **Step 6: Commit**

```bash
git add -A crates/oku-slides-core crates/oku-slides-import
git commit -F <scratchpad>/msg-t1.txt   # refactor(core) : sniff et dimensions des médias dans le cœur, partagés par l'import et le collage (t-1162)
```

---

### Task 2: `[paste]` settings and the tab's attachment store

**Files:**
- Modify: `crates/oku-slides-platform/src/config.default.toml` (new `[paste]` section after `[import]`), `crates/oku-slides-platform/src/config.rs` (`Settings.paste`, `PasteSettings`)
- Create: `crates/oku-slides-session/src/agent/attachments.rs`, `crates/oku-slides-session/src/agent/attachments_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/mod.rs`, `crates/oku-slides-session/src/lib.rs` (field `attachments`, built in `Session::unrendered`), `crates/oku-slides-session/src/error.rs`, `crates/oku-slides-session/ARCHITECTURE.md`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 files)

**Interfaces:**
- Consumes: `oku_slides_core::media::{sniff, dimensions}`, `oku_slides_core::asset_id`.
- Produces:
  - `oku_slides_platform::config::PasteSettings { max_image_mib: usize, max_image_pixels: u64, max_draft_images: usize, max_kept_mib: usize, prompt_image_mib: usize }` with `max_image_bytes()`, `max_kept_bytes()`, `prompt_image_bytes()` (all `usize`); `Settings.paste`.
  - `oku_slides_session::agent::AttachmentRef { id: String, mime: String, width: u32, height: u32 }` (`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`).
  - `Session::paste_image(&mut self, bytes: Vec<u8>, keep: &[String]) -> Result<AttachmentRef>`
  - `Session::pasted_image(&self, id: &str) -> Option<(AttachmentRef, Arc<[u8]>)>`
  - `SessionErrorKind::Paste`; Fluent keys `session-paste-unsupported`, `session-paste-too-large`, `session-paste-too-many-pixels`, `session-paste-unreadable`.

- [ ] **Step 1: Add the settings**

`config.default.toml`, after the `[import]` section:

```toml
[paste]
# Images collées dans le brouillon de l'agent : PNG, JPEG, GIF ou WebP.
# Taille maximale d'une image (Mio).
max_image_mib = 10
# Pixels au plus d'une image, lus dans son en-tête avant tout décodage (64 Mpx, comme l'import).
max_image_pixels = 67108864
# Images au plus dans un brouillon.
max_draft_images = 8
# Taille cumulée des images gardées par onglet (Mio) ; au-delà, les plus anciennes hors du
# brouillon sont oubliées. Une image posée dans le deck n'en dépend plus.
max_kept_mib = 100
# Plus grande image montrée à l'agent (Mio) ; au-delà, il n'en reçoit que l'identifiant.
prompt_image_mib = 5
```

`config.rs`: add `pub paste: PasteSettings,` to `Settings` after `pub import: ImportSettings,`, and after `SourcesSettings`'s impl:

```rust
/// Bornes des images collées dans le brouillon de l'agent (`[paste]`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct PasteSettings {
    /// Taille maximale d'une image (Mio).
    pub max_image_mib: usize,
    /// Pixels au plus d'une image, lus dans son en-tête.
    pub max_image_pixels: u64,
    /// Images au plus dans un brouillon.
    pub max_draft_images: usize,
    /// Taille cumulée des images gardées par onglet (Mio).
    pub max_kept_mib: usize,
    /// Plus grande image envoyée à l'agent en bloc image (Mio).
    pub prompt_image_mib: usize,
}

impl PasteSettings {
    /// [`Self::max_image_mib`] en octets.
    pub fn max_image_bytes(&self) -> usize {
        self.max_image_mib.saturating_mul(1024 * 1024)
    }

    /// [`Self::max_kept_mib`] en octets.
    pub fn max_kept_bytes(&self) -> usize {
        self.max_kept_mib.saturating_mul(1024 * 1024)
    }

    /// [`Self::prompt_image_mib`] en octets.
    pub fn prompt_image_bytes(&self) -> usize {
        self.prompt_image_mib.saturating_mul(1024 * 1024)
    }
}
```

Add a test to `crates/oku-slides-platform/src/config_tests.rs`:

```rust
#[test]
fn defaults_paste_is_ten_mib_eight_images_five_mib_shown() {
    let s = Settings::defaults().unwrap();
    assert_that!((s.paste.max_image_bytes(), s.paste.max_draft_images, s.paste.prompt_image_bytes()))
        .is_equal_to((10 << 20, 8, 5 << 20));
}
```

Run: `cargo nextest run -p oku-slides-platform config` — Expected: PASS.

- [ ] **Step 2: Write the failing store tests**

`crates/oku-slides-session/src/agent/attachments_tests.rs`:

```rust
use std::io::Cursor;

use rstest::rstest;
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
    assert_that!((r.id, r.mime.as_str(), r.width, r.height))
        .is_equal_to((oku_slides_core::asset_id(&bytes), "image/png", 3, 2));
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
```

Register the tests at the bottom of `attachments.rs` (`#[cfg(test)] #[path = "attachments_tests.rs"] mod attachments_tests;`).

Run: `cargo nextest run -p oku-slides-session attachments` — Expected: FAIL to compile (`paste_image` not found).

- [ ] **Step 3: Write the store**

`crates/oku-slides-session/src/agent/attachments.rs`:

```rust
//! Images collées dans le brouillon de l'agent : gardées par onglet, en mémoire.
//!
//! Une image est validée au collage (type réel, octets, pixels de l'en-tête), rangée sous
//! son identifiant d'asset `sha256:…`, et n'entre dans le deck que quand une op la cite
//! (`Session::embed_op_assets`, tâche suivante).

use std::collections::VecDeque;
use std::sync::Arc;

use oku_slides_core::media::{dimensions, sniff};
use serde::{Deserialize, Serialize};

use crate::error::PasteSnafu;
use crate::{Result, Session};

/// Les types qu'un collage accepte : ceux que Typst dessine tels quels.
const PASTE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Une image collée : son identifiant d'asset, son type réel et sa taille en pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRef {
    /// `sha256:<hex>` de ses octets, le même que son asset une fois posée.
    pub id: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
}

/// Pourquoi un collage est refusé (erreur [`crate::SessionErrorKind::Paste`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PasteReason {
    /// Pas un PNG, JPEG, GIF ni WebP.
    Unsupported,
    /// Au-delà de `[paste] max_image_mib`.
    TooLarge,
    /// Au-delà de `[paste] max_image_pixels`.
    TooManyPixels,
    /// En-tête illisible.
    Unreadable,
}

impl PasteReason {
    /// La clé Fluent du refus.
    pub fn key(self) -> &'static str {
        match self {
            Self::Unsupported => "session-paste-unsupported",
            Self::TooLarge => "session-paste-too-large",
            Self::TooManyPixels => "session-paste-too-many-pixels",
            Self::Unreadable => "session-paste-unreadable",
        }
    }
}

/// Les images collées d'un onglet, de la plus ancienne à la plus récente.
#[derive(Debug)]
pub(crate) struct Attachments {
    limits: oku_slides_platform::config::PasteSettings,
    items: VecDeque<(AttachmentRef, Arc<[u8]>)>,
    bytes: usize,
}

impl Attachments {
    pub(crate) fn new(limits: oku_slides_platform::config::PasteSettings) -> Self {
        Self { limits, items: VecDeque::new(), bytes: 0 }
    }

    /// Valide et garde `bytes` ; au-delà de `max_kept_mib`, oublie les plus anciennes hors de `keep`.
    fn add(&mut self, bytes: Vec<u8>, keep: &[String]) -> std::result::Result<AttachmentRef, PasteReason> {
        let mime = sniff(&bytes);
        if !PASTE_TYPES.contains(&mime) {
            return Err(PasteReason::Unsupported);
        }
        if bytes.len() > self.limits.max_image_bytes() {
            return Err(PasteReason::TooLarge);
        }
        let (width, height) = dimensions(&bytes, mime).ok_or(PasteReason::Unreadable)?;
        if u64::from(width) * u64::from(height) > self.limits.max_image_pixels {
            return Err(PasteReason::TooManyPixels);
        }
        let id = oku_slides_core::asset_id(&bytes);
        // Déjà là : elle redevient la plus récente.
        if let Some(i) = self.items.iter().position(|(r, _)| r.id == id) {
            if let Some(item) = self.items.remove(i) {
                let r = item.0.clone();
                self.items.push_back(item);
                return Ok(r);
            }
        }
        let r = AttachmentRef { id, mime: mime.to_owned(), width, height };
        self.bytes += bytes.len();
        self.items.push_back((r.clone(), bytes.into()));
        self.evict(keep, &r.id);
        Ok(r)
    }

    /// Oublie les plus anciennes, hors de `keep` et de `fresh`, tant que le total dépasse la borne.
    fn evict(&mut self, keep: &[String], fresh: &str) {
        while self.bytes > self.limits.max_kept_bytes() {
            let Some(i) = self.items.iter().position(|(r, _)| r.id != fresh && !keep.contains(&r.id)) else {
                return;
            };
            if let Some((_, b)) = self.items.remove(i) {
                self.bytes -= b.len();
            }
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<(AttachmentRef, Arc<[u8]>)> {
        self.items.iter().find(|(r, _)| r.id == id).map(|(r, b)| (r.clone(), b.clone()))
    }
}

impl Session {
    /// Valide une image collée et la garde pour l'onglet ; `keep` : les ids du brouillon.
    pub fn paste_image(&mut self, bytes: Vec<u8>, keep: &[String]) -> Result<AttachmentRef> {
        self.attachments.add(bytes, keep).map_err(|reason| PasteSnafu { reason }.build().into())
    }

    /// Une image collée gardée par l'onglet, et ses octets.
    pub fn pasted_image(&self, id: &str) -> Option<(AttachmentRef, Arc<[u8]>)> {
        self.attachments.get(id)
    }
}

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod attachments_tests;
```

`agent/mod.rs`: add `pub mod attachments;` after `mod address;`, and `pub use attachments::{AttachmentRef, PasteReason};` after `pub use bounds::…`; append to the module doc: `//! Images collées : \`attachments\` (\`AttachmentRef\`).`

`lib.rs`: add the field after `ui_order` with its doc:

```rust
    /// Images collées dans le brouillon de l'agent de cet onglet (`[paste]`).
    attachments: agent::attachments::Attachments,
```

and in `Session::unrendered`, in the `Self { … }` literal: `attachments: agent::attachments::Attachments::new(settings.paste.clone()),`. If the compiler reports another `Session { … }` literal, add the same line there.

`error.rs`: add to `SessionErrorKind` (before `AgentForm`):

```rust
    /// Une image collée refusée ([`SessionError::paste_reason_key`] dit pourquoi).
    Paste,
```

to `SessionInner`, next to `AgentForm`:

```rust
    #[snafu(display("pasted image refused: {reason:?}"))]
    Paste { reason: crate::agent::PasteReason, backtrace: Backtrace },
```

to the kind match: `Paste { .. } => SessionErrorKind::Paste,`; to `localize`: `Paste { reason, .. } => l.tr(reason.key()),`; and next to `agent_form_reason`:

```rust
    /// La clé Fluent d'un collage refusé, pour une erreur [`SessionErrorKind::Paste`].
    pub fn paste_reason_key(&self) -> Option<&'static str> {
        match &*self.0 {
            SessionInner::Paste { reason, .. } => Some(reason.key()),
            _ => None,
        }
    }
```

`session.ftl`, appended at the end of each of the 20 files (English then French shown; translate for the other 18):

```ftl
# en
session-paste-unsupported = Only PNG, JPEG, GIF and WebP images can be pasted.
session-paste-too-large = This image is too heavy to paste.
session-paste-too-many-pixels = This image has too many pixels to paste.
session-paste-unreadable = This image could not be read.
# fr
session-paste-unsupported = Seules les images PNG, JPEG, GIF et WebP se collent.
session-paste-too-large = Cette image est trop lourde pour être collée.
session-paste-too-many-pixels = Cette image a trop de pixels pour être collée.
session-paste-unreadable = Cette image est illisible.
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-session attachments && cargo nextest run -p oku-slides-i18n`
Expected: PASS (the i18n test `every_locale_has_exactly_the_reference_keys` checks the 20 files).

- [ ] **Step 5: Update `crates/oku-slides-session/ARCHITECTURE.md`**

In the `agent/` bullet, after `` `reason` (`AgentFormReason`) ``, add: `` ; `attachments` (pasted images: in-memory per tab, `[paste]` bounds, `AttachmentRef`) ``. Keep the file at most 50 lines.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-platform crates/oku-slides-session crates/oku-slides-i18n
git commit -F <scratchpad>/msg-t2.txt   # feat(session) : images collées gardées par onglet, validées au collage, bornes [paste] (t-1163)
```

---

### Task 3: Add a pasted image to the deck when an op cites it

**Files:**
- Modify: `crates/oku-slides-core/src/validate.rs` (+ `asset_refs`, shared walk), `crates/oku-slides-core/src/validate_tests.rs`, `crates/oku-slides-core/src/lib.rs` (re-export)
- Modify: `crates/oku-slides-session/src/agent/attachments.rs` (+ `embed_op_assets`), `crates/oku-slides-session/src/agent/attachments_tests.rs`, `crates/oku-slides-session/src/deck_widgets.rs:327` (call), `crates/oku-slides-session/src/error.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 files)

**Interfaces:**
- Consumes: `Session::paste_image`, `Attachments::get` (Task 2).
- Produces: `oku_slides_core::asset_refs(deck: &Deck, registry: &WidgetRegistry) -> BTreeSet<String>`; `Session::embed_op_assets(&mut self, next: &Deck, registry: &WidgetRegistry) -> Result<()>` (crate-private); `SessionErrorKind::UnknownAsset`, key `session-asset-unknown`.

- [ ] **Step 1: Write the failing core test**

In `validate_tests.rs` (its helpers `content`, `registry`, `custom_registry` and the `test-el/bornes` widget with an `images` list whose `img` column is an asset are already there):

```rust
#[test]
fn asset_refs_lists_the_asset_of_an_image_element() {
    let d = content("blocks", json!([{ "id": "i", "kind": "base/image", "params": { "asset": "sha256:ab" } }]));
    assert_that!(asset_refs(&d, &registry())).is_equal_to(BTreeSet::from(["sha256:ab".to_owned()]));
}

#[test]
fn asset_refs_lists_the_assets_of_a_list_column() {
    let d = content("blocks", json!([{ "id": "f", "kind": "test-el/bornes", "params": { "images": [
        { "id": "a", "img": "sha256:ab" }, { "id": "b", "img": "sha256:cd" }
    ] } }]));
    let refs = asset_refs(&d, &custom_registry());
    assert_that!(refs).is_equal_to(BTreeSet::from(["sha256:ab".to_owned(), "sha256:cd".to_owned()]));
}
```

(add `use std::collections::BTreeSet;` to the file's imports.)

Run: `cargo nextest run -p oku-slides-core asset_refs` — Expected: FAIL (`asset_refs` not found).

- [ ] **Step 2: Share the asset walk**

In `validate.rs`, move the body of `Validator::assets` into a free function and call it from both:

```rust
/// Les valeurs des paramètres `asset` de `params` selon `schema`, listes comprises.
fn param_assets<'p>(schema: &WidgetSchema, params: &'p Params) -> Vec<&'p str> {
    let mut out = Vec::new();
    for p in &schema.params {
        match &p.ty {
            ParamType::Asset => out.extend(params.text(&p.name)),
            ParamType::List { item, .. } => {
                for it in params.list(&p.name).unwrap_or_default() {
                    for c in item {
                        match &c.ty {
                            ParamType::Asset => out.extend(it.values.text(&c.name)),
                            ParamType::List { item: sub, .. } => {
                                for nested in it.values.list(&c.name).unwrap_or_default() {
                                    let assets = sub.iter().filter(|s| s.ty == ParamType::Asset);
                                    out.extend(assets.filter_map(|s| nested.values.text(&s.name)));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Les assets que cite le deck : paramètres `asset` de ses slides et éléments.
pub fn asset_refs(deck: &Deck, registry: &WidgetRegistry) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for s in &deck.slides {
        if let Some(schema) = registry.get(&s.kind) {
            out.extend(param_assets(schema, &s.params).into_iter().map(str::to_owned));
        }
        for l in s.walk() {
            if let Some(schema) = registry.get(&l.element.kind) {
                out.extend(param_assets(schema, &l.element.params).into_iter().map(str::to_owned));
            }
        }
    }
    out
}
```

and `Validator::assets` becomes:

```rust
    /// Les assets des paramètres `asset` sont dans les sources.
    fn assets(&mut self, at: Place<'_>, schema: Option<&WidgetSchema>, params: &Params) {
        let (Some(schema), Some(assets)) = (schema, self.assets) else { return };
        let absent: Vec<String> =
            param_assets(schema, params).into_iter().filter(|a| !assets.contains(*a)).map(str::to_owned).collect();
        for asset in absent {
            self.push(at, Message::new("issue-missing-asset").arg("asset", asset));
        }
    }
```

The loops are those of today's `Validator::assets`, which now push the found values instead of the missing ones. Add `BTreeSet` to the `std::collections` import. In `lib.rs`, add `asset_refs` to the existing `pub use validate::{…}` list, and to the crate doc line about validation: `` validation ([`validate`], assets cités : [`asset_refs`]) ``.

Run: `cargo nextest run -p oku-slides-core validate` — Expected: PASS (new test and every existing `issue-missing-asset` test).

- [ ] **Step 3: Write the failing session tests**

Append to `attachments_tests.rs`:

```rust
use oku_slides_core::Op;
use serde_json::json;

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
    let e = s.apply("image", vec![set_image("sha256:1234")], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
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
```

Run: `cargo nextest run -p oku-slides-session attachments` — Expected: FAIL (`apply_citing_a_pasted_image…` finds no asset; `UnknownAsset` not found).

- [ ] **Step 4: Embed in `Session::apply`**

`error.rs`: kind `UnknownAsset` (doc: `/// Une op cite un asset ni dans le deck ni parmi les images collées.`), inner variant:

```rust
    #[snafu(display("an op cites asset {asset} that is neither in the deck nor pasted"))]
    UnknownAsset { asset: String, backtrace: Backtrace },
```

kind match `UnknownAsset { .. } => SessionErrorKind::UnknownAsset,`, localize `UnknownAsset { .. } => l.tr("session-asset-unknown"),`.

`attachments.rs`, in `impl Session`:

```rust
    /// Ajoute au bundle les images collées que `next` cite et que le deck ne citait pas.
    ///
    /// Un autre asset nouvellement cité, absent du bundle, est refusé ; un asset déjà
    /// manquant avant l'op (deck importé) reste au `validate` de le signaler.
    pub(crate) fn embed_op_assets(&mut self, next: &Deck, registry: &WidgetRegistry) -> Result<()> {
        let before = oku_slides_core::asset_refs(self.doc.sources.deck(), registry);
        let mut found = Vec::new();
        for id in oku_slides_core::asset_refs(next, registry) {
            if before.contains(&id) || self.doc.sources.assets.contains_key(&id) {
                continue;
            }
            match self.attachments.get(&id) {
                Some((_, bytes)) => found.push(bytes),
                None => return UnknownAssetSnafu { asset: id }.fail()?,
            }
        }
        // Tout est vérifié avant d'ajouter : un refus n'ajoute rien.
        for bytes in found {
            self.doc.sources.add_asset(bytes.to_vec());
        }
        Ok(())
    }
```

(imports: `oku_slides_core::{Deck, WidgetRegistry}`, `crate::error::UnknownAssetSnafu`; `return …Snafu.fail()?` is the form `deck_widgets.rs` already uses for `NotRenderedSnafu`.)

`deck_widgets.rs`, in `Session::apply`, right after `let moves = oku_slides_core::apply_all_moving(&mut next, ops, &registry).context(OpSnafu)?;`:

```rust
        // Une image collée que le deck cite désormais entre dans les sources (en ajout seul).
        self.embed_op_assets(&next, &registry)?;
```

`session.ftl` ×20:

```ftl
# en
session-asset-unknown = This image is neither in the deck nor among the pasted images: use the id of a pasted image.
# fr
session-asset-unknown = Cette image n'est ni dans le deck ni parmi les images collées : donnez l'identifiant d'une image collée.
```

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p oku-slides-session && cargo nextest run -p oku-slides-i18n`
Expected: PASS (whole session crate: `apply` is on every edit path).

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-core crates/oku-slides-session crates/oku-slides-i18n
git commit -F <scratchpad>/msg-t3.txt   # feat(session) : une image collée entre dans le deck quand une op la cite, un asset inconnu est refusé (t-1164)
```

---

### Task 4: `paste_image` and `remove_image` on the host draft

**Files:**
- Create: `crates/oku-slides-agent/src/paste.rs`, `crates/oku-slides-agent/tests/paste.rs`
- Modify: `crates/oku-slides-agent/Cargo.toml` (`base64.workspace = true`; dev-dependency `png.workspace = true`), `src/lib.rs` (`mod paste;`), `src/stack.rs` (variants and arms), `src/tab_entry.rs` (`Draft`, `TabLink.session`), `src/events.rs` (`Draft.images`), `src/command_error.rs`, `src/command_error_tests.rs`, `tests/common/mod.rs` (helpers moved from `tests/stack.rs`), `tests/stack.rs`, `crates/oku-slides-agent/ARCHITECTURE.md`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 files)

**Interfaces:**
- Consumes: `Session::paste_image`, `AttachmentRef` (Task 2), `PasteSettings.max_draft_images`.
- Produces:
  - JSON commands `{"cmd":"paste_image","tab":…,"data":"<base64>"}` and `{"cmd":"remove_image","tab":…,"id":"sha256:…"}`.
  - `AgentEvent::Draft { text: Option<String>, origin: Option<String>, images: Vec<AttachmentRef> }` (JSON `{"kind":"draft","text":…,"origin":…,"images":[{id,mime,width,height}]}`).
  - `TabLink::draft_images(&self) -> Vec<AttachmentRef>`; `TabLink.session: TabSession`.
  - Fluent keys `agent-error-too-many-images`, `agent-error-image-not-base64`, plus `CommandInner::PasteRefused` localized by its session source.

- [ ] **Step 1: Move the stack test helpers to `tests/common`**

Move `is`, `next`, `shown` and `drafted` from `tests/stack.rs` (lines 383-425) to the end of `tests/common/mod.rs`, made `pub`; in `tests/stack.rs` import them from `common`. Run `cargo nextest run -p oku-slides-agent --test stack` — Expected: PASS (pure move).

- [ ] **Step 2: Write the failing tests**

`crates/oku-slides-agent/tests/paste.rs`:

```rust
//! Images collées dans le brouillon : dépôt, retrait, refus.

mod common;

use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use common::{cmd, drafted, is, next, shown, stack_with_sink};
use serde_json::json;
use speculoos::prelude::*;

/// Un PNG valide de 2 × 2 pixels, propre à `seed`, en base64.
fn png64(seed: u8) -> String {
    let mut out = Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, 2, 2);
    e.set_color(png::ColorType::Grayscale);
    let mut w = e.write_header().unwrap();
    w.write_image_data(&[seed, 1, 2, 3]).unwrap();
    drop(w);
    STANDARD.encode(out.into_inner())
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_adds_it_to_the_broadcast_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("pose-la"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(1) }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let image = &draft["event"]["images"][0];
    assert_that!((image["mime"].clone(), image["width"].clone(), draft["event"]["text"].clone()))
        .is_equal_to((json!("image/png"), json!(2), json!("pose-la")));
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_on_a_closed_draft_opens_it_empty() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", None, from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(2) }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(""));
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_twice_lists_it_once() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(3) }))).unwrap();
    let (at, _) = next(&rx, from, is("agent", "draft", "t-1")).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(3) }))).unwrap();
    let (_, draft) = next(&rx, at + 1, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["images"].as_array().map(Vec::len)).is_equal_to(Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_past_the_draft_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    drafted(&stack, &rx, "t-1", Some("x"), from).await;
    for seed in 0..8 {
        stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(10 + seed) }))).unwrap();
    }
    let refused = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(30) })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_that_is_not_an_image_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1", None).await;
    let data = STANDARD.encode(b"bonjour");
    let refused = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": data })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_on_a_tab_without_agent_is_refused() {
    let (stack, _rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let refused = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(4) })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_image_drops_it_from_the_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(5) }))).unwrap();
    let (at, pasted) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let id = pasted["event"]["images"][0]["id"].clone();
    stack.handle(cmd(json!({ "cmd": "remove_image", "tab": "t-1", "id": id }))).unwrap();
    let (_, draft) = next(&rx, at + 1, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["images"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_null_drops_the_images() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1", None).await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(6) }))).unwrap();
    let (at, _) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let from = drafted(&stack, &rx, "t-1", None, at + 1).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["images"].clone()).is_equal_to(json!([]));
}
```

Also update the existing assertion in `tests/stack.rs::set_draft_is_broadcast_with_its_tab_and_origin` to expect `"images": []` in the event JSON.

In `command_error_tests.rs`:

```rust
#[test]
fn too_many_images_is_said_in_the_user_language() {
    let e = CommandError::from(TooManyImagesSnafu { max: 8usize }.build());
    assert_that!(e.localize(&Localizer::new(Some("fr")))).is_equal_to("Le brouillon a déjà toutes ses images.".to_owned());
}
```

Run: `cargo nextest run -p oku-slides-agent --test paste command_error` — Expected: FAIL (unknown variant `paste_image`).

- [ ] **Step 3: Implement**

`Cargo.toml`: `base64.workspace = true` under `[dependencies]`, `png.workspace = true` under `[dev-dependencies]`.

`command_error.rs`, new variants:

```rust
    #[snafu(display("draft already holds {max} images"))]
    TooManyImages { max: usize, backtrace: Backtrace },
    #[snafu(display("pasted image data is not base64"))]
    ImageNotBase64 { source: base64::DecodeError, backtrace: Backtrace },
    /// Cause boxée : comme `AgentNotSaved`.
    #[snafu(display("pasted image refused by the session"))]
    PasteRefused {
        #[snafu(source(from(oku_slides_session::SessionError, Box::new)))]
        source: Box<oku_slides_session::SessionError>,
        backtrace: Backtrace,
    },
```

and in `localize`:

```rust
            CommandInner::TooManyImages { .. } => l.tr("agent-error-too-many-images"),
            CommandInner::ImageNotBase64 { .. } => l.tr("agent-error-image-not-base64"),
            CommandInner::PasteRefused { source, .. } => source.localize(l),
```

`events.rs`, `Draft` gains (after `origin`):

```rust
        /// Les images collées du brouillon ; l'hôte en est seul juge, chaque vue les reprend.
        images: Vec<AttachmentRef>,
```

(`use oku_slides_session::agent::AttachmentRef;`).

`tab_entry.rs`:

```rust
/// Le brouillon d'un onglet : son texte (`None` : fermé) et ses images collées.
#[derive(Debug, Clone, Default)]
pub(crate) struct Draft {
    pub text: Option<String>,
    pub images: Vec<AttachmentRef>,
}
```

`TabLink.draft: Arc<Mutex<Draft>>`, new field `pub session: TabSession` (doc `/// La session de l'onglet : magasin des images collées.`), filled in `TabEntry::start` with `session.clone()` before `session` moves into `Agent`. Methods:

```rust
    /// Garde le texte du brouillon et le diffuse ; le fermer (`None`) en retire les images.
    pub fn set_draft(&self, out: &mpsc::UnboundedSender<Out>, text: Option<String>, origin: Option<String>) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        if text.is_none() {
            draft.images.clear();
        }
        draft.text.clone_from(&text);
        let images = draft.images.clone();
        emit(out, "agent", &self.tab, &AgentEvent::Draft { text, origin, images });
    }

    /// Ajoute une image au brouillon (une fois), en l'ouvrant vide s'il était fermé.
    pub fn add_image(&self, out: &mpsc::UnboundedSender<Out>, image: AttachmentRef) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        if !draft.images.iter().any(|i| i.id == image.id) {
            draft.images.push(image);
        }
        let text = Some(draft.text.take().unwrap_or_default());
        draft.text.clone_from(&text);
        emit(out, "agent", &self.tab, &AgentEvent::Draft { text, origin: None, images: draft.images.clone() });
    }

    /// Retire l'image `id` du brouillon.
    pub fn remove_image(&self, out: &mpsc::UnboundedSender<Out>, id: &str) {
        let mut draft = self.draft.lock().unwrap_or_else(PoisonError::into_inner);
        draft.images.retain(|i| i.id != id);
        let event = AgentEvent::Draft { text: draft.text.clone(), origin: None, images: draft.images.clone() };
        emit(out, "agent", &self.tab, &event);
    }

    /// Les images du brouillon.
    pub fn draft_images(&self) -> Vec<AttachmentRef> {
        self.draft.lock().unwrap_or_else(PoisonError::into_inner).images.clone()
    }
```

`replay` emits `AgentEvent::Draft { text: draft.text.clone(), origin: None, images: draft.images.clone() }`.

`paste.rs`:

```rust
//! Images collées : dépôt dans la session de l'onglet, puis dans son brouillon.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oku_slides_platform::config::PasteSettings;
use snafu::{ResultExt, ensure};
use tokio::sync::mpsc;

use crate::command_error::{CommandError, ImageNotBase64Snafu, PasteRefusedSnafu, TooManyImagesSnafu};
use crate::tab_entry::{Out, TabLink};

/// Décode, valide et garde l'image `data` (base64), puis l'ajoute au brouillon de `t`.
///
/// Bloquant (verrou de la session) : depuis `Stack::handle`, jamais sous le verrou du brouillon.
pub(crate) fn paste_image(
    t: &TabLink,
    out: &mpsc::UnboundedSender<Out>,
    settings: &PasteSettings,
    data: &str,
) -> Result<(), CommandError> {
    // Le `?` convertit chaque `CommandInner` en `CommandError` (`From` dérivé par Snafu).
    let bytes = STANDARD.decode(data).context(ImageNotBase64Snafu)?;
    let keep: Vec<String> = t.draft_images().into_iter().map(|i| i.id).collect();
    let max = settings.max_draft_images;
    if keep.len() >= max {
        return Err(TooManyImagesSnafu { max }.build().into());
    }
    let image = t.session.with(|s| s.paste_image(bytes, &keep)).and_then(|r| r).context(PasteRefusedSnafu)?;
    t.add_image(out, image);
    Ok(())
}
```

(imports: drop `ensure` from the `snafu` line.) Its behaviour is covered by `tests/paste.rs`; no unit test file. In `lib.rs`, add `mod paste;` after `mod model_download;`.

`stack.rs`, variants (after `SetDraft`):

```rust
    /// Une image collée (`data` : ses octets en base64), ajoutée au brouillon de l'onglet.
    ///
    /// Refusée sans agent démarré, au-delà de `[paste] max_draft_images`, ou par la session.
    PasteImage {
        tab: String,
        data: String,
    },
    /// Retire l'image `id` du brouillon.
    RemoveImage {
        tab: String,
        id: String,
    },
```

`tab()` gains `| Self::PasteImage { tab, .. } | Self::RemoveImage { tab, .. }`. Arms in `handle`:

```rust
            AgentCommand::PasteImage { tab, data } => {
                let l = self.platform.live_localizer.current();
                let Some(t) = self.tab(&tab) else {
                    return Err(CommandError::from(NoAgentSnafu.build()).localize(&l));
                };
                return crate::paste::paste_image(&t, &self.out_tx, &self.platform.settings.paste, &data).map_err(|e| {
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "image collée refusée");
                    e.localize(&l)
                });
            }
            AgentCommand::RemoveImage { tab, id } => match self.tab(&tab) {
                Some(t) => t.remove_image(&self.out_tx, &id),
                None => {
                    let e = CommandError::from(NoAgentSnafu.build());
                    return Err(e.localize(&self.platform.live_localizer.current()));
                }
            },
```

`agent.ftl` ×20, after `agent-error-draft-no-agent`:

```ftl
# en
agent-error-too-many-images = The draft already has all its images.
agent-error-image-not-base64 = The pasted image did not arrive whole.
# fr
agent-error-too-many-images = Le brouillon a déjà toutes ses images.
agent-error-image-not-base64 = L'image collée n'est pas arrivée entière.
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-agent && cargo nextest run -p oku-slides-i18n`
Expected: PASS.

- [ ] **Step 5: Update `crates/oku-slides-agent/ARCHITECTURE.md`**

In the `tab_entry` bullet, replace "and holds the draft (`TabLink`)" by "and holds the draft (`TabLink`: text and pasted images, `session`)". Add a bullet after `tab_agent`: `` - `paste` : `paste_image` (base64, `[paste] max_draft_images`, `Session::paste_image`), the draft's images. `` At most 50 lines.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-agent crates/oku-slides-i18n
git commit -F <scratchpad>/msg-t4.txt   # feat(agent) : paste_image et remove_image, images du brouillon tenues par l'hôte (t-1165)
```

---

### Task 5: Send pasted images with the prompt

**Files:**
- Create: `crates/oku-slides-agent/src/prompt_images.rs`, `crates/oku-slides-agent/src/prompt_images_tests.rs`
- Modify: `crates/oku-slides-agent/src/stack.rs` (`Prompt.images`), `src/paste.rs` (+ `resolve_images`), `src/tab_agent.rs` (`Instruction.images`, `AgentHandle::spawn` gains `prompt_image_bytes`, `Ctx.prompt_image_bytes`), `src/tab_entry.rs` (`Agent.prompt_image_bytes`, the tools-failure path), `src/link.rs` (`SessionCaps.image`), `src/turn.rs`, `src/lib.rs`, `src/command_error.rs`, `src/instructions/agent.md`, `src/bin/fake_agent.rs` (`prompt_image`), `tests/common/mod.rs:147`, `src/tab_entry_tests.rs`, `tests/paste.rs`, `crates/oku-slides-agent/ARCHITECTURE.md`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 files)

**Interfaces:**
- Consumes: `Session::pasted_image` (Task 2), `TabLink.session`, `draft` (Task 4), `PasteSettings::prompt_image_bytes`.
- Produces:
  - JSON `{"cmd":"prompt",…,"images":["sha256:…"]}` (`images` optional, default `[]`).
  - `Instruction.images: Vec<AttachmentRef>` (used by Task 6 for the bubble).
  - `prompt_images::image_blocks(images: &[(AttachmentRef, Option<Arc<[u8]>>)], see: bool, max_bytes: usize) -> Vec<ContentBlock>`.
  - Fake agent script key `"prompt_image": true`.
  - Fluent key `agent-error-image-gone`.

- [ ] **Step 1: Write the failing unit tests**

`prompt_images_tests.rs`:

```rust
use std::sync::Arc;

use agent_client_protocol::schema::v1::ContentBlock;
use oku_slides_session::agent::AttachmentRef;
use speculoos::prelude::*;

use super::image_blocks;

fn image(id: &str, len: usize) -> (AttachmentRef, Option<Arc<[u8]>>) {
    let r = AttachmentRef { id: id.into(), mime: "image/png".into(), width: 4, height: 3 };
    (r, Some(vec![0u8; len].into()))
}

fn text_of(b: &ContentBlock) -> &str {
    match b {
        ContentBlock::Text(t) => &t.text,
        _ => "",
    }
}

#[test]
fn image_blocks_list_the_ids_then_one_image_block_each() {
    let blocks = image_blocks(&[image("sha256:aa", 8), image("sha256:bb", 8)], true, 100);
    assert_that!(blocks.len()).is_equal_to(3);
    assert_that!(text_of(&blocks[0])).contains("1. sha256:aa image/png 4×3");
    assert_that!(matches!(&blocks[2], ContentBlock::Image(i) if i.mime_type == "image/png")).is_true();
}

#[test]
fn image_blocks_for_an_agent_without_images_only_list_them() {
    let blocks = image_blocks(&[image("sha256:aa", 8)], false, 100);
    assert_that!(blocks.len()).is_equal_to(1);
    assert_that!(text_of(&blocks[0])).contains("not shown: this agent does not receive images");
}

#[test]
fn image_blocks_over_the_prompt_limit_are_only_listed() {
    let blocks = image_blocks(&[image("sha256:aa", 200)], true, 100);
    assert_that!(blocks.len()).is_equal_to(1);
    assert_that!(text_of(&blocks[0])).contains("not shown: too large to show you");
}

#[test]
fn image_blocks_of_a_forgotten_image_say_it_is_gone() {
    let gone = (AttachmentRef { id: "sha256:cc".into(), mime: "image/png".into(), width: 1, height: 1 }, None);
    let blocks = image_blocks(&[gone], true, 100);
    assert_that!(text_of(&blocks[0])).contains("no longer available");
}

#[test]
fn image_blocks_without_images_are_empty() {
    assert_that!(image_blocks(&[], true, 100)).is_empty();
}
```

Run: `cargo nextest run -p oku-slides-agent prompt_images` — Expected: FAIL (module missing).

- [ ] **Step 2: Write `prompt_images.rs`**

```rust
//! Les blocs du prompt pour les images collées : leur liste, puis celles que l'agent voit.

use std::sync::Arc;

use agent_client_protocol::schema::v1::{ContentBlock, ImageContent, TextContent};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oku_slides_session::agent::AttachmentRef;

/// Les blocs des images de la consigne, avant son texte ; aucun sans image.
///
/// `see` : l'agent annonce `promptCapabilities.image`. Une image plus lourde que `max_bytes`,
/// ou oubliée par l'onglet (`None`), n'est que listée, avec la raison.
pub(crate) fn image_blocks(images: &[(AttachmentRef, Option<Arc<[u8]>>)], see: bool, max_bytes: usize) -> Vec<ContentBlock> {
    if images.is_empty() {
        return Vec::new();
    }
    let mut list = String::from(
        "Pasted images (pasted by the user with this message; place one with apply: \
         add_element {\"kind\": \"base/image\", \"params\": {\"asset\": \"<id>\"}}, or set_param asset \
         on an existing image; never write any other asset id):\n",
    );
    let mut shown = Vec::new();
    for (n, (r, bytes)) in images.iter().enumerate() {
        let note = match bytes {
            None => " (no longer available: ask the user to paste it again)",
            Some(_) if !see => " (not shown: this agent does not receive images)",
            Some(b) if b.len() > max_bytes => " (not shown: too large to show you)",
            Some(b) => {
                shown.push(ContentBlock::Image(ImageContent::new(STANDARD.encode(b), r.mime.clone())));
                ""
            }
        };
        list.push_str(&format!("{}. {} {} {}×{}{note}\n", n + 1, r.id, r.mime, r.width, r.height));
    }
    let mut blocks = vec![ContentBlock::Text(TextContent::new(list))];
    blocks.extend(shown);
    blocks
}

#[cfg(test)]
#[path = "prompt_images_tests.rs"]
mod prompt_images_tests;
```

`lib.rs`: `mod prompt_images;` after `mod picker;`.

Run: `cargo nextest run -p oku-slides-agent prompt_images` — Expected: PASS.

- [ ] **Step 3: Write the failing integration tests**

Append to `tests/paste.rs`:

```rust
use common::{logged, stack_with_sink_and_log, wait_turn_done};

/// Dépose une image dans le brouillon de `tab` et rend son id.
async fn pasted(stack: &common::TestStack, rx: &common::Relayed, tab: &'static str, seed: u8) -> String {
    let from = shown(stack, rx, tab, None).await;
    let from = drafted(stack, rx, tab, Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": tab, "data": png64(seed) }))).unwrap();
    let (_, draft) = next(rx, from, is("agent", "draft", tab)).await;
    draft["event"]["images"][0]["id"].as_str().unwrap().to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_image_lists_it_then_shows_it_before_the_text() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "prompt_image": true, "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 40).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "images": [id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let blocks = logged(&log, "session/prompt")[0]["prompt"].as_array().cloned().unwrap();
    let n = blocks.len();
    assert_that!(blocks[n - 3]["text"].as_str().unwrap()).contains(id.as_str());
    assert_that!((blocks[n - 2]["type"].clone(), blocks[n - 2]["mimeType"].clone()))
        .is_equal_to((json!("image"), json!("image/png")));
    assert_that!(blocks[n - 1]["text"].clone()).is_equal_to(json!("pose-la"));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_image_to_an_agent_without_images_sends_only_its_id() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 41).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "images": [id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = logged(&log, "session/prompt")[0]["prompt"].to_string();
    assert_that!((prompt.contains(&id), prompt.contains("\"type\":\"image\""))).is_equal_to((true, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_unknown_image_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1", None).await;
    let refused = stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "x", "images": ["sha256:00"] })));
    assert_that!(refused).is_err();
}
```

Run: `cargo nextest run -p oku-slides-agent --test paste prompt_` — Expected: FAIL (`images` ignored: no image block).

- [ ] **Step 4: Implement the prompt path**

`command_error.rs`: `#[snafu(display("pasted image {id} is no longer kept"))] ImageGone { id: String, backtrace: Backtrace },` localized `l.tr("agent-error-image-gone")`. `agent.ftl` ×20:

```ftl
# en
agent-error-image-gone = A pasted image is no longer available: paste it again.
# fr
agent-error-image-gone = Une image collée n'est plus disponible : collez-la de nouveau.
```

`paste.rs`:

```rust
/// Les images `ids` de la consigne, toutes gardées par l'onglet, sinon une erreur.
pub(crate) fn resolve_images(t: &TabLink, ids: &[String]) -> Result<Vec<AttachmentRef>, CommandError> {
    ids.iter()
        .map(|id| match t.session.with(|s| s.pasted_image(id).map(|(r, _)| r)) {
            Ok(Some(r)) => Ok(r),
            Ok(None) | Err(_) => Err(CommandError::from(ImageGoneSnafu { id: id.clone() }.build())),
        })
        .collect()
}
```

`stack.rs`, `Prompt` gains:

```rust
        /// Les images collées envoyées avec la consigne (ids `sha256:…` du brouillon).
        #[serde(default)]
        images: Vec<String>,
```

and its arm becomes (answer first, shortcut second, both only without images, per `crates/oku-slides-agent/AGENTS.md`):

```rust
            AgentCommand::Prompt { tab, scope, text, ui, answer, images } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                // Une réponse ou un mot-raccourci ne portent pas d'image : avec une, c'est une consigne.
                if images.is_empty() && answer.is_some_and(|id| t.hub.answer(&id, &text)) {
                    return Ok(());
                }
                if images.is_empty() && matches_shortcut(&text, THEME_SHORTCUT, &self.platform.live_localizer.current()) {
                    /* unchanged show_themes block */
                }
                let images = crate::paste::resolve_images(&t, &images).map_err(|e| {
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "consigne refusée");
                    e.localize(&self.platform.live_localizer.current())
                })?;
                t.send(Command::Prompt(Instruction { scope, text, ui, tune: false, images }));
            }
```

Keep the existing comments of the two checks. `Tune` sends `images: Vec::new()`.

`tab_agent.rs`: `Instruction` gains `/// Les images collées envoyées avec la consigne.` `pub images: Vec<AttachmentRef>,`; `AgentHandle::prompt_in` builds it with `images: Vec::new()`; `Ctx` gains `/// Plus grande image envoyée en bloc image ([paste] prompt_image_mib).` `pub prompt_image_bytes: usize,`; `AgentHandle::spawn` gains a last parameter `prompt_image_bytes: usize` stored in `Ctx`. Its call in `run` (around line 251) passes `live.caps.image`:

```rust
        match crate::turn::run(&self.ctx, &live.cx, rx, n, id.clone(), instruction, fresh, live.caps.image, prompting).await {
```

`tab_entry.rs`: `Agent` gains `prompt_image_bytes: usize` (from `platform.settings.paste.prompt_image_bytes()` in `TabEntry::start`), passed to `AgentHandle::spawn`. `tests/common/mod.rs:147` passes `platform.settings.paste.prompt_image_bytes()`. `src/tab_entry_tests.rs:51` adds `images: Vec::new()`.

`link.rs`: `SessionCaps` gains `/// \`ContentBlock::Image\` dans un prompt.` `pub image: bool,`; in `handshake`: `SessionCaps { load: caps.load_session, resume: caps.session_capabilities.resume.is_some(), image: caps.prompt_capabilities.image }`.

`turn.rs`: `run` gains `see_images: bool` before `prompting`, update its doc ("… le contexte, les images collées, puis le texte de la consigne, toujours en dernier"); destructure `images` from the instruction; fetch the bytes with the context in the same blocking call:

```rust
    let Instruction { scope, text, ui, tune, images } = instruction;
    // …
    let blocking = tokio::task::spawn_blocking(move || {
        tab.with(|s| {
            s.begin_turn(turn, &text_for_begin);
            let pasted: Vec<_> = images.into_iter().map(|r| {
                let bytes = s.pasted_image(&r.id).map(|(_, b)| b);
                (r, bytes)
            }).collect();
            (s.agent_context(&ui), pasted)
        })
    })
    .await;
    let (context, pasted) = match blocking {
        Ok(Ok(x)) => x,
        Ok(Err(e)) => { /* same warn */ (String::new(), Vec::new()) }
        Err(e) => { /* same warn */ (String::new(), Vec::new()) }
    };
    // …
    blocks.push(ContentBlock::Text(TextContent::new(context)));
    blocks.extend(crate::prompt_images::image_blocks(&pasted, see_images, ctx.prompt_image_bytes));
    blocks.push(ContentBlock::Text(TextContent::new(text)));
```

When the tab session is gone (`Ok(Err(_))`, already logged), the turn has neither context nor images: there is no session to read them from.

`instructions/agent.md`: add one paragraph at the end of the section about editing the deck:

```markdown
Pasted images: a prompt may list "Pasted images" with their asset ids; they are the user's. Place one with apply (add_element base/image with params.asset set to its id, or set_param asset on an existing image). Use only ids listed in this conversation; never invent one.
```

`fake_agent.rs`: `Script` gains

```rust
    /// Annonce `promptCapabilities.image` (blocs image dans un prompt).
    #[serde(default)]
    prompt_image: bool,
```

and the `initialize` response adds `.prompt_capabilities(PromptCapabilities::new().image(s1.script.prompt_image))` (import `PromptCapabilities`).

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p oku-slides-agent && cargo nextest run -p oku-slides-i18n && cargo nextest run -p oku-slides-local-agent`
Expected: PASS (the local agent still reads only the last text block).

- [ ] **Step 6: Update `crates/oku-slides-agent/ARCHITECTURE.md`**

Extend the `paste` bullet: ``, `resolve_images` (ids of a prompt)``; add `` `prompt_images` (the "Pasted images" text block, then image blocks when `SessionCaps.image` and within `[paste] prompt_image_mib`) `` to the `turn` line. At most 50 lines.

- [ ] **Step 7: Commit**

```bash
git add crates/oku-slides-agent crates/oku-slides-i18n
git commit -F <scratchpad>/msg-t5.txt   # feat(agent) : les images collées partent avec la consigne, en blocs image si l'agent les reçoit (t-1166)
```

---

### Task 6: Pasted images in the thread bubble

**Files:**
- Modify: `crates/oku-slides-a2ui/src/message.rs:227`, `crates/oku-slides-a2ui/src/lib.rs`, `crates/oku-slides-a2ui/src/message_tests.rs:439`
- Modify: `crates/oku-slides-session/src/agent/turn_view.rs` (`TurnView.images`, bubble), `turn_view_tests.rs`, `forms_tests.rs:362`, `crates/oku-slides-session/src/agent/attachments.rs` (`AttachmentRef::shown`)
- Modify: `crates/oku-slides-mcp/src/hub_scopes_tests.rs:35`
- Modify: `crates/oku-slides-agent/src/events.rs` (`UserText.images`), `src/tab_agent.rs:210`, `src/tab_entry.rs:281`, `src/turns.rs:329,431-446`, every `AgentEvent::UserText {` in `src/turns_tests.rs`, `src/events_tests.rs`, `tests/scopes.rs`, `tests/turns.rs`, `tests/launch.rs`, `tests/common/mod.rs:231`
- Modify: `ui/src/a2ui/turnComponents.tsx`, `ui/src/a2ui/a2ui.css`, `ui/src/a2ui/catalog.test.tsx`, `ui/src/a2ui/catalog.render.test.tsx`
- Create: `ui/src/agent/pastedImages.ts` (only `imageLabel` in this task), `ui/src/agent/pastedImages.test.ts`
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 files)

**Interfaces:**
- Consumes: `Instruction.images` (Task 5), `AttachmentRef` (Task 2).
- Produces:
  - `oku_slides_a2ui::PromptImage { mime: String, width: u32, height: u32 }`; `ComponentKind::Prompt { text, images: Vec<PromptImage> }` (JSON `images` omitted when empty).
  - `AttachmentRef::shown(&self) -> PromptImage`.
  - TS `imageLabel(mime: string, width: number, height: number): string` in `ui/src/agent/pastedImages.ts`; key `ui-pasted-image`.

- [ ] **Step 1: Write the failing tests**

`message_tests.rs`:

```rust
#[test]
fn prompt_without_images_serializes_no_images_field() {
    let p = Component::new("p", ComponentKind::Prompt { text: Dynamic::Literal("x".into()), images: Vec::new() });
    assert_that!(serde_json::to_value(&p).unwrap().get("images").is_none()).is_true();
}

#[test]
fn prompt_image_with_an_unknown_field_is_refused() {
    let v = json!({ "id": "p", "component": "Prompt", "text": "x", "images": [{ "mime": "image/png", "width": 1, "height": 1, "data": "…" }] });
    assert_that!(serde_json::from_value::<Component>(v)).is_err();
}
```

`turn_view_tests.rs`:

```rust
#[test]
fn turn_surface_prompt_bubble_carries_the_pasted_images() {
    let image = oku_slides_a2ui::PromptImage { mime: "image/png".into(), width: 4, height: 3 };
    let v = TurnView { images: vec![image.clone()], ..view(TurnState::Running) };
    let s = turn_surface(&v, &Localizer::new(Some("en")));
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json.as_str()).contains("\"images\":[{\"mime\":\"image/png\",\"width\":4,\"height\":3}]");
}
```

`ui/src/agent/pastedImages.test.ts`:

```ts
import { beforeAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { applyMessages } from "../i18n";
import { imageLabel } from "./pastedImages";

const ftl = (l: string) => readFileSync(`${import.meta.dir}/../../../crates/oku-slides-i18n/src/locales/${l}/ui.ftl`, "utf8");
beforeAll(() => applyMessages({ locale: "en", resources: [ftl("en")], chosen: null }));

test("imageLabel gives the short type and the size", () => {
  expect(imageLabel("image/png", 1200, 800)).toBe("PNG 1200×800");
});

test("imageLabel of jpeg says JPEG", () => {
  expect(imageLabel("image/jpeg", 2, 1)).toBe("JPEG 2×1");
});
```

`catalog.render.test.tsx`, in `describe("composants du fil (v3)")`:

```ts
  test("Prompt montre une puce par image collée", async () => {
    const { host } = await mount({ component: "Prompt", text: "Pose-la", images: [{ mime: "image/png", width: 4, height: 3 }] });
    expect([...host.querySelectorAll(".a2ui-prompt-image")].map((e) => e.textContent)).toEqual(["PNG 4×3"]);
  });
```

`catalog.test.tsx`, a new row in the `test.each([...])("refuse %s", …)` table:

```ts
    ["une image de consigne avec ses octets", { component: "Prompt", text: "x", images: [{ mime: "image/png", width: 1, height: 1, data: "x" }] }, /Prompt/],
```

and, next to the test that lists the thread components (`typesOf([... { id: "p", component: "Prompt", text: "Change le fond" } ...])`), one accepting images:

```ts
  test("une consigne avec ses images collées est acceptée", () => {
    expect(typesOf([{ id: "root", component: "Prompt", text: "x", images: [{ mime: "image/png", width: 4, height: 3 }] }])).toEqual({ root: "Prompt" });
  });
```

Run: `cargo nextest run -p oku-slides-a2ui -p oku-slides-session turn_view prompt && (cd ui && bun test src/agent/pastedImages.test.ts src/a2ui/catalog.test.tsx && bun test src/a2ui/catalog.render.test.tsx)`
Expected: FAIL (no `images` field).

- [ ] **Step 2: Implement (Rust)**

`message.rs`, before `TimelineStep`:

```rust
/// Une image collée avec la consigne, montrée en puce : son type et sa taille, jamais ses octets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptImage {
    pub mime: String,
    pub width: u32,
    pub height: u32,
}
```

and `ComponentKind::Prompt`:

```rust
    /// La consigne de l'utilisateur et ses images collées (hôte seul).
    Prompt {
        text: Dynamic<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        images: Vec<PromptImage>,
    },
```

Export `PromptImage` from `lib.rs` (and re-export it from `oku_slides_session::agent` next to `TimelineStep`). Fix the `ComponentKind::Prompt { text: … }` literals listed above with `images: Vec::new()`.

`attachments.rs`:

```rust
impl AttachmentRef {
    /// Ce que la bulle du fil montre de l'image.
    pub fn shown(&self) -> oku_slides_a2ui::PromptImage {
        oku_slides_a2ui::PromptImage { mime: self.mime.clone(), width: self.width, height: self.height }
    }
}
```

`turn_view.rs`: `TurnView` gains `/// Les images collées avec la consigne.` `pub images: Vec<PromptImage>,`; the bubble becomes `ComponentKind::Prompt { text: Dynamic::Literal(v.prompt.clone()), images: v.images.clone() }`. Fix every `TurnView {` literal listed above with `images: Vec::new()` (in `turns.rs` the value comes from the event, below).

`events.rs`, `UserText` gains:

```rust
        /// Les images collées avec la consigne.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        images: Vec<PromptImage>,
```

`tab_agent.rs:210`: `images: instruction.images.iter().map(AttachmentRef::shown).collect()`. `tab_entry.rs:281`: destructure `images` and pass the same map. `turns.rs`: `AgentEvent::UserText { turn, scope, text, images } => return self.open(*turn, scope.clone(), text, images, now, l)`, `open` gains `images: &[PromptImage]` and sets `images: images.to_vec()` in the `TurnView`. Every other `AgentEvent::UserText {` literal listed above gains `images: Vec::new()` (or `..` in patterns).

- [ ] **Step 3: Implement (UI)**

`ui/src/agent/pastedImages.ts`:

```ts
// Images collées dans le brouillon de l'agent : ce que l'UI en lit et en montre.

import { t } from "../i18n";

/** Le libellé d'une image : son type court et sa taille, « PNG 1200×800 ». */
export function imageLabel(mime: string, width: number, height: number): string {
  const type = mime.replace(/^image\//, "").toUpperCase();
  return t("ui-pasted-image", { type, width: String(width), height: String(height) });
}
```

`turnComponents.tsx`:

```ts
const PromptImage = z.object({ mime: z.string(), width: z.number().int(), height: z.number().int() }).strict();
const PromptApi = {
  name: "Prompt",
  schema: z.object({ text: CommonSchemas.DynamicString, images: z.array(PromptImage).optional() }).strict(),
};
```

```tsx
/** La consigne de l'utilisateur, et une puce par image collée. */
export const Prompt = createComponentImplementation(PromptApi, ({ props }) => (
  <div className="a2ui-prompt-turn">
    <p className="a2ui-prompt">{props.text}</p>
    {props.images && props.images.length > 0 && (
      <ul className="a2ui-prompt-images" aria-label={t("ui-draft-images")}>
        {props.images.map((i, n) => (
          <li key={n} className="a2ui-prompt-image">{imageLabel(i.mime, i.width, i.height)}</li>
        ))}
      </ul>
    )}
  </div>
));
```

(import `imageLabel` from `../agent/pastedImages`). If wrapping `.a2ui-prompt` in a `div` changes an existing test selector (`AgentThread.render.test.tsx:220` reads `.a2ui-prompt` text, which stays the same), keep it; otherwise render the `ul` as a sibling fragment instead.

`a2ui.css`, after the `.card.prompt, .a2ui-prompt` rule:

```css
.a2ui-prompt-images { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 4px; margin: 4px 0 0; padding: 0; list-style: none; }
.a2ui-prompt-image { font: 500 12px/18px var(--sans); padding: 0 8px; border-radius: var(--radius-sm); background: color-mix(in oklab, var(--paper-200) 90%, var(--ink)); }
```

`ui.ftl` ×20, after `ui-draft-discard`:

```ftl
# en
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Pasted images
# fr
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Images collées
```

(`ui-pasted-image` is the same pattern everywhere; it is not English text copied, the i18n rule is about prose. In RTL locales keep the same order: the type and the size are code-like tokens.)

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-a2ui -p oku-slides-session -p oku-slides-mcp -p oku-slides-agent -p oku-slides-i18n && (cd ui && bun test && bunx tsc -b)`
Expected: PASS.

- [ ] **Step 5: Commit** (Rust catalog and `turnComponents.tsx` together)

```bash
git add crates ui/src
git commit -F <scratchpad>/msg-t6.txt   # feat(agent) : la bulle de la consigne montre ses images collées (t-1167)
```

---

### Task 7: Paste in the draft (UI)

**Files:**
- Modify: `ui/src/api.ts:570,629,651` (types, commands, `agentApi`)
- Modify: `ui/src/agent/pastedImages.ts`, `ui/src/agent/pastedImages.test.ts`
- Create: `ui/src/agent/usePastedImages.ts`
- Modify: `ui/src/agent/useDraft.ts`, `ui/src/agent/useDraft.render.test.tsx`, `ui/src/agent/draftActions.ts`, `ui/src/agent/draftActions.test.ts`, `ui/src/agent/AgentPanel.tsx:59-64,98,192,226-253`, `ui/src/useDictation.ts`, `ui/src/AgentWindow.tsx:154-166,276-286`, `ui/src/App.tsx:469-483`
- Modify: `ui/src/DictationDraft.tsx`, `ui/src/DictationDraft.render.test.tsx`, `ui/src/styles.css` (after `.draft-actions`)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 files)
- Modify: `ui/ARCHITECTURE.md` if it lists the draft files (check with `grep -n Draft ui/ARCHITECTURE.md`)

**Interfaces:**
- Consumes: commands `paste_image`, `remove_image`, `prompt.images` (Tasks 4–5); event `draft.images`; `imageLabel` (Task 6).
- Produces:
  - `export interface PastedImage { id: string; mime: string; width: number; height: number }` (api.ts).
  - `agentApi(tab).pasteImage(data: string)`, `.removeImage(id: string)`, `.prompt(scope, text, ui, answer, images: string[])`.
  - `DraftState.images: PastedImage[]`; `Draft.onHost(e: { text; origin; images })`.
  - `clipboardImages(dt: DataTransfer | null): File[]`, `toBase64(buf: ArrayBuffer): string`, `assetId(buf: ArrayBuffer): Promise<string | null>` in `pastedImages.ts`.
  - `usePastedImages(images, paste, say) → { thumbs: ReadonlyMap<string, string>; onPaste: (files: File[]) => void }`.
  - `DictationDraft` props `images`, `thumbs`, `onPasteImages`, `onRemoveImage`; `onSend(text, images: string[])`.
  - Keys `ui-draft-remove-image`, `ui-draft-send-needs-text`.

- [ ] **Step 1: Write the failing pure tests**

Append to `pastedImages.test.ts`:

```ts
import { assetId, clipboardImages, toBase64 } from "./pastedImages";

const file = (type: string) => new File([new Uint8Array([1, 2, 3])], "x", { type });
const clip = (text: string, files: File[]) =>
  ({ getData: (k: string) => (k === "text/plain" ? text : ""), files }) as unknown as DataTransfer;

test("clipboardImages takes the image files of a clipboard without text", () => {
  expect(clipboardImages(clip("", [file("image/png"), file("application/pdf")])).map((f) => f.type)).toEqual(["image/png"]);
});

test("clipboardImages with text keeps the text and takes no image", () => {
  expect(clipboardImages(clip("un paragraphe", [file("image/png")]))).toEqual([]);
});

test("toBase64 encodes the bytes", () => {
  expect(toBase64(new Uint8Array([104, 105]).buffer)).toBe("aGk=");
});

test("assetId is the sha256 asset id the host computes", async () => {
  expect(await assetId(new TextEncoder().encode("abc").buffer as ArrayBuffer)).toBe(
    "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
});
```

Run: `cd ui && bun test src/agent/pastedImages.test.ts` — Expected: FAIL (not exported).

- [ ] **Step 2: Write the helpers**

Append to `pastedImages.ts`:

```ts
/** Les fichiers image d'un collage ; aucun s'il porte du texte (une copie de Word y joint une image du rendu). */
export function clipboardImages(dt: DataTransfer | null): File[] {
  if (!dt || dt.getData("text/plain").trim() !== "") return [];
  return [...dt.files].filter((f) => f.type.startsWith("image/"));
}

/** Les octets en base64, par tranches : `String.fromCharCode` refuse les très longues listes. */
export function toBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

/** L'identifiant d'asset de l'hôte (`sha256:<hex>`) ; `null` hors d'un contexte sûr (pas de `crypto.subtle`). */
export async function assetId(buf: ArrayBuffer): Promise<string | null> {
  if (!globalThis.crypto?.subtle) return null;
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", buf));
  return `sha256:${[...digest].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
}
```

Run: `cd ui && bun test src/agent/pastedImages.test.ts` — Expected: PASS.

- [ ] **Step 3: Write the failing component and hook tests**

`DictationDraft.render.test.tsx`: extend `mount` with options `images = [] as PastedImage[]` and `thumbs = new Map<string, string>()`, collect `pastes: File[][]`, `removed: string[]` and `sent` as `[text, ids]` pairs. Tests:

```ts
  test("coller une image appelle onPasteImages, sans coller de texte", async () => {
    const m = await mount("Pose-la");
    const e = new Event("paste", { bubbles: true, cancelable: true });
    const png = new File([new Uint8Array([137, 80])], "x.png", { type: "image/png" });
    Object.defineProperty(e, "clipboardData", { value: { getData: () => "", files: [png] } });
    await act(async () => m.note().dispatchEvent(e));
    expect([m.pastes.length, e.defaultPrevented]).toEqual([1, true]);
  });

  test("coller du texte ne joint aucune image", async () => {
    const m = await mount("Pose-la");
    const e = new Event("paste", { bubbles: true, cancelable: true });
    const png = new File([new Uint8Array([137, 80])], "x.png", { type: "image/png" });
    Object.defineProperty(e, "clipboardData", { value: { getData: () => "du texte", files: [png] } });
    await act(async () => m.note().dispatchEvent(e));
    expect(m.pastes.length).toBe(0);
  });

  test("une puce par image, son bouton la retire", async () => {
    const m = await mount("Pose-la", { images: [{ id: "sha256:aa", mime: "image/png", width: 4, height: 3 }] });
    expect(m.host.querySelector(".draft-image")?.textContent).toContain("PNG 4×3");
    await act(async () => (m.host.querySelector(".draft-image button") as HTMLButtonElement).click());
    expect(m.removed).toEqual(["sha256:aa"]);
  });

  test("la miniature locale s'affiche quand la vue a l'image", async () => {
    const m = await mount("x", { images: [{ id: "sha256:aa", mime: "image/png", width: 4, height: 3 }], thumbs: new Map([["sha256:aa", "blob:t"]]) });
    expect(m.host.querySelector(".draft-image img")?.getAttribute("src")).toBe("blob:t");
  });

  test("envoyer passe le texte et les ids des images", async () => {
    const m = await mount("Pose-la", { images: [{ id: "sha256:aa", mime: "image/png", width: 4, height: 3 }] });
    await act(async () => m.send().click());
    expect(m.sent).toEqual([["Pose-la", ["sha256:aa"]]]);
  });

  test("des images sans texte : l'envoi attend le texte, et le dit", async () => {
    const m = await mount("", { images: [{ id: "sha256:aa", mime: "image/png", width: 4, height: 3 }] });
    expect([m.send().disabled, m.send().title]).toEqual([true, "Dites ce qu'il faut en faire"]);
  });
```

(The French title is the `fr` value of `ui-draft-send-needs-text` below; this file loads `fr`.)

`useDraft.render.test.tsx`, in `describe("useDraft")` (its `mount` records each send's origin in `m.origins`):

```ts
  const image = { id: "sha256:aa", mime: "image/png", width: 1, height: 1 };

  test("useDraft reprend les images de l'hôte, même dans l'écho d'un envoi de la vue", async () => {
    const m = await mount();
    await act(async () => m.draft().append("x"));
    await act(async () => m.draft().onHost({ text: "x", origin: m.origins[0]!, images: [image] }));
    expect(m.draft().state.images).toEqual([image]);
  });

  test("useDraft close vide aussi les images", async () => {
    const m = await mount();
    await act(async () => m.draft().append("x"));
    await act(async () => m.draft().onHost({ text: "x", origin: null, images: [image] }));
    await act(async () => m.draft().close());
    expect(m.draft().state.images).toEqual([]);
  });
```

Every existing `onHost({ text, origin })` call in this file gains `images: []`, and the assertion `toEqual({ text: "bonjour", dictations: 1, typing: 0 })` gains `images: []`.

`draftActions.test.ts`: `sendDraft("fais ceci", ["sha256:aa"])` logs `prompt:fais ceci:sha256:aa` with a `prompt` dep that logs its images.

Run: `cd ui && bun test src/agent/draftActions.test.ts && bun test src/DictationDraft.render.test.tsx src/agent/useDraft.render.test.tsx` — Expected: FAIL.

- [ ] **Step 4: Implement the API and draft state**

`api.ts`:

```ts
/** Une image collée dans le brouillon, tenue par l'hôte (`AttachmentRef`). */
export interface PastedImage {
  id: string;
  mime: string;
  width: number;
  height: number;
}
```

`AgentEvent` draft: `{ kind: "draft"; text: string | null; origin: string | null; images: PastedImage[] }`. `AgentCommand`: prompt gains `images: string[]`; add `| { cmd: "paste_image"; tab: string; data: string } | { cmd: "remove_image"; tab: string; id: string }`. `agentApi`:

```ts
    prompt: (scope: AgentScope, text: string, ui: AgentUi, answer: string | null, images: string[] = []) =>
      agentCall({ cmd: "prompt", tab, scope, text, ui: { ...ui, locale: locale() }, answer, images }),
    /** Une image collée (octets en base64), ajoutée au brouillon par l'hôte. */
    pasteImage: (data: string) => agentCall({ cmd: "paste_image", tab, data }),
    /** Retire l'image `id` du brouillon. */
    removeImage: (id: string) => agentCall({ cmd: "remove_image", tab, id }),
```

`useDraft.ts`: `DraftState` gains `images: PastedImage[]` (`EMPTY.images = []`); `onHost` takes `{ text; origin; images }` and, as its first statement:

```ts
      // Les images viennent de l'hôte seul : chaque évènement, écho compris, les donne.
      if (!sameImages(current.current.images, e.images)) write({ ...current.current, images: e.images });
```

with, at module level:

```ts
const sameImages = (a: PastedImage[], b: PastedImage[]) => a.length === b.length && a.every((x, i) => x.id === b[i]?.id);
```

`close()` writes `{ ...current.current, text: null, images: [] }`. `AgentPanel.tsx`: the `onDraft` prop type and its call at line 192 pass `images: e.images`; `AgentHandle.prompt: (text: string, images: string[]) => Promise<boolean>`; `promptAgent(text, answer, images)` forwards `images` to `agent.prompt(scope, text, props.ui, answer, images)`; `useImperativeHandle(... prompt: (text, images) => promptAgent(text, answerTarget(keysTarget), images) ...)`. Keep the existing `answer` default by making the parameter order `(text, answer = answerTarget(keysTarget), images: string[] = [])`.

`draftActions.ts`: `sendDraft: (text: string, images: string[]) => Promise<void>`; `prompt: (text: string, images: string[]) => Promise<boolean>`; `sendDraft: async (text, images) => { if (await prompt(text, images)) draft.close(); }`. `useDictation.ts` and `AgentWindow.tsx:166` adapt their `prompt` signatures (`(text, images) => handle.current?.prompt(text, images) ?? Promise.resolve(false)`).

- [ ] **Step 5: Write `usePastedImages.ts`**

```ts
// Les images collées dans une vue : chaque fichier part à l'hôte, et sa miniature reste locale
// à cette vue (URL blob du fichier de l'utilisateur), retirée quand l'image quitte le brouillon.

import { useCallback, useEffect, useRef, useState } from "react";
import type { PastedImage } from "../api";
import { assetId, toBase64 } from "./pastedImages";

export interface PastedImages {
  /** Miniatures des images collées dans cette vue, par id. */
  thumbs: ReadonlyMap<string, string>;
  /** Envoie les fichiers collés à l'hôte, un à la fois. */
  onPaste: (files: File[]) => void;
}

export function usePastedImages(images: PastedImage[], paste: (data: string) => Promise<void>, say: (m: string) => void): PastedImages {
  const [thumbs, setThumbs] = useState<ReadonlyMap<string, string>>(new Map());
  const owned = useRef(new Map<string, string>());
  const send = useRef(paste);
  send.current = paste;
  const queue = useRef(Promise.resolve());
  // Une image retirée du brouillon (ici ou ailleurs) : sa miniature est libérée.
  useEffect(() => {
    const live = new Set(images.map((i) => i.id));
    let changed = false;
    for (const [id, url] of owned.current) {
      if (!live.has(id)) {
        URL.revokeObjectURL(url);
        owned.current.delete(id);
        changed = true;
      }
    }
    if (changed) setThumbs(new Map(owned.current));
  }, [images]);
  // StrictMode démonte puis remonte : tout est libéré, la vue repart sans miniature.
  useEffect(() => () => {
    for (const url of owned.current.values()) URL.revokeObjectURL(url);
    owned.current.clear();
  }, []);
  const onPaste = useCallback((files: File[]) => {
    for (const f of files) {
      queue.current = queue.current.then(async () => {
        const buf = await f.arrayBuffer();
        const id = await assetId(buf);
        try {
          await send.current(toBase64(buf));
        } catch (e) {
          say(String(e));
          return;
        }
        if (id && !owned.current.has(id)) {
          owned.current.set(id, URL.createObjectURL(f));
          setThumbs(new Map(owned.current));
        }
      });
    }
  }, [say]);
  return { thumbs, onPaste };
}
```

- [ ] **Step 6: Wire the component**

`DictationDraft.tsx` props (documented):

```ts
  /** Les images collées du brouillon, tenues par l'hôte. */
  images: PastedImage[];
  /** Miniatures des images collées dans cette vue. */
  thumbs: ReadonlyMap<string, string>;
  onPasteImages: (files: File[]) => void;
  onRemoveImage: (id: string) => void;
  onSend: (text: string, images: string[]) => void;
```

On the `<section>`:

```tsx
      onPaste={(e) => {
        const files = clipboardImages(e.clipboardData);
        if (files.length === 0) return;
        e.preventDefault();
        onPasteImages(files);
      }}
```

`send` calls `onSend(said, images.map((i) => i.id))`. The send button's `title` is `said === "" && images.length > 0 ? t("ui-draft-send-needs-text") : t("ui-draft-send-title")`. Before `.draft-actions`:

```tsx
      {images.length > 0 && (
        <ul className="draft-images" aria-label={t("ui-draft-images")}>
          {images.map((i) => (
            <li key={i.id} className="draft-image">
              {thumbs.get(i.id) && <img src={thumbs.get(i.id)} alt="" draggable={false} />}
              <span>{imageLabel(i.mime, i.width, i.height)}</span>
              <IconButton className="token ghost" label={t("ui-draft-remove-image")} icon={Icon.close} onClick={() => onRemoveImage(i.id)} />
            </li>
          ))}
        </ul>
      )}
```

`styles.css` after `.draft-actions .token.ghost:hover…`:

```css
.draft-images { display: flex; flex-wrap: wrap; gap: 6px; margin: 6px 0 0; padding: 0; list-style: none; }
.draft-image { display: flex; align-items: center; gap: 6px; padding: 2px 2px 2px 6px; border-radius: var(--radius-sm); background: color-mix(in oklab, var(--paper-200) 90%, var(--ink)); font: 500 12px/18px var(--sans); }
.draft-image img { width: 28px; height: 28px; object-fit: cover; border-radius: 4px; }
.draft-image .token { width: 24px; height: 24px; }
```

`App.tsx`: after `const dictation = useDictation(…)` (line 293):

```tsx
  const pasted = usePastedImages(dictation.draft.state.images, agent.pasteImage, (m) => say(m, true));
```

and in its `<DictationDraft` (line 473), replace `onSend` and add:

```tsx
          images={dictation.draft.state.images}
          thumbs={pasted.thumbs}
          onPasteImages={pasted.onPaste}
          onRemoveImage={(id) => void agent.removeImage(id).catch((e) => say(errorText(e), true))}
          onSend={(text, images) => void dictation.actions.sendDraft(text, images)}
```

`App.tsx`'s `promptAgent` (passed to `useDictation` as `prompt`) gains the `images: string[]` parameter and forwards it to `agentHandle.current?.prompt(text, images)`; check its body with `grep -n "promptAgent" ui/src/App.tsx`.

`AgentWindow.tsx`: after `const draft = useDraft(host.agent.setDraft);` (line 154):

```tsx
  const pasted = usePastedImages(draft.state.images, host.agent.pasteImage, (m) => say(m, true));
```

and in its `<DictationDraft` (line 276):

```tsx
        images={draft.state.images}
        thumbs={pasted.thumbs}
        onPasteImages={pasted.onPaste}
        onRemoveImage={(id) => void host.agent.removeImage(id).catch((e) => say(String(e), true))}
        onSend={(text, images) => void actions.sendDraft(text, images)}
```

`host.agent` is typed by the agent-window host interface: add `pasteImage` and `removeImage` to it where `setDraft` is declared (`grep -n "setDraft" ui/src/AgentWindow.tsx ui/src/agent/*.ts`).

`ui.ftl` ×20:

```ftl
# en
ui-draft-remove-image = Remove this image
ui-draft-send-needs-text = Say what to do with it
# fr
ui-draft-remove-image = Retirer cette image
ui-draft-send-needs-text = Dites ce qu'il faut en faire
```

- [ ] **Step 7: Run the tests**

Run: `cd ui && bun test && bunx tsc -b && cd .. && cargo nextest run -p oku-slides-i18n`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add ui crates/oku-slides-i18n
git commit -F <scratchpad>/msg-t7.txt   # feat(ui) : coller une image dans le brouillon de l'agent, puces et miniatures locales (t-1168)
```

---

### Task 8: End to end and final gate

**Files:**
- Modify: `fixtures/agent/e2e.json` (`prompt_image`, a `by_prompt` entry)
- Create: `ui/e2e/260-coller-image.cjs`

**Interfaces:**
- Consumes: everything above. The 1×1 PNG used by the spec is
  `iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==`,
  id `sha256:c414cd0e204de974f73753c7e28d7638e7b3691bb8b1a2bab6b25bb7fed7ce77` (checked: valid CRCs, 1×1).

- [ ] **Step 1: Script the fake agent**

`fixtures/agent/e2e.json`: add `"prompt_image": true,` after `"http": true,`, and in `by_prompt`:

```json
    "pose l'image collée": {
      "mcp": [
        { "tool": "apply", "args": { "label": "Image collée", "ops": [
          { "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "asset",
            "value": "sha256:c414cd0e204de974f73753c7e28d7638e7b3691bb8b1a2bab6b25bb7fed7ce77" }
        ] } }
      ],
      "chunks": ["Image posée."]
    },
```

- [ ] **Step 2: Write the spec**

`ui/e2e/260-coller-image.cjs`:

```js
// Coller une image dans le brouillon de l'agent : la puce apparaît, la consigne part avec la
// liste des images et un bloc image, et le faux agent la pose sur `s-libre` (élément `e-image`).
// Contre `oku-slides serve` lancé par `just e2e` (journal du faux agent : E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post, activeTab } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const PNG = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
const ID = 'sha256:c414cd0e204de974f73753c7e28d7638e7b3691bb8b1a2bab6b25bb7fed7ce77';

/** Colle le PNG dans le brouillon ouvert, comme le presse-papier d'une capture d'écran. */
const pastePng = (p) =>
  p.locator('.draft').evaluate((el, b64) => {
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const dt = new DataTransfer();
    dt.items.add(new File([bytes], 'capture.png', { type: 'image/png' }));
    el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true }));
  }, PNG);

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  const from = logOffset(LOG);

  await p.keyboard.press('ControlOrMeta+k');
  await p.locator('.draft-text').fill("pose l'image collée");
  await pastePng(p);
  await p.locator('.draft-image').waitFor();
  assert.match(await p.locator('.draft-image').innerText(), /PNG 1×1/);
  assert.equal(await p.locator('.draft-image img').count(), 1, 'miniature locale');

  await p.locator('.draft-send').click();
  await p.waitForSelector('.draft-text', { state: 'detached' });
  // Le journal du faux agent : une ligne `{ method, params }` par message reçu.
  const prompts = () => logSince(LOG, from).filter((m) => m.method === 'session/prompt');
  await until(p, async () => prompts().length > 0, 'la consigne doit atteindre le faux agent', 15000);
  const blocks = prompts()[0].params.prompt;
  assert.ok(blocks.some((x) => x.type === 'text' && x.text.includes(ID)), 'bloc « Pasted images »');
  assert.ok(blocks.some((x) => x.type === 'image' && x.mimeType === 'image/png' && x.data === PNG), 'bloc image');
  assert.equal(blocks.at(-1).text, "pose l'image collée");

  // Le faux agent pose l'image : elle est dans le deck, et la bulle du fil garde sa puce.
  const tab = await activeTab(p);
  const slide = () => post(p, '/api', { tab, req: { cmd: 'slide', id: 's-libre' } });
  await until(p, async () => JSON.stringify(await slide()).includes(ID), "l'image doit être posée sur s-libre", 15000);
  await p.locator('.a2ui-prompt-image').first().waitFor();

  assert.deepEqual(errors, []);
  console.log('coller-image : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

(`until(p, fn, why, timeout)` polls from Node, as `ui/AGENTS.md` requires instead of an async `page.waitForFunction` predicate. Close the browser the way `230-raccourci-theme.cjs` does if it differs.)

- [ ] **Step 3: Run the e2e suite**

Run: `just e2e 4999`
Expected: PASS for every spec, `260-coller-image` included (earlier specs see `prompt_image: true` but send no image).

- [ ] **Step 4: Run the full gate**

Run: `just ci`
Expected: PASS (fmt, clippy `-D warnings`, tsc, fonts-lfs, agents-lint, nextest, UI tests, audit).

- [ ] **Step 5: Commit**

```bash
git add fixtures/agent/e2e.json ui/e2e/260-coller-image.cjs
git commit -F <scratchpad>/msg-t8.txt   # test(e2e) : coller une image, consigne avec bloc image, image posée par l'agent (t-1169)
```
