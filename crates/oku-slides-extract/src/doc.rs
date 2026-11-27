//! What the text documents (DOCX, PPTX) are written with: runs of a paragraph rendered as
//! escaped Markdown, blocks charged against one text budget, images cited by placeholder
//! within their limits, and the notices of what was left out.

use std::borrow::Cow;

use oku_slides_core::knowledge::{Cut, ExtractLimits, ExtractRefusal, Extracted};
use oku_slides_core::num::count_u64;

use crate::finish;
use crate::images::{Images, Mark, Skip};
use crate::markdown::{Bound, cut_line, escape_inline, line_start, link_url, skipped_line, table};

/// Most cells of one table read; the cells past it are left out, their text stated as cut
/// with the text left out, not as a cut of their own (accepted, t-1760). A row past it whose
/// cells are all left out is dropped; an empty row before it is kept as the source has it.
pub(crate) const MAX_TABLE_CELLS: usize = 100_000;

/// One piece of a paragraph.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    /// Text on one line, with its emphasis and its link (an index in `Inline::links`).
    Text { text: String, bold: bool, italic: bool, link: Option<usize> },
    /// A line break.
    Break,
    /// An image, by its key in the document (part path, asset id) and its alternative text.
    Image { key: String, alt: String },
}

/// The runs of one paragraph, kept within a byte cap; the text past it is only counted.
#[derive(Debug, Default)]
pub(crate) struct Inline {
    pieces: Vec<Piece>,
    /// Each link's URL, `None` when its scheme is not kept (the link becomes its text).
    links: Vec<Option<String>>,
    /// Bytes of text kept.
    raw: usize,
    /// Most bytes of text kept.
    cap: usize,
    /// Bytes of text past `cap`.
    dropped: u64,
}

impl Inline {
    /// An empty paragraph keeping at most `cap` bytes of text.
    pub(crate) fn new(cap: usize) -> Self {
        Self { cap, ..Self::default() }
    }

    /// Adds `text` (line ends and tabs become spaces) with its emphasis, in `link` if any. Once
    /// a run does not fit the cap, every later one is only counted: no hole mid-paragraph.
    pub(crate) fn text(&mut self, text: &str, bold: bool, italic: bool, link: Option<usize>) {
        if text.is_empty() {
            return;
        }
        if self.dropped > 0 || self.raw + text.len() > self.cap {
            self.dropped = self.dropped.saturating_add(count_u64(text.len()));
            return;
        }
        self.raw += text.len();
        let text = text.replace(['\n', '\r', '\t'], " ");
        match self.pieces.last_mut() {
            Some(Piece::Text { text: last, bold: b, italic: i, link: l }) if (*b, *i, *l) == (bold, italic, link) => {
                last.push_str(&text);
            }
            _ => self.pieces.push(Piece::Text { text, bold, italic, link }),
        }
    }

    /// Adds a line break; a break after a break or at the start is dropped.
    pub(crate) fn line_break(&mut self) {
        if !matches!(self.pieces.last(), None | Some(Piece::Break)) {
            self.pieces.push(Piece::Break);
        }
    }

    /// Adds the image `key`.
    pub(crate) fn image(&mut self, key: &str, alt: &str) {
        self.pieces.push(Piece::Image { key: key.to_owned(), alt: alt.to_owned() });
    }

    /// Opens a link to `url`, kept only for `http`, `https` and `mailto`; returns its index for
    /// [`Inline::text`].
    pub(crate) fn link(&mut self, url: &str) -> usize {
        self.links.push(link_url(url));
        self.links.len() - 1
    }

    /// True when the paragraph has neither visible text nor image, and none was left out.
    pub(crate) fn is_blank(&self) -> bool {
        self.dropped == 0
            && self.pieces.iter().all(|p| match p {
                Piece::Text { text, .. } => text.trim().is_empty(),
                Piece::Break => true,
                Piece::Image { .. } => false,
            })
    }

    /// Bytes of text left out past the cap.
    pub(crate) fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Bytes of text read, kept or not.
    pub(crate) fn read_bytes(&self) -> u64 {
        count_u64(self.raw).saturating_add(self.dropped)
    }

    /// Its images, key and alternative text, in order.
    pub(crate) fn images(&self) -> impl Iterator<Item = (&str, &str)> {
        self.pieces.iter().filter_map(|p| match p {
            Piece::Image { key, alt } => Some((key.as_str(), alt.as_str())),
            _ => None,
        })
    }

    /// Its text without markup or images, breaks as spaces: what a table cell holds.
    pub(crate) fn plain(&self) -> String {
        let mut out = String::new();
        for p in &self.pieces {
            match p {
                Piece::Text { text, .. } => out.push_str(text),
                Piece::Break => out.push(' '),
                Piece::Image { .. } => {}
            }
        }
        out.trim().to_owned()
    }

    /// The paragraph as Markdown: text escaped, emphasis and kept links as markup, each image
    /// as what `cite` gives (nothing when it is not imported). `one_line` turns breaks into
    /// spaces; otherwise each line, once assembled, opens as a Markdown line, its block marks
    /// escaped and its blank lines dropped.
    pub(crate) fn render(&self, one_line: bool, cite: &mut dyn FnMut(&str, &str) -> Option<String>) -> String {
        let mut out = String::new();
        let mut i = 0;
        while let Some(piece) = self.pieces.get(i) {
            match piece {
                Piece::Break => {
                    out.push(if one_line { ' ' } else { '\n' });
                    i += 1;
                }
                Piece::Image { key, alt } => {
                    if let Some(md) = cite(key, alt) {
                        out.push_str(&md);
                    }
                    i += 1;
                }
                Piece::Text { link, .. } => {
                    let end = self.pieces[i..]
                        .iter()
                        .position(|p| !matches!(p, Piece::Text { link: l, .. } if l == link))
                        .map_or(self.pieces.len(), |n| i + n);
                    let inner = styled(&self.pieces[i..end]);
                    match link.and_then(|l| self.links.get(l)).and_then(Option::as_ref) {
                        Some(url) if !inner.trim().is_empty() => {
                            out.push('[');
                            out.push_str(&inner);
                            out.push_str("](");
                            out.push_str(url);
                            out.push(')');
                        }
                        _ => out.push_str(&inner),
                    }
                    i = end;
                }
            }
        }
        if one_line {
            return out.trim().to_owned();
        }
        let lines: Vec<String> = out.split('\n').map(line_start).filter(|l| !l.trim().is_empty()).collect();
        lines.join("\n").trim_end().to_owned()
    }
}

/// Text pieces as escaped inline Markdown, each emphasis wrapped around its text without its
/// outer spaces.
fn styled(pieces: &[Piece]) -> String {
    let mut out = String::new();
    for p in pieces {
        let Piece::Text { text, bold, italic, .. } = p else { continue };
        let core = text.trim_matches(' ');
        if core.is_empty() {
            out.push_str(text);
            continue;
        }
        let lead = &text[..text.len() - text.trim_start_matches(' ').len()];
        let trail = &text[text.trim_end_matches(' ').len()..];
        let mark = match (bold, italic) {
            (true, true) => "***",
            (true, false) => "**",
            (false, true) => "*",
            (false, false) => "",
        };
        out.push_str(lead);
        out.push_str(mark);
        out.push_str(&escape_inline(core));
        out.push_str(mark);
        out.push_str(trail);
    }
    out
}

/// Heading text: rendered on one line, every `#` escaped so that none closes the heading.
pub(crate) fn heading(inline: &Inline, cite: &mut dyn FnMut(&str, &str) -> Option<String>) -> String {
    inline.render(true, cite).trim_start().replace('#', "\\#")
}

/// How a block joins the previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Join {
    /// A blank line: a paragraph, a heading, a table.
    Block,
    /// A list item at `level`: right after an item of the same kind (ordered or not), else
    /// a blank line.
    Item { ordered: bool, level: usize },
}

/// The Markdown body, charged against one text budget: past it, blocks are only counted.
#[derive(Debug)]
pub(crate) struct Writer {
    out: String,
    budget: usize,
    /// The list kind and level of the last block, if it was an item.
    last_item: Option<(bool, usize)>,
    /// Bytes of text left out for the budget.
    dropped: u64,
    exhausted: bool,
}

impl Writer {
    /// An empty body of at most `budget` bytes.
    pub(crate) fn new(budget: usize) -> Self {
        Self { out: String::new(), budget, last_item: None, dropped: 0, exhausted: false }
    }

    /// Bytes the body may still take.
    pub(crate) fn room(&self) -> usize {
        if self.exhausted { 0 } else { self.budget.saturating_sub(self.out.len()) }
    }

    /// The level an item at `level` is written at: at most one below the last block's item,
    /// 0 after any other block, so that no item becomes indented code.
    pub(crate) fn item_level(&self, level: usize) -> usize {
        self.last_item.map_or(0, |(_, last)| level.min(last + 1))
    }

    /// Adds `text` as a block joined by `join`; false (the block left out, and every later
    /// one) when it does not fit the budget.
    pub(crate) fn block(&mut self, text: &str, join: Join) -> bool {
        if text.is_empty() {
            return true;
        }
        let sep = match (join, self.last_item) {
            _ if self.out.is_empty() => "",
            (Join::Item { ordered, .. }, Some((last, _))) if ordered == last => "\n",
            _ => "\n\n",
        };
        if self.exhausted || self.out.len() + sep.len() + text.len() > self.budget {
            self.exhausted = true;
            return false;
        }
        self.out.push_str(sep);
        self.out.push_str(text);
        self.last_item = match join {
            Join::Item { ordered, level } => Some((ordered, level)),
            Join::Block => None,
        };
        true
    }

    /// Leaves out every later block: one did not fit.
    fn close(&mut self) {
        self.exhausted = true;
    }

    /// Counts `bytes` of text left out.
    pub(crate) fn drop_bytes(&mut self, bytes: u64) {
        self.dropped = self.dropped.saturating_add(bytes);
    }

    /// The body (ending with a line end when not empty) and the bytes left out.
    pub(crate) fn finish(mut self) -> (String, u64) {
        if !self.out.is_empty() {
            self.out.push('\n');
        }
        (self.out, self.dropped)
    }
}

/// What a paragraph is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Plain text.
    Para,
    /// A heading of this level, 1 to 6.
    Heading(usize),
    /// A list item at this nesting level, numbered or not.
    Item { ordered: bool, level: usize },
}

/// Reads the bytes of an image by its key.
pub(crate) type Fetch<'f, 'b> = &'f dyn Fn(&str) -> Result<Cow<'b, [u8]>, Skip>;

/// A document being written: its body within the text budget and its images within theirs.
#[derive(Debug)]
pub(crate) struct Doc<'l> {
    writer: Writer,
    images: Images<'l>,
}

impl<'l> Doc<'l> {
    /// An empty document; `pixels` is the pixel limit of one image.
    pub(crate) fn new(limits: &'l ExtractLimits, pixels: u64, token: &'l str) -> Self {
        Self { writer: Writer::new(limits.max_text_bytes), images: Images::new(limits, pixels, token) }
    }

    /// Bytes the body may still take: the cap of the next paragraph's text.
    pub(crate) fn room(&self) -> usize {
        self.writer.room()
    }

    /// Adds a paragraph of kind `kind`; past the budget it is only counted, its images listed.
    pub(crate) fn paragraph(&mut self, inline: &Inline, kind: Kind, fetch: Fetch<'_, '_>) {
        if inline.is_blank() {
            return;
        }
        if self.writer.room() == 0 {
            self.left_out(&[inline]);
            return;
        }
        let mark = self.images.mark();
        let images = &mut self.images;
        let mut cite = |key: &str, alt: &str| images.cite(key, alt, || fetch(key));
        let (text, join) = match kind {
            Kind::Para => (inline.render(false, &mut cite), Join::Block),
            Kind::Heading(level) => (prefixed(&"#".repeat(level), &heading(inline, &mut cite)), Join::Block),
            Kind::Item { ordered, level } => {
                let level = self.writer.item_level(level);
                let marker = format!("{}{}", "    ".repeat(level), if ordered { "1." } else { "-" });
                let text = inline.render(false, &mut cite).replace('\n', " ");
                (prefixed(&marker, &text), Join::Item { ordered, level })
            }
        };
        self.commit(&text, join, &[inline], mark);
    }

    /// Adds a table, its first row the header, each cell's text plain; the images of its
    /// cells follow it.
    pub(crate) fn table(&mut self, rows: &[Vec<Inline>], fetch: Fetch<'_, '_>) {
        let cells: Vec<&Inline> = rows.iter().flatten().collect();
        if cells.iter().all(|c| c.is_blank()) {
            return;
        }
        // Each row is padded to the widest: at least `|`, then ` ` and ` |` per cell and a line
        // end, the separator row included. A table that cannot fit is never built.
        let width = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
        let least = width.checked_mul(3).and_then(|w| w.checked_add(2)).and_then(|r| r.checked_mul(rows.len() + 1));
        if least.is_none_or(|n| n > self.writer.room()) {
            self.writer.close();
            self.left_out(&cells);
            return;
        }
        let mark = self.images.mark();
        let plain: Vec<Vec<String>> = rows.iter().map(|r| r.iter().map(Inline::plain).collect()).collect();
        let mut text = table(&plain).trim_end().to_owned();
        let mut cited = Vec::new();
        for (key, alt) in cells.iter().flat_map(|c| c.images()) {
            cited.extend(self.images.cite(key, alt, || fetch(key)));
        }
        if !cited.is_empty() {
            text.push_str("\n\n");
            text.push_str(&cited.join(" "));
        }
        self.commit(&text, Join::Block, &cells, mark);
    }

    /// Adds a block already written as Markdown from `raw` bytes of text, `dropped` more
    /// already left out of it; past the budget it is only counted.
    pub(crate) fn block(&mut self, text: &str, raw: u64, dropped: u64) {
        if self.writer.block(text, Join::Block) {
            self.writer.drop_bytes(dropped);
        } else {
            self.writer.drop_bytes(raw.saturating_add(dropped));
        }
    }

    /// Counts image `key` as not imported for `why`.
    pub(crate) fn skip_image(&mut self, key: &str, why: Skip) {
        self.images.skip(key, why);
    }

    fn commit(&mut self, text: &str, join: Join, inlines: &[&Inline], mark: Mark) {
        if self.writer.block(text, join) {
            for i in inlines {
                self.writer.drop_bytes(i.dropped());
            }
            return;
        }
        self.images.rollback(mark);
        self.left_out(inlines);
    }

    /// Counts `inlines` as left out for the budget: their text dropped, their images listed
    /// without being read.
    pub(crate) fn left_out(&mut self, inlines: &[&Inline]) {
        for i in inlines {
            for (key, _) in i.images() {
                self.images.skip(key, Skip::Budget);
            }
            self.writer.drop_bytes(i.read_bytes());
        }
    }

    /// The extracted document: the body, then the notices of the reader's own `lead` cut
    /// (nested elements, pages) bounded by `bound`, of the images not imported and of the
    /// `skipped` content, all fitted to `max_text_bytes`. A kept image whose placeholder the
    /// fit cuts is counted as left out for the budget, and the notices fitted again.
    pub(crate) fn finish(
        self,
        lead: Cut,
        bound: Bound<'_>,
        skipped: &[&str],
        limits: &ExtractLimits,
    ) -> Result<Extracted, ExtractRefusal> {
        let (body, dropped) = self.writer.finish();
        let mut images = self.images;
        loop {
            let (notices, image_cut) = images.notices();
            let mut cuts = Vec::new();
            let mut close = String::new();
            if lead.count > 0 {
                close.push_str(&cut_line(lead, bound));
                cuts.push(lead);
            }
            close.push_str(&notices);
            cuts.extend(image_cut);
            if !skipped.is_empty() {
                close.push_str(&skipped_line(skipped));
            }
            if !body.is_empty() && !close.is_empty() {
                close.insert(0, '\n');
            }
            let mut out = finish("", &body, &close, limits, cuts, dropped)?;
            let Some(from) = images.first_uncited(&out.markdown) else {
                out.images = images.into_kept();
                return Ok(out);
            };
            images.demote(from);
        }
    }
}

/// `text` behind `marker` and a space; nothing when `text` is empty.
fn prefixed(marker: &str, text: &str) -> String {
    if text.is_empty() { String::new() } else { format!("{marker} {text}") }
}

#[cfg(test)]
#[path = "doc_tests.rs"]
mod doc_tests;
