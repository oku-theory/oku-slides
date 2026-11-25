//! Bundle-absolute knowledge paths, parsed strictly: nothing is normalized.

use uuid::Uuid;

use super::limits::{PATH_MAX_BYTES, PATH_MAX_SEGMENTS};
use crate::error::PathReason;

/// A validated knowledge path such as `/a/b.md`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KnowledgePath(String);

const RESERVED: [&str; 4] = ["con", "prn", "aux", "nul"];

/// A Windows reserved device name (`con`, `com1`…), compared on the part before the first dot.
fn is_reserved(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or_default();
    RESERVED.contains(&base)
        || (base.len() == 4
            && (base.starts_with("com") || base.starts_with("lpt"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

fn is_segment_char(c: char) -> bool {
    matches!(c, 'a'..='z' | '0'..='9' | '.' | '_' | '-')
}

impl KnowledgePath {
    /// Parses a path; refuses what is malformed, never fixes case or slashes.
    ///
    /// # Errors
    /// The [`PathReason`] of the first rule broken.
    pub fn parse(text: &str) -> Result<Self, PathReason> {
        if text.is_empty() || text == "/" {
            return Err(PathReason::Empty);
        }
        let Some(rest) = text.strip_prefix('/') else {
            return Err(PathReason::NotAbsolute);
        };
        let mut segments = 0;
        for segment in rest.split('/') {
            segments += 1;
            if segment.is_empty() || !segment.chars().all(is_segment_char) {
                return Err(PathReason::BadSegment);
            }
            if segment.starts_with('.') {
                return Err(PathReason::DotSegment);
            }
            if segment.ends_with('.') {
                return Err(PathReason::BadSegment);
            }
            if is_reserved(segment) {
                return Err(PathReason::Reserved);
            }
        }
        if segments > PATH_MAX_SEGMENTS {
            return Err(PathReason::TooDeep);
        }
        if text.len() > PATH_MAX_BYTES {
            return Err(PathReason::TooLong);
        }
        Ok(Self(text.to_owned()))
    }

    /// Like [`parse`](Self::parse), and the path must end in `.md`: what ops may write.
    ///
    /// # Errors
    /// [`PathReason::NotMarkdown`] when the extension is not `.md`, else as `parse`.
    pub fn parse_writable(text: &str) -> Result<Self, PathReason> {
        let path = Self::parse(text)?;
        if std::path::Path::new(&path.0).extension().is_some_and(|ext| ext == "md") {
            Ok(path)
        } else {
            Err(PathReason::NotMarkdown)
        }
    }

    /// The whole path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The last segment.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or_default()
    }

    /// The file name without its last extension.
    #[must_use]
    pub fn stem(&self) -> &str {
        let name = self.file_name();
        name.rsplit_once('.').map_or(name, |(stem, _)| stem)
    }

    /// The directory, without trailing slash (`""` for the root).
    #[must_use]
    pub fn dir(&self) -> &str {
        let cut = self.0.len() - self.file_name().len() - 1;
        &self.0[..cut]
    }

    /// Is the file named `index.md`?
    #[must_use]
    pub fn is_index(&self) -> bool {
        self.file_name() == "index.md"
    }

    /// Is this the bundle's `/index.md`?
    #[must_use]
    pub fn is_root_index(&self) -> bool {
        self.0 == "/index.md"
    }

    /// Is the file named `log.md`?
    #[must_use]
    pub fn is_log(&self) -> bool {
        self.file_name() == "log.md"
    }

    /// `/recovered/<id>.md`, where a file whose path is invalid goes: always a valid path,
    /// since a file id is 32 characters of `[0-9a-f]`.
    pub(super) fn recovered(id: &FileId) -> Self {
        Self(format!("/recovered/{}.md", id.as_str()))
    }

    /// The path with `-n` appended to the stem (`/a/b.md`, 2 gives `/a/b-2.md`).
    ///
    /// # Errors
    /// [`PathReason::TooLong`] when the result exceeds the byte limit.
    pub fn with_suffix(&self, n: u32) -> Result<Self, PathReason> {
        let name = self.file_name();
        let out = match name.rsplit_once('.') {
            Some((stem, ext)) => format!("{}/{stem}-{n}.{ext}", self.dir()),
            None => format!("{}/{name}-{n}", self.dir()),
        };
        if out.len() > PATH_MAX_BYTES {
            return Err(PathReason::TooLong);
        }
        Ok(Self(out))
    }
}

/// A checked file id: 32 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FileId(String);

impl FileId {
    /// The id, if `text` is one; else `text` back.
    pub(super) fn parse(text: String) -> Result<Self, String> {
        if is_file_id(&text) { Ok(Self(text)) } else { Err(text) }
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn into_string(self) -> String {
        self.0
    }
}

/// A fresh file id: 32 lowercase hex digits.
#[must_use]
pub fn knowledge_file_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Is `text` a file id (32 lowercase hex digits)?
#[must_use]
pub fn is_file_id(text: &str) -> bool {
    text.len() == 32 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod path_tests;
