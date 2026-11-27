//! Markdown and plain text documents.

use oku_slides_core::knowledge::{ExtractLimits, ExtractRefusal, split};

use crate::finish;
use crate::markdown::{clean, escape_text, fence_for, sanitize_markdown};
use oku_slides_core::knowledge::Extracted;

/// A Markdown document: its frontmatter stripped (the body kept), then sanitized.
///
/// A frontmatter that does not parse (no closing `---`, say) is not one: the whole text is the
/// body, its opening lines sanitized like any other Markdown and kept, never dropped.
pub(crate) fn markdown(bytes: &[u8], limits: &ExtractLimits) -> Result<Extracted, ExtractRefusal> {
    let text = utf8(bytes)?;
    let body = split(text).map_or(text, |(_, body)| body);
    finish("", &sanitize_markdown(body), "", limits, Vec::new(), 0)
}

/// A plain text document: escaped, or fenced whole when a line would open a code fence or a
/// frontmatter (it starts with three backticks or `---`).
pub(crate) fn plain(bytes: &[u8], limits: &ExtractLimits) -> Result<Extracted, ExtractRefusal> {
    let text = clean(utf8(bytes)?);
    if text.lines().any(|l| l.starts_with("```") || l.starts_with("---")) {
        let fence = fence_for(&text);
        let body = text.trim_end_matches('\n');
        finish(&format!("{fence}\n"), body, &format!("\n{fence}\n"), limits, Vec::new(), 0)
    } else {
        finish("", &escape_text(&text), "", limits, Vec::new(), 0)
    }
}

/// The UTF-8 text of the bytes, its BOM removed; other bytes are not a text document.
fn utf8(bytes: &[u8]) -> Result<&str, ExtractRefusal> {
    let text = std::str::from_utf8(bytes).map_err(|_| ExtractRefusal::TypeMismatch)?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(text))
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
