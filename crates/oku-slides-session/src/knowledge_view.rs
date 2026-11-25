//! What the agent may read of the deck knowledge base: list, read and search.
//!
//! A file's text is open only when the user approved that exact text
//! (`knowledge_approvals`); an awaiting file shows its path and nothing else, in
//! every view. The hash is taken on each call, so an edit makes the file await again.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use snafu::{OptionExt, ensure};

use oku_slides_core::knowledge::limits::MAX_FILE_BYTES;
use oku_slides_core::{KnowledgeFile, KnowledgePath, asset_id, cited_images, frontmatter, media};
use oku_slides_platform::config::KnowledgeSettings;

use crate::agent::bounds::{clip, label_within};
use crate::error::{
    KnowledgeAwaitingSnafu, KnowledgeDirInvalidSnafu, KnowledgeNotFoundSnafu, KnowledgePathInvalidSnafu,
    KnowledgeQueryInvalidSnafu,
};
use crate::{Result, Session};

/// Most characters of a knowledge search query.
pub const QUERY_MAX_CHARS: usize = 200;

/// Most characters of the line shown for one search hit.
const HIT_MAX_CHARS: usize = 200;

/// Most characters of a listed `type` or `status` value.
const KIND_MAX: usize = 40;
/// Most characters of a listed `title` value.
const TITLE_MAX: usize = 80;
/// Most characters of a listed `description` value.
const DESCRIPTION_MAX: usize = 200;

/// One file of the knowledge base as listed; an awaiting file shows only its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeEntry {
    /// Bundle-absolute path (`/a/b.md`).
    pub path: String,
    /// True when the user approved this exact text.
    pub approved: bool,
    /// The frontmatter `type`; `None` when awaiting.
    pub kind: Option<String>,
    /// The frontmatter `title`; `None` when awaiting.
    pub title: Option<String>,
    /// The frontmatter `description`; `None` when awaiting.
    pub description: Option<String>,
    /// The frontmatter `status`; `None` when awaiting.
    pub status: Option<String>,
    /// The frontmatter `stale: true`; false when awaiting.
    pub stale: bool,
}

impl KnowledgeEntry {
    /// The entry as the agent is shown it, in the index and by `knowledge_list` alike.
    ///
    /// Each frontmatter value goes through `label_within` (controls, bidi and hidden characters
    /// dropped, whitespace collapsed, quotes made inert), cut at 40 characters for `type` and
    /// `status`, 80 for `title`, 200 for `description`.
    #[must_use]
    pub fn bounded(&self) -> Self {
        let within = |v: &Option<String>, max| v.as_deref().map(|v| label_within(v, max));
        Self {
            path: self.path.clone(),
            approved: self.approved,
            kind: within(&self.kind, KIND_MAX),
            title: within(&self.title, TITLE_MAX),
            description: within(&self.description, DESCRIPTION_MAX),
            status: within(&self.status, KIND_MAX),
            stale: self.stale,
        }
    }
}

/// An approved knowledge file, as the agent reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeRead {
    /// Bundle-absolute path.
    pub path: String,
    /// The text, at most `MAX_FILE_BYTES` bytes, cut on a character boundary.
    pub text: String,
    /// Bytes of the file left out of `text`.
    pub truncated: usize,
    /// The images `text` cites, by asset id.
    pub images: Vec<KnowledgeImage>,
}

/// An image cited by a knowledge file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeImage {
    /// Asset id (`sha256:<hex>`).
    pub asset: String,
    /// Its bundle path (`/references/assets/<hex>.<ext>`), as first cited.
    pub path: String,
    /// Media type sniffed from the bytes; `application/octet-stream` when absent.
    pub mime: &'static str,
    /// Width in pixels; 0 when absent or not measurable (SVG, unknown format).
    pub width: u32,
    /// Height in pixels; 0 when absent or not measurable.
    pub height: u32,
    /// The bytes; `None` when the deck does not hold the asset.
    pub bytes: Option<Arc<[u8]>>,
}

/// The lines of approved files matching a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeHits {
    /// The first hits, by path then line.
    pub hits: Vec<KnowledgeHit>,
    /// Matching lines left out past the hit limit.
    pub more: usize,
}

/// One matching line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeHit {
    /// Bundle-absolute path of the file.
    pub path: String,
    /// Line number, from 1.
    pub line: usize,
    /// The line, clipped to 200 characters.
    pub text: String,
}

/// The `[knowledge]` values already logged as above their ceiling, by key and value.
static LOGGED_ABOVE: Mutex<BTreeSet<(&str, usize)>> = Mutex::new(BTreeSet::new());

/// The `[knowledge]` settings the session keeps; a value above its ceiling is logged once per process.
pub(crate) fn knowledge_settings(settings: &KnowledgeSettings) -> KnowledgeSettings {
    let checks = [
        ("index_entries", settings.index_entries.get(), KnowledgeSettings::INDEX_ENTRIES_MAX),
        ("index_kib", settings.index_kib.get(), KnowledgeSettings::INDEX_KIB_MAX),
        ("search_hits", settings.search_hits.get(), KnowledgeSettings::SEARCH_HITS_MAX),
    ];
    for (key, value, ceiling) in checks {
        if value > ceiling && LOGGED_ABOVE.lock().unwrap_or_else(PoisonError::into_inner).insert((key, value)) {
            tracing::warn!(key, value, ceiling, "knowledge setting above its ceiling: the ceiling applies");
        }
    }
    settings.clone()
}

/// `text` cut to at most `max` bytes, on a character boundary.
pub(crate) fn cut(text: &str, max: usize) -> &str {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

impl Session {
    /// The knowledge files, sorted by path; under `dir` (`/references`) when given.
    ///
    /// # Errors
    ///
    /// `KnowledgeDirInvalid` when `dir` is not a bundle-absolute folder.
    pub fn knowledge_entries(&self, dir: Option<&str>) -> Result<Vec<KnowledgeEntry>> {
        let prefix = match dir {
            None | Some("/") => None,
            Some(d) => {
                let trimmed = d.strip_suffix('/').unwrap_or(d);
                let path = KnowledgePath::parse(trimmed).ok().context(KnowledgeDirInvalidSnafu { dir: d })?;
                Some(format!("{}/", path.as_str()))
            }
        };
        Ok(self.knowledge_entries_under(prefix.as_deref()))
    }

    /// Every knowledge file, sorted by path, as [`Session::knowledge_entries`] lists them.
    pub(crate) fn all_knowledge_entries(&self) -> Vec<KnowledgeEntry> {
        self.knowledge_entries_under(None)
    }

    /// The knowledge files whose path starts with `prefix` (all without one), sorted by path.
    fn knowledge_entries_under(&self, prefix: Option<&str>) -> Vec<KnowledgeEntry> {
        let mut entries: Vec<KnowledgeEntry> = self
            .knowledge_files()
            .into_iter()
            .filter(|f| prefix.is_none_or(|p| f.path.starts_with(p)))
            .map(|f| self.knowledge_entry(f))
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }

    /// The approved file at `path`, cut to `MAX_FILE_BYTES`, with the images that cut text cites.
    ///
    /// # Errors
    ///
    /// `KnowledgePathInvalid` for a malformed path, `KnowledgeNotFound` when no file has it,
    /// `KnowledgeAwaiting` when its text is not approved.
    pub fn knowledge_file(&self, path: &str) -> Result<KnowledgeRead> {
        KnowledgePath::parse(path).map_err(|reason| KnowledgePathInvalidSnafu { path, reason }.build())?;
        let file =
            self.knowledge_files().into_iter().find(|f| f.path == path).context(KnowledgeNotFoundSnafu { path })?;
        ensure!(self.knowledge_approved(file), KnowledgeAwaitingSnafu { path });
        let text = cut(&file.text, MAX_FILE_BYTES);
        let images =
            cited_images(&file.path, text).into_iter().map(|(asset, path)| self.knowledge_image(asset, path)).collect();
        Ok(KnowledgeRead {
            path: file.path.clone(),
            text: text.to_owned(),
            truncated: file.text.len() - text.len(),
            images,
        })
    }

    /// The lines of approved files containing `query`, ignoring case and accents.
    ///
    /// Each file is searched as `knowledge_file` reads it: its first `MAX_FILE_BYTES` bytes.
    ///
    /// At most `[knowledge] search_hits` hits (by path, then line); `more` counts the rest.
    ///
    /// # Errors
    ///
    /// `KnowledgeQueryInvalid` when `query` is blank or longer than [`QUERY_MAX_CHARS`].
    pub fn knowledge_search(&self, query: &str) -> Result<KnowledgeHits> {
        ensure!(query.chars().count() <= QUERY_MAX_CHARS, KnowledgeQueryInvalidSnafu { max: QUERY_MAX_CHARS });
        let needle = oku_slides_i18n::fold(query);
        // Checked after folding: a query of accents alone folds to nothing and would match every line.
        ensure!(!needle.trim().is_empty(), KnowledgeQueryInvalidSnafu { max: QUERY_MAX_CHARS });
        let limit = self.knowledge_settings.effective_search_hits();
        let mut files: Vec<&KnowledgeFile> =
            self.knowledge_files().into_iter().filter(|f| self.knowledge_approved(f)).collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let (mut hits, mut more) = (Vec::new(), 0);
        for file in files {
            for (n, line) in cut(&file.text, MAX_FILE_BYTES).lines().enumerate() {
                if !oku_slides_i18n::fold(line).contains(&needle) {
                    continue;
                }
                if hits.len() < limit {
                    hits.push(KnowledgeHit { path: file.path.clone(), line: n + 1, text: clip(line, HIT_MAX_CHARS) });
                } else {
                    more += 1;
                }
            }
        }
        Ok(KnowledgeHits { hits, more })
    }

    /// The files of the deck shown, in no order.
    fn knowledge_files(&self) -> Vec<&KnowledgeFile> {
        self.doc.sources.deck().knowledge.values().collect()
    }

    /// True when the user approved this exact text of `file`.
    pub(crate) fn knowledge_approved(&self, file: &KnowledgeFile) -> bool {
        self.library.knowledge_approvals().is_approved(&asset_id(file.text.as_bytes()))
    }

    /// The listing of `file`: its frontmatter when approved, its path only otherwise.
    fn knowledge_entry(&self, file: &KnowledgeFile) -> KnowledgeEntry {
        let awaiting = KnowledgeEntry {
            path: file.path.clone(),
            approved: false,
            kind: None,
            title: None,
            description: None,
            status: None,
            stale: false,
        };
        if !self.knowledge_approved(file) {
            return awaiting;
        }
        // `log.md` and every non-root `index.md` carry no frontmatter: listed without fields.
        match frontmatter(&file.text) {
            Ok(f) => KnowledgeEntry {
                approved: true,
                kind: f.kind,
                title: f.title,
                description: f.description,
                status: f.status,
                stale: f.stale,
                ..awaiting
            },
            Err(_) => KnowledgeEntry { approved: true, ..awaiting },
        }
    }

    /// The cited image `asset` at bundle path `path`, with its bytes when the deck holds them.
    fn knowledge_image(&self, asset: String, path: String) -> KnowledgeImage {
        let Some(bytes) = self.doc.sources.assets.get(&asset) else {
            return KnowledgeImage { asset, path, mime: media::OCTET_STREAM, width: 0, height: 0, bytes: None };
        };
        let mime = media::sniff(bytes);
        let (width, height) = media::dimensions(bytes, mime).unwrap_or((0, 0));
        KnowledgeImage { asset, path, mime, width, height, bytes: Some(Arc::from(bytes.as_slice())) }
    }
}

#[cfg(test)]
#[path = "knowledge_view_tests.rs"]
mod knowledge_view_tests;
