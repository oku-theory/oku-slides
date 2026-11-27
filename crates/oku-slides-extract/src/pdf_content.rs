//! One PDF page's content, walked for the text it shows and the images it draws.
//!
//! lopdf's own text extraction never follows `Do`, so text drawn by a form `XObject` would be
//! lost. This walk follows forms on an explicit stack, at most [`MAX_FORM_DEPTH`] deep, each
//! form read once per page, every stream decoded within the page's one byte budget. The text
//! collected stops at the same limit: a `ToUnicode` map can turn one byte into a long string.
//!
//! A [`Cache`] kept across pages decodes each font's encoding once, its `ToUnicode` map charged
//! to the page's budget and so to the document's total: the cache holds at most that total of
//! maps (t-1760). It also keeps what a form with resources of its own drew: a form shared by
//! every page is decoded once. Only a form read whole is kept: one cut by the depth, a loop,
//! the limit or a failure reads again.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use lopdf::content::{Content, Operation};
use lopdf::{DecompressError, Dictionary, Document, Encoding, Error, Object, ObjectId};

/// Two `Tm` vertical offsets closer than this, in text space units, are one line.
const SAME_LINE: f32 = 0.01;

/// Deepest nesting of forms read; a form drawn deeper is stated as not read.
pub(crate) const MAX_FORM_DEPTH: usize = 8;

/// Why a page's own content, or part of its text, did not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Failure {
    /// Its content, or a font's, decodes past the text limit.
    Over,
    /// Its content, resources or a font do not read.
    Unreadable,
}

/// What the walk of one page found.
#[derive(Debug, Default)]
pub(crate) struct Walk {
    /// The text shown by the page and its forms, in drawing order.
    pub(crate) text: String,
    /// The image `XObject`s drawn by the page and its forms, each once, in drawing order.
    pub(crate) images: Vec<ObjectId>,
    /// Inline images (`BI` to `EI`), which are not imported.
    pub(crate) inline_images: usize,
    /// Forms drawn but not read: too deep, drawing themselves, or unreadable.
    pub(crate) forms_unread: usize,
    /// Drawn names that resolve to no readable `XObject`, an unresolved reference included.
    pub(crate) broken: usize,
    /// The failure of the page's own content, resources or fonts, if any.
    pub(crate) failure: Option<Failure>,
    /// Content bytes decoded for the page and its forms.
    pub(crate) decoded: usize,
}

/// What the walks of a document's pages share: font encodings and forms read whole.
#[derive(Default)]
pub(crate) struct Cache<'p> {
    /// Fonts by object id: their encoding, or why it does not read.
    fonts: BTreeMap<ObjectId, Result<Rc<Encoding<'p>>, Failure>>,
    /// Forms with resources of their own, read whole, by object id.
    forms: BTreeMap<ObjectId, Rc<Drawn>>,
    /// Bytes of text the kept forms hold, at most the limit of one page.
    text: usize,
}

/// What a form read whole drew: replayed when another page draws it.
#[derive(Debug)]
struct Drawn {
    text: String,
    images: Vec<ObjectId>,
    inline_images: usize,
    forms_unread: usize,
    broken: usize,
}

/// Where a form's own drawing starts in the page's [`Walk`], and what it drew so far.
#[derive(Debug)]
struct Record {
    text: usize,
    images: Vec<ObjectId>,
    inline_images: usize,
    forms_unread: usize,
    broken: usize,
    failure: Option<Failure>,
    /// Something in the form read differently than it would alone: it is not kept.
    partial: bool,
}

/// What a `Do` leads to.
enum Next<'p> {
    /// An image `XObject`, listed.
    Image(ObjectId),
    /// A form to walk.
    Form(Frame<'p>),
    /// A form already read whole on an earlier page.
    Cached(Rc<Drawn>),
    /// A form not read here although it could be elsewhere: too deep, looping or read already.
    Partial,
}

impl Walk {
    /// Records `failure`; a content over the limit outweighs an unreadable one.
    fn fail(&mut self, failure: Failure) {
        if self.failure != Some(Failure::Over) {
            self.failure = Some(failure);
        }
    }
}

/// Walks page `page`, decoding at most `limit` bytes of content for it and its forms; what
/// `cache` holds is reused, and what it reads whole kept there.
pub(crate) fn walk<'p>(pdf: &'p Document, page: ObjectId, limit: usize, cache: &mut Cache<'p>) -> Walk {
    let mut walker = Walker {
        pdf,
        cache,
        limit,
        left: limit,
        read: BTreeSet::new(),
        drawn: BTreeSet::new(),
        full: false,
        tm_y: None,
        out: Walk::default(),
    };
    let Some(resources) = page_resources(pdf, page) else {
        walker.out.fail(Failure::Unreadable);
        return walker.out;
    };
    if !contents_resolve(pdf, page) {
        // lopdf skips a content stream that does not resolve: the page would read as blank.
        walker.out.fail(Failure::Unreadable);
    }
    let ops = pdf.get_page_content_with_limit(page, limit).and_then(|data| {
        walker.left = limit.saturating_sub(data.len());
        Content::decode(&data)
    });
    match ops {
        Ok(content) => walker.run(Frame::new(content.operations, resources, None, None)),
        Err(e) => {
            if failure(&e) == Failure::Over {
                // Inflated up to the limit before failing.
                walker.left = 0;
            }
            walker.out.fail(failure(&e));
        }
    }
    walker.out.decoded = limit - walker.left;
    walker.out
}

/// The resource dictionaries of page `page`, its own then inherited, or `None` when one of
/// them does not read.
fn page_resources(pdf: &Document, page: ObjectId) -> Option<Vec<&Dictionary>> {
    let (own, inherited) = pdf.get_page_resources(page).ok()?;
    let mut dicts: Vec<&Dictionary> = own.into_iter().collect();
    for id in inherited {
        dicts.push(pdf.get_dictionary(id).ok()?);
    }
    Some(dicts)
}

/// Whether every content stream of page `page` resolves; a page without `Contents` is blank.
fn contents_resolve(pdf: &Document, page: ObjectId) -> bool {
    let Ok(page) = pdf.get_dictionary(page) else { return false };
    // `Dictionary::get` fails only on a missing key.
    let Ok(contents) = page.get(b"Contents") else { return true };
    let is_stream = |o: &Object| pdf.dereference(o).is_ok_and(|(_, o)| o.as_stream().is_ok());
    match pdf.dereference(contents) {
        Ok((_, Object::Array(items))) => items.iter().all(is_stream),
        Ok((_, Object::Stream(_))) => true,
        // Any other kind of object: no content to read.
        Ok((_, _other)) => false,
        // A missing object, or a reference that does not resolve.
        Err(_unresolved) => false,
    }
}

/// The failure a lopdf error stands for.
fn failure(err: &Error) -> Failure {
    match err {
        Error::Decompress(DecompressError::MemoryLimitExceeded { .. }) => Failure::Over,
        // Any other error: the content or font does not parse.
        _unreadable => Failure::Unreadable,
    }
}

/// One content stream being walked: the page's or a form's.
struct Frame<'p> {
    ops: std::vec::IntoIter<Operation>,
    /// The resource dictionaries its names resolve in, the first match winning.
    resources: Vec<&'p Dictionary>,
    /// The fonts met so far by resource name: their encoding, `None` when it does not read.
    fonts: BTreeMap<Vec<u8>, Option<Rc<Encoding<'p>>>>,
    /// The resource name of the current font.
    font: Option<Vec<u8>>,
    /// The form read, `None` for the page.
    form: Option<ObjectId>,
    /// What the form drew, when it has resources of its own: kept once it is read whole.
    record: Option<Record>,
}

impl<'p> Frame<'p> {
    fn new(
        ops: Vec<Operation>,
        resources: Vec<&'p Dictionary>,
        form: Option<ObjectId>,
        record: Option<Record>,
    ) -> Self {
        Self { ops: ops.into_iter(), resources, fonts: BTreeMap::new(), font: None, form, record }
    }
}

/// The walk's state across the frames of one page.
struct Walker<'p, 'c> {
    pdf: &'p Document,
    cache: &'c mut Cache<'p>,
    /// The limit of one font's decoded data.
    limit: usize,
    /// Content bytes the page's forms may still decode.
    left: usize,
    /// Forms already read on this page.
    read: BTreeSet<ObjectId>,
    /// Images already listed for this page.
    drawn: BTreeSet<ObjectId>,
    /// The text reached the limit: the walk stops.
    full: bool,
    /// The vertical offset of the last `Tm` in this text object, until a `Td` moves the line.
    tm_y: Option<f32>,
    out: Walk,
}

impl<'p> Walker<'p, '_> {
    /// Walks `page` and the forms it draws, in drawing order.
    fn run(&mut self, page: Frame<'p>) {
        let mut stack = vec![page];
        while let Some(frame) = stack.last_mut() {
            if self.full {
                break;
            }
            let Some(op) = frame.ops.next() else {
                if let Some(done) = stack.pop() {
                    self.keep(done);
                }
                continue;
            };
            match op.operator.as_str() {
                "Tf" => self.set_font(frame, &op.operands),
                "Tj" | "TJ" => self.show(frame, &op.operands),
                "'" => {
                    new_line(&mut self.out.text);
                    self.show(frame, &op.operands);
                }
                "\"" => {
                    new_line(&mut self.out.text);
                    self.show(frame, op.operands.get(2..).unwrap_or_default());
                }
                "T*" | "ET" => new_line(&mut self.out.text),
                "BT" => self.tm_y = None,
                "Td" | "TD" => self.moved(&op.operands),
                "Tm" => self.matrix(&op.operands),
                "BI" => self.out.inline_images += 1,
                "Do" => match self.draw(&op.operands, &stack) {
                    Some(Next::Form(form)) => stack.push(form),
                    Some(Next::Image(id)) => self.image(id, &mut stack),
                    Some(Next::Cached(drawn)) => self.replay(&drawn, &mut stack),
                    Some(Next::Partial) => {
                        stack.iter_mut().filter_map(|f| f.record.as_mut()).for_each(|r| r.partial = true);
                    }
                    None => {}
                },
                // Any other operator positions, paints or styles: it shows no text.
                _other => {}
            }
        }
    }

    /// Lists image `id`, once per page, and in what every form being recorded drew.
    fn image(&mut self, id: ObjectId, stack: &mut [Frame<'p>]) {
        if self.drawn.insert(id) {
            self.out.images.push(id);
        }
        for record in stack.iter_mut().filter_map(|f| f.record.as_mut()) {
            record.images.push(id);
        }
    }

    /// Draws again what a form read whole on an earlier page drew.
    fn replay(&mut self, drawn: &Drawn, stack: &mut [Frame<'p>]) {
        self.out.text.push_str(&drawn.text);
        self.out.inline_images += drawn.inline_images;
        self.out.forms_unread += drawn.forms_unread;
        self.out.broken += drawn.broken;
        for &id in &drawn.images {
            self.image(id, stack);
        }
        self.cap();
    }

    /// Keeps what the form of `frame`, walked to its end, drew: when it was recorded, read
    /// whole, and its text fits what the cache has left.
    fn keep(&mut self, frame: Frame<'p>) {
        let (Some(id), Some(record)) = (frame.form, frame.record) else { return };
        if record.partial || self.full || self.out.failure != record.failure {
            return;
        }
        let text = self.out.text.get(record.text..).unwrap_or_default();
        if self.cache.text.saturating_add(text.len()) > self.limit {
            return;
        }
        self.cache.text += text.len();
        let drawn = Drawn {
            text: text.to_owned(),
            images: record.images,
            inline_images: self.out.inline_images - record.inline_images,
            forms_unread: self.out.forms_unread - record.forms_unread,
            broken: self.out.broken - record.broken,
        };
        self.cache.forms.insert(id, Rc::new(drawn));
    }

    /// Handles `Td` / `TD`: a vertical move starts a new line, a move right on the same line
    /// leaves a space. A generator placing each glyph by `Td` gets spaces between them: an
    /// accepted cost, words never glued being the point.
    fn moved(&mut self, operands: &[Object]) {
        // A missing or non-numeric operand moves nothing on that axis, as a viewer draws it.
        let number = |i: usize| operands.get(i).and_then(|o| o.as_float().ok()).unwrap_or(0.0);
        let (tx, ty) = (number(0), number(1));
        if ty != 0.0 {
            self.tm_y = None;
            new_line(&mut self.out.text);
        } else if tx > 0.0 {
            space(&mut self.out.text);
        }
    }

    /// Handles `Tm`: a new line unless its vertical offset is the last `Tm`'s in this text
    /// object (within [`SAME_LINE`]), then a space.
    fn matrix(&mut self, operands: &[Object]) {
        let y = operands.get(5).and_then(|o| o.as_float().ok());
        match (y, self.tm_y) {
            (Some(y), Some(last)) if (y - last).abs() < SAME_LINE => space(&mut self.out.text),
            _ => new_line(&mut self.out.text),
        }
        self.tm_y = y;
    }

    /// Makes the font named by `Tf`'s operands current, reading its encoding once.
    fn set_font(&mut self, frame: &mut Frame<'p>, operands: &[Object]) {
        let Some(name) = operands.first().and_then(|o| o.as_name().ok()) else {
            self.out.fail(Failure::Unreadable);
            frame.font = None;
            return;
        };
        if !frame.fonts.contains_key(name) {
            let encoding = self.encoding(&frame.resources, name);
            frame.fonts.insert(name.to_vec(), encoding);
        }
        frame.font = Some(name.to_vec());
    }

    /// The encoding of the font named `name` in `resources`, its failure recorded; read once
    /// per document when the font is an indirect object.
    fn encoding(&mut self, resources: &[&'p Dictionary], name: &[u8]) -> Option<Rc<Encoding<'p>>> {
        let (id, font) = match resource(self.pdf, resources, b"Font", name) {
            Ok(Some((id, font))) => (id, font.as_dict().ok()),
            Ok(None) | Err(()) => (None, None),
        };
        let cached = id.and_then(|id| self.cache.fonts.get(&id)).cloned();
        let encoding = cached.unwrap_or_else(|| {
            let read = font
                .ok_or(Failure::Unreadable)
                .and_then(|font| self.charge(font).map(|limit| (font, limit)))
                .and_then(|(font, limit)| font.get_font_encoding_with_limit(self.pdf, limit).map_err(|e| failure(&e)))
                .map(Rc::new);
            if let Some(id) = id {
                self.cache.fonts.insert(id, read.clone());
            }
            read
        });
        encoding.map_err(|why| self.out.fail(why)).ok()
    }

    /// Charges the decoded size of `font`'s `ToUnicode` map to what the page may still decode
    /// (t-1760), returning the limit to read the font's encoding with. A map past what is left
    /// spends it: the font is over and the document's total runs out, the pages left cut; one
    /// that does not decode makes the font unreadable.
    fn charge(&mut self, font: &Dictionary) -> Result<usize, Failure> {
        let Ok(map) = font.get_deref(b"ToUnicode", self.pdf).and_then(Object::as_stream) else {
            // No map: the encoding is a name or a table, a few hundred bytes at most.
            return Ok(self.limit);
        };
        match map.decompressed_content_with_limit(self.left.min(self.limit)) {
            Ok(data) => {
                self.left = self.left.saturating_sub(data.len());
                Ok(data.len())
            }
            Err(e) => {
                let why = failure(&e);
                if why == Failure::Over {
                    // Inflated up to what was left before failing.
                    self.left = 0;
                }
                // A map that does not decode leaves the font's text unreadable, never guessed.
                Err(why)
            }
        }
    }

    /// Appends the strings of a text-showing operator, decoded with the current font.
    fn show(&mut self, frame: &Frame<'p>, operands: &[Object]) {
        let encoding = frame.font.as_ref().and_then(|name| frame.fonts.get(name)).and_then(Option::as_ref);
        let Some(encoding) = encoding else {
            // No font set, or one whose encoding does not read: the text cannot be decoded.
            self.out.fail(Failure::Unreadable);
            return;
        };
        if collect(&mut self.out.text, encoding, operands, self.limit).is_err() {
            self.out.fail(Failure::Unreadable);
        }
        self.cap();
    }

    /// Cuts the text to the limit, the walk stopping, once it is past it.
    fn cap(&mut self) {
        if self.out.text.len() > self.limit {
            let mut end = self.limit;
            while !self.out.text.is_char_boundary(end) {
                end -= 1;
            }
            self.out.text.truncate(end);
            self.out.fail(Failure::Over);
            self.full = true;
        }
    }

    /// Handles a `Do` in the frame atop `stack`: an image or a form to read is returned,
    /// anything else counted as broken or unread.
    fn draw(&mut self, operands: &[Object], stack: &[Frame<'p>]) -> Option<Next<'p>> {
        let pdf = self.pdf;
        let parent: &[&'p Dictionary] = stack.last().map_or(&[], |f| f.resources.as_slice());
        let target =
            operands.first().and_then(|o| o.as_name().ok()).map(|name| resource(pdf, parent, b"XObject", name));
        let Some(Ok(Some((Some(id), object)))) = target else {
            self.out.broken += 1;
            return None;
        };
        let Ok(stream) = object.as_stream() else {
            self.out.broken += 1;
            return None;
        };
        match stream.dict.get(b"Subtype").and_then(Object::as_name) {
            Ok(b"Image") => Some(Next::Image(id)),
            Ok(b"Form") => self.form(id, stream, parent, stack),
            // A PostScript XObject, or one of no known subtype: what it draws cannot be read.
            Ok(_) | Err(_) => {
                self.out.broken += 1;
                None
            }
        }
    }

    /// What drawing form `id` leads to: its frame, what an earlier page read of it, or
    /// nothing when it is unreadable; too deep, drawing itself or read already on this page,
    /// it is not read here.
    fn form(
        &mut self,
        id: ObjectId,
        stream: &'p lopdf::Stream,
        parent: &[&'p Dictionary],
        stack: &[Frame<'p>],
    ) -> Option<Next<'p>> {
        if stack.len() > MAX_FORM_DEPTH || stack.iter().any(|f| f.form == Some(id)) {
            self.out.forms_unread += 1;
            return Some(Next::Partial);
        }
        if !self.read.insert(id) {
            // Drawn again on this page: its text is already in once.
            return Some(Next::Partial);
        }
        if let Some(drawn) = self.cache.forms.get(&id) {
            return Some(Next::Cached(Rc::clone(drawn)));
        }
        let (resources, own) = match stream.dict.get(b"Resources") {
            // A form without resources uses those of what draws it: what it draws depends on it.
            Err(_) => (Some(parent.to_vec()), false),
            Ok(r) => (self.pdf.dereference(r).and_then(|(_, r)| r.as_dict()).ok().map(|d| vec![d]), true),
        };
        let data = match stream.decompressed_content_with_limit(self.left) {
            Ok(data) => Some(data),
            Err(e) => {
                if failure(&e) == Failure::Over {
                    self.out.fail(Failure::Over);
                }
                None
            }
        };
        let ops = data.and_then(|data| {
            self.left = self.left.saturating_sub(data.len());
            Content::decode(&data).ok()
        });
        if let (Some(resources), Some(content)) = (resources, ops) {
            let record = own.then(|| Record {
                text: self.out.text.len(),
                images: Vec::new(),
                inline_images: self.out.inline_images,
                forms_unread: self.out.forms_unread,
                broken: self.out.broken,
                failure: self.out.failure,
                partial: false,
            });
            Some(Next::Form(Frame::new(content.operations, resources, Some(id), record)))
        } else {
            self.out.forms_unread += 1;
            None
        }
    }
}

/// The entry `name` of the `category` resource dictionary (`Font`, `XObject`) of the first of
/// `resources` that has it, dereferenced, with its object id when indirect.
///
/// `Ok(None)` when no dictionary has it; `Err` when one that does fails to resolve: a broken
/// reference is unreadable, never absent.
fn resource<'p>(
    pdf: &'p Document,
    resources: &[&'p Dictionary],
    category: &[u8],
    name: &[u8],
) -> Result<Option<(Option<ObjectId>, &'p Object)>, ()> {
    for dict in resources {
        // `Dictionary::get` fails only on a missing key.
        let Ok(entries) = dict.get(category) else { continue };
        let entries = pdf.dereference(entries).and_then(|(_, e)| e.as_dict()).map_err(|_| ())?;
        let Ok(entry) = entries.get(name) else { continue };
        return pdf.dereference(entry).map(Some).map_err(|_| ());
    }
    Ok(None)
}

/// Separates the next word of `text` by a space, unless it starts the text or follows one.
fn space(text: &mut String) {
    if !text.is_empty() && !text.ends_with(char::is_whitespace) {
        text.push(' ');
    }
}

/// Starts a new line of `text`, unless it is at one already.
fn new_line(text: &mut String) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
}

/// Appends the strings of `operands` decoded with `encoding`, stopping once `text` is past
/// `limit` bytes; a wide negative adjustment in a `TJ` array stands for a space.
fn collect(text: &mut String, encoding: &Encoding<'_>, operands: &[Object], limit: usize) -> lopdf::Result<()> {
    for operand in operands {
        if text.len() > limit {
            return Ok(());
        }
        match operand {
            Object::String(bytes, _) => decode(text, encoding, bytes, limit)?,
            Object::Array(items) => {
                collect(text, encoding, items, limit)?;
                text.push(' ');
            }
            Object::Integer(i) if *i < -100 => text.push(' '),
            Object::Real(r) if *r < -100.0 => text.push(' '),
            // Spacing and other operands: no character to collect.
            _other => {}
        }
    }
    Ok(())
}

/// Appends `bytes` decoded with `encoding`, a `ToUnicode` map read only until `text` would pass
/// `limit` bytes.
///
/// The map's codes are matched as lopdf's `write_to_string` does (shortest code first, at most
/// 4 bytes), but its output is checked as it grows: one code may map to a long string.
fn decode(text: &mut String, encoding: &Encoding<'_>, bytes: &[u8], limit: usize) -> lopdf::Result<()> {
    let Encoding::UnicodeMapEncoding(map) = encoding else {
        // The other encodings give at most one character per byte, and the content is bounded.
        return encoding.write_to_string(bytes, text);
    };
    let room = limit.saturating_sub(text.len());
    let mut units: Vec<u16> = Vec::new();
    let (mut code, mut len) = (0u32, 0u8);
    for &byte in bytes {
        if units.len() > room {
            break;
        }
        if len == 4 {
            units.extend(map.get_or_replacement_char(code, 4));
            (code, len) = (0, 0);
        }
        len += 1;
        code = (code << 8) | u32::from(byte);
        if let Some(value) = map.get(code, len) {
            units.extend(value);
            (code, len) = (0, 0);
        }
    }
    if len > 0 && units.len() <= room {
        units.extend(map.get_or_replacement_char(code, len));
    }
    text.extend(char::decode_utf16(units).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)));
    Ok(())
}

#[cfg(test)]
#[path = "pdf_content_tests.rs"]
mod pdf_content_tests;
