//! The characters imported knowledge text never keeps, and the escaping of text within a line.
//!
//! The extractor (`oku-slides-extract`) strips them in its worker; the host checks the
//! worker's Markdown with [`is_clean_text`] before trusting it, and the session escapes an
//! attachment's name with [`escape_inline`].

use std::borrow::Cow;

/// Characters escaped wherever they appear in extracted text.
const SPECIAL: &[char] = &['\\', '`', '*', '_', '[', ']', '<', '>', '!', '|', '&', '~'];

/// Format characters (Unicode `Cf`) outside the tag block, which [`is_stripped`] also drops.
const FORMAT_CHARS: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0000, 0xE007F),
];

/// True for a character extracted text never keeps: format (`Cf`, bidi and tag characters
/// included) and control characters other than tab and line feed.
fn is_stripped(c: char) -> bool {
    let u = u32::from(c);
    (c.is_control() && c != '\t' && c != '\n') || FORMAT_CHARS.iter().any(|&(lo, hi)| (lo..=hi).contains(&u))
}

/// True for printable ASCII, tab and line feed: bytes [`clean_text`] never changes.
fn is_plain_byte(b: u8) -> bool {
    (0x20..0x7F).contains(&b) || b == b'\n' || b == b'\t'
}

/// Whether `text` is as [`clean_text`] leaves it: no carriage return and no stripped
/// character.
#[must_use]
pub fn is_clean_text(text: &str) -> bool {
    text.bytes().all(is_plain_byte) || !text.chars().any(|c| c == '\r' || is_stripped(c))
}

/// Text with line ends as `\n` and the stripped characters (controls other than tab and line
/// feed, format, bidi and tag characters) removed; borrowed when nothing changes.
#[must_use]
pub fn clean_text(text: &str) -> Cow<'_, str> {
    if is_clean_text(text) {
        return Cow::Borrowed(text);
    }
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    Cow::Owned(text.chars().filter(|&c| !is_stripped(c)).collect())
}

/// Text already through [`clean_text`] with every Markdown special character escaped and each
/// line end turned into a space: safe within a line (a table cell, a heading, a banner).
#[must_use]
pub fn escape_specials(clean: &str) -> String {
    let mut out = String::with_capacity(clean.len() + clean.len() / 8);
    for c in clean.chars() {
        if c == '\n' {
            out.push(' ');
            continue;
        }
        if SPECIAL.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Any text cleaned ([`clean_text`]) then escaped ([`escape_specials`]): safe within a line.
#[must_use]
pub fn escape_inline(text: &str) -> String {
    escape_specials(&clean_text(text))
}

#[cfg(test)]
#[path = "sanitize_tests.rs"]
mod sanitize_tests;
