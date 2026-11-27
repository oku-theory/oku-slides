//! PPTX: each slide read by `read_pptx` as `## Slide N — <title>`, then its text, lists,
//! tables and pictures in drawing order and its notes under `### Notes`. Hidden slides,
//! charts, embedded objects and what `read_pptx` warns it could not read are left out and
//! stated.

use std::borrow::Cow;
use std::collections::BTreeSet;

use oku_slides_core::knowledge::{Cut, CutKind, ExtractLimits, ExtractRefusal};
use oku_slides_core::media::{dimensions, sniff};
use oku_slides_core::num::count_u64;
use oku_slides_import::bounded::BoundedZip;
use oku_slides_import::source::{
    Bullet, Item, ItemKind, Opaque, OpaqueReason, Role, SourceDeck, SourceSlide, TextBox, WarningKind,
};
use oku_slides_import::{ReadLimits, read_pptx};
use roxmltree::Node;

use crate::archive::{self, children};
use crate::doc::{Doc, Fetch, Inline, Kind, MAX_TABLE_CELLS, heading};
use crate::images::Skip;
use crate::markdown::{Bound, escape_text};
use oku_slides_core::knowledge::Extracted;
use oku_slides_core::knowledge::RASTERS;

/// The `PresentationML` namespace.
const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";

/// The namespace of relationship ids.
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// Deepest list level written.
const MAX_LEVEL: usize = 8;

/// Extracts a PPTX presentation.
pub(crate) fn extract(
    bytes: &[u8],
    limits: &ExtractLimits,
    read: &ReadLimits,
    token: &str,
) -> Result<Extracted, ExtractRefusal> {
    let zip = archive::open(bytes, read)?;
    let main = archive::main_part(&zip)?;
    let text = archive::text(&zip, &main)?;
    let xml = archive::xml(&zip, &text, &main)?;
    let root = xml.root_element().tag_name();
    if root.name() != "presentation" || root.namespace() != Some(P) {
        return Err(ExtractRefusal::TypeMismatch);
    }
    let pixels = limits.max_image_pixels.min(read.max_image_pixels);
    let deck = read_pptx(bytes, &ReadLimits { max_image_pixels: pixels, ..*read }).map_err(|e| archive::refusal(&e))?;
    let mut doc = Doc::new(limits, pixels, token);
    let mut skipped = Skipped::default();
    let mut depth = 0u64;
    // Picture parts read again, each once: a subset of what `read_pptx` read within the same
    // limits.
    let mut reread = BTreeSet::new();
    for w in &deck.warnings {
        match &w.kind {
            // Rendering only, or text kept (an unsafe link becomes its text): nothing is lost
            // from the extracted text. `DataAdjusted` comes only from the import plan lifting
            // inline values into deck data; extraction reads the source cells, never lifted.
            WarningKind::EffectIgnored(_)
            | WarningKind::Approximated(_)
            | WarningKind::UnknownTableStyle(_)
            | WarningKind::FontEmbeddedUnreadable(_)
            | WarningKind::UnsafeLink(_)
            | WarningKind::DataAdjusted { .. } => {}
            WarningKind::UnreadableItem(_) => skipped.met(Lost::Items),
            WarningKind::UnreadablePart(_) | WarningKind::MissingPart(_) => skipped.met(Lost::Parts),
            WarningKind::UnsupportedChart(_) => skipped.met(Lost::Charts),
            // A sanitized SVG is kept and refused by type when cited; a rejected one is not.
            WarningKind::UnsafeSvg(_) => skipped.met(Lost::Svg),
            WarningKind::NestingTooDeep(_) => depth = depth.saturating_add(1),
            WarningKind::OversizedImage(path) => doc.skip_image(path, Skip::Pixels),
            WarningKind::ExternalImage(target) => doc.skip_image(target, Skip::Unreadable),
            // A raster whose header does not read has no asset; any other type keeps one and
            // is refused by type when cited. `read_pptx` read these bytes, so they read again
            // unless the warning clipped the path: no path then links it to an asset, and the
            // picture is stated without a count rather than maybe counted twice.
            WarningKind::UnsupportedImage(path) => {
                reread.insert(path.clone());
                match zip.read(path) {
                    Ok(b) if RASTERS.contains(&sniff(&b)) => doc.skip_image(path, Skip::Unreadable),
                    Ok(_) => {}
                    Err(_) => skipped.met(Lost::Pictures),
                }
            }
            WarningKind::Truncated => {
                skipped.met(Lost::Warnings);
                let (refused, cut) = unwarned_images(&zip, &main, xml.root_element(), &deck, pixels, &mut reread);
                for (image, why) in refused {
                    doc.skip_image(&image, why);
                }
                if cut {
                    skipped.met(Lost::Pictures);
                }
            }
        }
    }
    let mut writer = Slides { doc, deck: &deck, skipped };
    for slide in &deck.slides {
        writer.slide(slide);
    }
    let Slides { doc, skipped, .. } = writer;
    doc.finish(Cut { what: CutKind::Depth, count: depth }, Bound::TooDeep, &skipped.list(), limits)
}

/// The pictures of the shown slides that `read_pptx` refused once its warnings were
/// cut, and whether the scan stopped short. Each cited part is read again once (`reread`);
/// a citation `read_pptx` never parsed (an unchosen branch, a group cut for depth) can
/// still spend this zip's budget, so the first read error of any kind, a slide's own parts
/// included, stops the scan: past it, a failing read could not tell a refused picture from
/// one the budget cut. A raster whose header does not read, or one past `pixels`, is
/// listed; a picture it imported, or keeps and refuses when cited, is not.
fn unwarned_images(
    zip: &BoundedZip<'_>,
    main: &str,
    root: Node<'_, '_>,
    deck: &SourceDeck,
    pixels: u64,
    reread: &mut BTreeSet<String>,
) -> (Vec<(String, Skip)>, bool) {
    let mut refused = Vec::new();
    let Ok((rels, _)) = archive::rels_and_links(zip, main) else { return (refused, true) };
    let ids = children(root, "sldIdLst").flat_map(|l| children(l, "sldId"));
    for (number, id) in (1u32..).zip(ids) {
        let shown = deck.slides.iter().any(|s| s.number == number && !s.hidden);
        let part = id.attribute((R, "id")).and_then(|id| rels.iter().find(|r| r.id == id));
        let (true, Some(part)) = (shown, part) else { continue };
        let (Ok(images), Ok(text)) = (archive::rels(zip, &part.target), archive::text(zip, &part.target)) else {
            return (refused, true);
        };
        let Ok(xml) = archive::xml(zip, &text, &part.target) else { return (refused, true) };
        let cited =
            xml.descendants().filter(|n| n.tag_name().name() == "blip").filter_map(|b| b.attribute((R, "embed")));
        for rid in cited {
            let Some(image) = images.iter().find(|r| r.id == rid && r.kind.ends_with("/image")) else { continue };
            if !reread.insert(image.target.clone()) {
                continue;
            }
            let Ok(bytes) = zip.read(&image.target) else { return (refused, true) };
            let mime = sniff(&bytes);
            let why = match dimensions(&bytes, mime) {
                _ if !RASTERS.contains(&mime) => None,
                None => Some(Skip::Unreadable),
                Some((w, h)) => (u64::from(w) * u64::from(h) > pixels).then_some(Skip::Pixels),
            };
            if let Some(why) = why {
                refused.push((image.target.clone(), why));
            }
        }
    }
    (refused, false)
}

/// Content met and left out, in the order the notice lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Lost {
    HiddenSlides,
    Charts,
    Objects,
    Items,
    Parts,
    Svg,
    Pictures,
    Warnings,
}

impl Lost {
    fn label(self) -> &'static str {
        match self {
            Self::HiddenSlides => "hidden slides",
            Self::Charts => "charts",
            Self::Objects => "media and embedded objects",
            Self::Items => "unreadable items",
            Self::Parts => "unreadable or missing parts",
            Self::Svg => "unsafe SVG images",
            Self::Pictures => "pictures whose part could not be read again, not counted",
            Self::Warnings => "what was met past the import warning limit",
        }
    }
}

/// What was met and left out.
#[derive(Debug, Default)]
struct Skipped(BTreeSet<Lost>);

impl Skipped {
    fn met(&mut self, lost: Lost) {
        self.0.insert(lost);
    }

    fn list(&self) -> Vec<&'static str> {
        self.0.iter().map(|l| l.label()).collect()
    }
}

/// The writing of the slides.
struct Slides<'d, 'l> {
    doc: Doc<'l>,
    deck: &'d SourceDeck,
    skipped: Skipped,
}

impl Slides<'_, '_> {
    fn fetch<'b>(deck: &'b SourceDeck, key: &str) -> Result<Cow<'b, [u8]>, Skip> {
        deck.assets.get(key).map(|a| Cow::Borrowed(a.bytes.as_slice())).ok_or(Skip::Unreadable)
    }

    fn slide(&mut self, slide: &SourceSlide) {
        if slide.hidden {
            self.skipped.met(Lost::HiddenSlides);
            return;
        }
        let deck = self.deck;
        let fetch = |key: &str| Self::fetch(deck, key);
        let title = slide.items.iter().find(|i| i.role == Some(Role::Title) && i.has_text());
        let mut name = Inline::new(self.doc.room());
        if let Some(tb) = title.and_then(Item::text) {
            for p in &tb.paragraphs {
                for run in &p.runs {
                    name.text(&run.text, false, false, None);
                }
                name.text(" ", false, false, None);
            }
        }
        let title_dropped = name.dropped();
        let name = heading(&name, &mut |_, _| None);
        let line = if name.is_empty() {
            format!("## Slide {}", slide.number)
        } else {
            format!("## Slide {} — {name}", slide.number)
        };
        self.doc.block(&line, count_u64(line.len()), title_dropped);
        let mut stack: Vec<&Item> = slide.items.iter().rev().collect();
        while let Some(item) = stack.pop() {
            if title.is_some_and(|t| std::ptr::eq(t, item)) {
                continue;
            }
            match &item.kind {
                ItemKind::Text(tb) => self.text(tb, &fetch),
                ItemKind::Shape(shape) => {
                    if let Some(tb) = &shape.text {
                        self.text(tb, &fetch);
                    }
                }
                ItemKind::Picture(picture) => {
                    let mut inline = Inline::new(0);
                    inline.image(&picture.asset, item.alt.as_deref().unwrap_or(""));
                    self.doc.paragraph(&inline, Kind::Para, &fetch);
                }
                ItemKind::Table(table) => {
                    let mut rows: Vec<Vec<Inline>> = Vec::new();
                    let mut cells = 0usize;
                    for row in &table.rows {
                        let mut out = Vec::new();
                        for cell in row {
                            let inline = self.cell(&cell.text, cell.covered);
                            if cells == MAX_TABLE_CELLS {
                                self.doc.left_out(&[&inline]);
                            } else {
                                cells += 1;
                                out.push(inline);
                            }
                        }
                        if !out.is_empty() || cells < MAX_TABLE_CELLS {
                            rows.push(out);
                        }
                    }
                    self.doc.table(&rows, &fetch);
                }
                ItemKind::Group { items, .. } => stack.extend(items.iter().rev()),
                ItemKind::SmartArt(art) => {
                    for node in &art.nodes {
                        let level = usize::from(node.depth).min(MAX_LEVEL);
                        for p in &node.text.paragraphs {
                            let inline = self.paragraph_inline(p);
                            self.doc.paragraph(&inline, Kind::Item { ordered: false, level }, &fetch);
                        }
                    }
                }
                ItemKind::Chart(_) => self.skipped.met(Lost::Charts),
                // A group past the nesting limit: its `NestingTooDeep` warning states it.
                ItemKind::Opaque(Opaque { reason: OpaqueReason::Unknown(why), .. }) if why == "nesting" => {}
                ItemKind::Opaque(_) => self.skipped.met(Lost::Objects),
            }
        }
        let notes = slide.notes.trim();
        if !notes.is_empty() {
            let block = format!("### Notes\n\n{}", escape_text(notes));
            self.doc.block(&block, count_u64(notes.len()), 0);
        }
    }

    /// The paragraphs of a text box, each a paragraph or a list item by its bullet.
    fn text(&mut self, tb: &TextBox, fetch: Fetch<'_, '_>) {
        for p in &tb.paragraphs {
            let inline = self.paragraph_inline(p);
            let level = usize::from(p.level).min(MAX_LEVEL);
            let kind = match p.style.bullet {
                Bullet::None => Kind::Para,
                Bullet::Char { .. } => Kind::Item { ordered: false, level },
                Bullet::AutoNum { .. } => Kind::Item { ordered: true, level },
            };
            self.doc.paragraph(&inline, kind, fetch);
        }
    }

    /// The runs of a paragraph: line breaks kept, emphasis and links.
    fn paragraph_inline(&self, p: &oku_slides_import::source::Paragraph) -> Inline {
        let mut inline = Inline::new(self.doc.room());
        for run in &p.runs {
            let link = run.style.link.as_deref().map(|url| inline.link(url));
            for (i, line) in run.text.split(['\n', '\u{b}']).enumerate() {
                if i > 0 {
                    inline.line_break();
                }
                inline.text(line, run.style.strong(), run.style.italic, link);
            }
        }
        inline
    }

    /// A table cell's text; a cell covered by a merge is empty.
    fn cell(&self, tb: &TextBox, covered: bool) -> Inline {
        let mut inline = Inline::new(if covered { 0 } else { self.doc.room() });
        if !covered {
            for p in &tb.paragraphs {
                for run in &p.runs {
                    inline.text(&run.text, false, false, None);
                }
                inline.text(" ", false, false, None);
            }
        }
        inline
    }
}

#[cfg(test)]
#[path = "pptx_tests.rs"]
mod pptx_tests;
