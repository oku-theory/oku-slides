//! Images a knowledge file cites: CommonMark `![alt](url)` pointing into the asset folder.
//!
//! Only CommonMark images count (default parser options, no extension). An HTML `<img>` tag is
//! not scanned: it is raw HTML, which this scan does not read. URLs are untrusted text: nothing
//! is fetched, only matched against `/references/assets/<64 hex>.<ext>`.

use std::collections::BTreeSet;

use pulldown_cmark::{Event, Parser, Tag};

use super::{KnowledgeFile, split};

/// The bundle folder that holds cited image assets.
pub const ASSETS_DIR: &str = "/references/assets/";

/// The longest file extension of a cited asset.
const EXT_MAX: usize = 10;

/// The asset ids (`sha256:<hex>`) that images of `file` cite.
///
/// The frontmatter is not read (an image there is YAML text); when no frontmatter block is found
/// (no opening `---` line, or no closing one) the whole text is scanned.
#[must_use]
pub fn cited_assets(file: &KnowledgeFile) -> BTreeSet<String> {
    let body = split(&file.text).map_or(file.text.as_str(), |(_, body)| body);
    let dir = file.path.rsplit_once('/').map_or("", |(dir, _)| dir);
    Parser::new(body)
        .filter_map(|event| match event {
            Event::Start(Tag::Image { dest_url, .. }) => asset_id_of(dir, &dest_url),
            _ => None,
        })
        .collect()
}

/// The asset `url` names once resolved against `dir`, if it is one.
fn asset_id_of(dir: &str, url: &str) -> Option<String> {
    if url.starts_with("//") || url.contains(['?', '#', '%']) {
        return None;
    }
    let first = url.split('/').next().unwrap_or_default();
    if first.contains(':') {
        return None;
    }
    let mut segments: Vec<&str> = Vec::new();
    let (base, path) = match url.strip_prefix('/') {
        Some(rest) => ("", rest),
        None => (dir, url),
    };
    segments.extend(base.split('/').filter(|segment| !segment.is_empty()));
    // The URL part refuses an empty segment (`//`, a trailing `/`); `./` is a no-op.
    for segment in path.split('/') {
        match segment {
            "" => return None,
            "." => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    let [dir_a, dir_b, name] = segments.as_slice() else {
        return None;
    };
    if format!("/{dir_a}/{dir_b}/") != ASSETS_DIR {
        return None;
    }
    let (hex, ext) = name.split_once('.')?;
    let hex_ok = hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    let ext_ok =
        (1..=EXT_MAX).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    (hex_ok && ext_ok).then(|| format!("sha256:{hex}"))
}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod assets_tests;
