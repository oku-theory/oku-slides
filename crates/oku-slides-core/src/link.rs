//! Link destinations written into Markdown.

/// `url` encoded as an inline Markdown link destination: a leading `<` would open an angle
/// destination (`%3C`), and a `\` would escape the closing parenthesis (`%5C`); spaces and
/// parentheses are percent-encoded too.
///
/// Mirrored by `encodeLink` in `ui/src`; the editor's inline writer and the knowledge
/// extractor both call it.
#[must_use]
pub fn encode_link_url(url: &str) -> String {
    let url = url.replace(' ', "%20").replace('(', "%28").replace(')', "%29").replace('\\', "%5C");
    match url.strip_prefix('<') {
        Some(rest) => format!("%3C{rest}"),
        None => url,
    }
}

#[cfg(test)]
#[path = "link_tests.rs"]
mod link_tests;
