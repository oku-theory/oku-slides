//! The one sanitizer of extracted text: control characters stripped, Markdown escaped, links
//! limited to `http`, `https` and `mailto`, Markdown documents rid of HTML and remote images,
//! and the table and cut writers every format shares.

use std::ops::Range;

use oku_slides_core::encode_link_url;
use oku_slides_core::knowledge::{Cut, CutKind, escape_specials};
pub(crate) use oku_slides_core::knowledge::{clean_text as clean, escape_inline};
use oku_slides_core::num::count_u64;
use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

/// The Markdown extensions every parse of a document enables.
const OPTIONS: Options = Options::ENABLE_TABLES.union(Options::ENABLE_STRIKETHROUGH);

/// Re-parses of a Markdown document before it is given up as unsafe and fenced.
const MAX_PASSES: usize = 8;

/// Cleaned text escaped so that Markdown shows it as written: cleaned once, then
/// [`escape_specials`] and [`line_start`] on each line.
pub(crate) fn escape_text(text: &str) -> String {
    clean(text).split('\n').map(|line| line_start(&escape_specials(line))).collect::<Vec<_>>().join("\n")
}

/// A line of escaped inline Markdown made safe as a line start: leading spaces and tabs are
/// dropped so that it does not become indented code, and the marks that open a heading, a
/// list item, a setext underline or a numbered item are escaped.
pub(crate) fn line_start(line: &str) -> String {
    let body = line.trim_start_matches([' ', '\t']);
    let digits = body.len() - body.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let mut out = String::with_capacity(body.len() + 1);
    if body.starts_with(['#', '-', '+', '=']) {
        out.push('\\');
    }
    if digits > 0 && body[digits..].starts_with(['.', ')']) {
        out.push_str(&body[..digits]);
        out.push('\\');
        out.push_str(&body[digits..]);
    } else {
        out.push_str(body);
    }
    out
}

/// Text of one table cell or heading: escaped, on one line.
pub(crate) fn escape_cell(text: &str) -> String {
    escape_inline(&text.replace(['\n', '\t'], " "))
}

/// The URL a link keeps, encoded, or `None` when its scheme is not `http`, `https` or `mailto`.
///
/// A URL with a control character (a line end among them) is refused: it could end the link
/// and start a line of Markdown.
pub(crate) fn link_url(url: &str) -> Option<String> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    let web = ["http://", "https://", "mailto:"].iter().any(|s| lower.starts_with(s));
    (web && !url.chars().any(char::is_control)).then(|| encode_link_url(url))
}

/// A GFM table whose first row is the header; every row is padded to the widest one.
///
/// Cells are raw text, escaped here. No rows give an empty string.
pub(crate) fn table(rows: &[Vec<String>]) -> String {
    let width = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut out = String::new();
    for (i, row) in rows.iter().enumerate() {
        out.push('|');
        for c in 0..width {
            out.push(' ');
            out.push_str(&escape_cell(row.get(c).map_or("", String::as_str)));
            out.push_str(" |");
        }
        out.push('\n');
        if i == 0 {
            out.push('|');
            out.push_str(&" --- |".repeat(width));
            out.push('\n');
        }
    }
    out
}

/// What bounded a cut, as its notice says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bound<'a> {
    /// A count or size limit: `(limit 2000)`.
    Limit(usize),
    /// The document's text budget, spent: `(text budget spent)`.
    Budget,
    /// The pixel limit of one image: `(over 1000 pixels)`.
    Pixels(u64),
    /// The byte limit of all images: `(past 1000 bytes of images)`.
    ImageBytes(u64),
    /// A media type that is not imported: `(image/emf)`.
    Format(&'a str),
    /// An image whose bytes or header do not read: `(unreadable)`.
    Unreadable,
    /// A part past the archive's read limits: `(past the read limits)`.
    ReadLimit,
    /// Nesting past what the reader follows: `(too deep)`.
    TooDeep,
}

/// The line stating a cut, in the document: `… 12 rows not imported (limit 2000)`.
pub(crate) fn cut_line(cut: Cut, bound: Bound<'_>) -> String {
    let what = match cut.what {
        CutKind::Sheets => "sheets",
        CutKind::Rows => "rows",
        CutKind::Columns => "columns",
        CutKind::Pages => "pages",
        CutKind::Text => "bytes",
        CutKind::Images => "images",
        CutKind::Depth => "nested elements",
    };
    let why = match bound {
        Bound::Limit(n) => format!("limit {n}"),
        Bound::Budget => "text budget spent".to_owned(),
        Bound::Pixels(n) => format!("over {n} pixels"),
        Bound::ImageBytes(n) => format!("past {n} bytes of images"),
        Bound::Format(mime) => escape_cell(mime),
        Bound::Unreadable => "unreadable".to_owned(),
        Bound::ReadLimit => "past the read limits".to_owned(),
        Bound::TooDeep => "too deep".to_owned(),
    };
    format!("… {} {what} not imported ({why})\n", cut.count)
}

/// The line stating content skipped on purpose (hidden text, deletions…), listed in `what`.
pub(crate) fn skipped_line(what: &[&str]) -> String {
    format!("… not imported: {}\n", what.join(", "))
}

/// A fence longer than every backtick run of `text`, at least three backticks.
pub(crate) fn fence_for(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}

/// `body` between `open` and `close`, cut at a line end to fit `max` bytes with the line that
/// states the cut; a fenced code block left open by the cut is closed before that line.
/// `dropped` bytes were already left out by the reader: they join the cut, which is then
/// stated even when the body fits.
///
/// Returns `None` when not even the cut line fits.
pub(crate) fn fit(open: &str, body: &str, close: &str, max: usize, dropped: u64) -> Option<(String, Option<Cut>)> {
    let whole = open.len() + body.len() + close.len();
    if whole <= max && dropped == 0 {
        return Some((format!("{open}{body}{close}"), None));
    }
    // The cut line counts at most `body.len()` bytes plus `dropped`, so its width is known
    // before the cut.
    let most = count_u64(body.len()).saturating_add(dropped);
    let notice_max = cut_line(Cut { what: CutKind::Text, count: most }, Bound::Limit(max)).len() + 2;
    let mut room = max.checked_sub(open.len() + close.len() + notice_max)?;
    let mut kept = keep(body, room);
    let mut out = format!("{open}{kept}{close}");
    if open_fence(&out).is_some() {
        // Any closing fence is at most the longest run of fence characters, plus its newline.
        room = room.checked_sub(longest_fence_run(body) + 1)?;
        kept = keep(body, room);
        out = format!("{open}{kept}{close}");
    }
    let fence = open_fence(&out);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if let Some(fence) = fence {
        out.push_str(&fence);
        out.push('\n');
    }
    let cut = Cut { what: CutKind::Text, count: count_u64(body.len() - kept.len()).saturating_add(dropped) };
    out.push('\n');
    out.push_str(&cut_line(cut, Bound::Limit(max)));
    debug_assert!(out.len() <= max, "fit gave {} bytes past its {max} byte bound", out.len());
    Some((out, Some(cut)))
}

/// `body` up to its last line end within `room` bytes (all of it when it fits).
fn keep(body: &str, room: usize) -> &str {
    if room >= body.len() {
        return body;
    }
    let mut end = room.min(body.len());
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    body[..end].rfind('\n').map_or(&body[..end], |nl| &body[..nl])
}

/// The fence that closes the fenced code block `text` ends inside, if any.
fn open_fence(text: &str) -> Option<String> {
    let probe = format!("{text}\n\nZ");
    let at = probe.len() - 1;
    Parser::new_ext(&probe, OPTIONS).into_offset_iter().find_map(|(event, range)| match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) if range.contains(&at) => {
            let line = probe[range.start..].trim_start_matches(' ');
            let mark = line.chars().next()?;
            Some(line.chars().take_while(|&c| c == mark).collect())
        }
        _ => None,
    })
}

/// The longest run of backticks or tildes in `text`.
fn longest_fence_run(text: &str) -> usize {
    text.split(|c| c != '`').chain(text.split(|c| c != '~')).map(str::len).max().unwrap_or(0)
}

/// A Markdown document rid of raw HTML, comments, reference definitions, remote images and
/// links other than `http`, `https` and `mailto`; the rest of its source is kept as written.
///
/// Each pass re-parses the result; a document still changing after [`MAX_PASSES`] is
/// returned as a fenced code block, shown and never interpreted.
pub(crate) fn sanitize_markdown(text: &str) -> String {
    let mut text = clean(text).into_owned();
    for _ in 0..MAX_PASSES {
        let edits = edits(&text);
        if edits.is_empty() {
            return text;
        }
        text = apply(&text, edits);
    }
    let fence = fence_for(&text);
    format!("{fence}\n{}\n{fence}\n", text.trim_end_matches('\n'))
}

/// An open link or image: its kind, URL, source range and the range of its children.
struct Open {
    image: bool,
    url: Option<String>,
    outer: Range<usize>,
    inner: Option<Range<usize>>,
}

/// The edits of one pass: replacements of source ranges, which may overlap (see [`apply`]).
fn edits(text: &str) -> Vec<(Range<usize>, String)> {
    let mut parser = Parser::new_ext(text, OPTIONS).into_offset_iter();
    let mut edits = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    for (event, range) in parser.by_ref() {
        if let Some(top) = open.last_mut()
            && !matches!(event, Event::End(TagEnd::Link | TagEnd::Image))
            && range.start >= top.outer.start
        {
            let inner = top.inner.get_or_insert(range.clone());
            inner.end = inner.end.max(range.end);
        }
        match event {
            Event::Html(_) | Event::InlineHtml(_) => edits.push((range, String::new())),
            Event::Start(Tag::Link { link_type, dest_url, .. }) => {
                let url =
                    if link_type == LinkType::Email { format!("mailto:{dest_url}") } else { dest_url.to_string() };
                open.push(Open { image: false, url: link_url(&url), outer: range, inner: None });
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let remote = dest_url.to_ascii_lowercase();
                let url = (remote.starts_with("http://") || remote.starts_with("https://"))
                    .then(|| link_url(&dest_url))
                    .flatten();
                open.push(Open { image: true, url, outer: range, inner: None });
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                if let Some(done) = open.pop() {
                    let inner = done.inner.map_or("", |r| &text[r]);
                    let with = match &done.url {
                        Some(url) => format!("[{inner}]({url})"),
                        None => inner.to_owned(),
                    };
                    if done.image || with != text[done.outer.clone()] {
                        edits.push((done.outer, with));
                    }
                }
            }
            _ => {}
        }
    }
    for (_, def) in parser.reference_definitions().iter() {
        edits.push((def.span.clone(), String::new()));
    }
    edits
}

/// `text` with the edits applied; an edit overlapping an earlier one waits for the next pass.
fn apply(text: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|(r, _)| (r.start, std::cmp::Reverse(r.end)));
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (range, with) in edits {
        if range.start < at {
            continue;
        }
        out.push_str(&text[at..range.start]);
        out.push_str(&with);
        at = range.end;
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod markdown_tests;
