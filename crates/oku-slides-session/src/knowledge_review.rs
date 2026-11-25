//! The user's review of the deck knowledge base: what awaits, a file's text, its approval.
//!
//! A file is approved by the hash of the text the user was shown (`KnowledgeText`): a text
//! changed since (merge, undo, receive) is refused, never approved unseen. The review and the
//! snapshot's [`KnowledgeSummary`] share one scan, cached on the bundle's generation and the
//! approvals count, so a slide edit does not hash the bundle again.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ensure};

use oku_slides_core::knowledge::limits::{MAX_FILE_BYTES, MAX_FILES};
use oku_slides_core::{KnowledgeFile, asset_id, frontmatter, knowledge_over_limits};

use crate::agent::bounds::{label, label_within};
use crate::error::{
    InHistorySnafu, KnowledgeApprovalInvalidSnafu, KnowledgeChangedSnafu, KnowledgeIdNotFoundSnafu,
    KnowledgeTooLargeToApproveSnafu, WrongDocumentSnafu,
};
use crate::knowledge_view::cut;
use crate::{DocumentKind, Mode, Result, Session};

/// Most rows of a review: the rest are counted in `more`.
const REVIEW_ROWS: usize = 200;
/// Most characters of a listed title.
const TITLE_MAX: usize = 80;

/// The knowledge files awaiting the user's approval, by path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeReview {
    /// The first awaiting files, by path.
    pub files: Vec<AwaitingKnowledge>,
    /// Awaiting files left out past the row limit.
    pub more: usize,
    /// The ceilings the bundle passes (`too-many-files`, `file-too-large`, `total-too-large`).
    pub over_limit: Vec<&'static str>,
}

/// One awaiting knowledge file, as the review lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AwaitingKnowledge {
    /// The file's id.
    pub id: String,
    /// Its path, cleaned for display (`label`).
    pub path: String,
    /// Its frontmatter `title`, cleaned and cut at 80 characters.
    pub title: Option<String>,
    /// Its size in bytes.
    pub bytes: usize,
}

/// A knowledge file's text as the user reviews it, with the hash an approval names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeText {
    /// The file's id.
    pub id: String,
    /// Its path, cleaned for display (`label`).
    pub path: String,
    /// The hash of the whole text (`sha256:<hex>`).
    pub hash: String,
    /// The text, at most `MAX_FILE_BYTES` bytes, cut on a character boundary.
    pub text: String,
    /// Bytes of the file left out of `text`.
    pub truncated: usize,
}

/// One file the user approves: its id and the hash of the text they were shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeApproval {
    /// The file's id.
    pub id: String,
    /// The hash `KnowledgeText` answered.
    pub hash: String,
}

/// What the snapshot says of the knowledge base: what awaits and the ceilings passed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeSummary {
    /// Files awaiting the user's approval.
    pub awaiting: usize,
    /// The ceilings the bundle passes, as in [`KnowledgeReview`].
    pub over_limit: Vec<&'static str>,
}

/// The cache key: the bundle's generation and the approvals count.
type SummaryKey = (u64, usize);

/// One hashing pass over the bundle: the awaiting files by path, and the ceilings passed.
#[derive(Debug)]
struct KnowledgeScan {
    awaiting: Vec<AwaitingKnowledge>,
    over_limit: Vec<&'static str>,
}

/// The scan last made, with the key it was made for; the review and the summary share it.
#[derive(Debug, Default)]
pub(crate) struct KnowledgeSummaryCache {
    last: Mutex<Option<(SummaryKey, Arc<KnowledgeScan>)>>,
    /// Scans made, for tests.
    #[cfg(test)]
    computed: std::sync::atomic::AtomicUsize,
}

impl KnowledgeSummaryCache {
    /// The scan for `key`: the cached one, or `compute`'s, kept; not locked while computing.
    fn scan(&self, key: SummaryKey, compute: impl FnOnce() -> KnowledgeScan) -> Arc<KnowledgeScan> {
        {
            let last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((_, scan)) = last.as_ref().filter(|(k, _)| *k == key) {
                return Arc::clone(scan);
            }
        }
        let scan = Arc::new(compute());
        #[cfg(test)]
        self.computed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = Some((key, Arc::clone(&scan)));
        scan
    }

    /// How many scans were made.
    #[cfg(test)]
    pub(crate) fn computations(&self) -> usize {
        self.computed.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The ceilings `files` passes, by slug.
fn over_limit(files: &std::collections::BTreeMap<String, KnowledgeFile>) -> Vec<&'static str> {
    knowledge_over_limits(files).into_iter().map(oku_slides_core::KnowledgeLimit::slug).collect()
}

impl Session {
    /// The awaiting knowledge files, by path, at most 200; the ceilings the bundle passes.
    ///
    /// # Errors
    ///
    /// `InHistory` while history is open: the version shown is not the deck's.
    pub(crate) fn knowledge_review(&self) -> Result<KnowledgeReview> {
        self.ensure_knowledge_review()?;
        let scan = self.knowledge_scan();
        let more = scan.awaiting.len().saturating_sub(REVIEW_ROWS);
        let files = scan.awaiting.iter().take(REVIEW_ROWS).cloned().collect();
        Ok(KnowledgeReview { files, more, over_limit: scan.over_limit.clone() })
    }

    /// The bundle's scan, cached on its generation and the approvals count.
    ///
    /// The count is read before the bundle is hashed: an approval made meanwhile only makes
    /// the key stale, and the next call scans again.
    fn knowledge_scan(&self) -> Arc<KnowledgeScan> {
        let key = (self.doc.sources.knowledge_generation(), self.library.knowledge_approvals().count());
        self.knowledge_summary.scan(key, || {
            let knowledge = &self.doc.sources.deck().knowledge;
            let mut awaiting: Vec<(&String, &KnowledgeFile)> =
                knowledge.iter().filter(|(_, f)| !self.knowledge_approved(f)).collect();
            awaiting.sort_by(|a, b| a.1.path.cmp(&b.1.path));
            let awaiting = awaiting
                .into_iter()
                .map(|(id, f)| AwaitingKnowledge {
                    id: id.clone(),
                    path: label(&f.path),
                    title: frontmatter(&f.text).ok().and_then(|m| m.title).map(|t| label_within(&t, TITLE_MAX)),
                    bytes: f.text.len(),
                })
                .collect();
            KnowledgeScan { awaiting, over_limit: over_limit(knowledge) }
        })
    }

    /// The text of the knowledge file `id`, approved or not, cut at `MAX_FILE_BYTES`.
    ///
    /// # Errors
    ///
    /// `InHistory` while history is open, `KnowledgeIdNotFound` when no file has that id.
    pub(crate) fn knowledge_text(&self, id: &str) -> Result<KnowledgeText> {
        self.ensure_knowledge_review()?;
        let file = self.doc.sources.deck().knowledge.get(id).context(KnowledgeIdNotFoundSnafu { id })?;
        let text = cut(&file.text, MAX_FILE_BYTES);
        Ok(KnowledgeText {
            id: id.to_owned(),
            path: label(&file.path),
            hash: asset_id(file.text.as_bytes()),
            text: text.to_owned(),
            truncated: file.text.len() - text.len(),
        })
    }

    /// Approves `files`, each for the text whose hash it names; all or none.
    ///
    /// An already approved file (even one past `MAX_FILE_BYTES`) or an empty list changes
    /// nothing.
    ///
    /// # Errors
    ///
    /// `InHistory` while history is open; `KnowledgeApprovalInvalid` for an id named twice or
    /// more than `MAX_FILES` files; `KnowledgeIdNotFound`, `KnowledgeChanged` (the text is not
    /// the one hashed) or `KnowledgeTooLargeToApprove` for a file; `KnowledgeApprovalsDamaged`
    /// or `KnowledgeApprovalsWrite`.
    pub(crate) fn approve_knowledge(&self, files: &[KnowledgeApproval]) -> Result<()> {
        self.ensure_knowledge_review()?;
        ensure!(files.len() <= MAX_FILES, KnowledgeApprovalInvalidSnafu);
        let ids: BTreeSet<&str> = files.iter().map(|f| f.id.as_str()).collect();
        ensure!(ids.len() == files.len(), KnowledgeApprovalInvalidSnafu);
        let knowledge = &self.doc.sources.deck().knowledge;
        let approvals = self.library.knowledge_approvals();
        let mut hashes = Vec::new();
        for approval in files {
            let id = approval.id.as_str();
            let file = knowledge.get(id).context(KnowledgeIdNotFoundSnafu { id })?;
            let hash = asset_id(file.text.as_bytes());
            ensure!(hash == approval.hash, KnowledgeChangedSnafu { path: &file.path });
            if approvals.is_approved(&hash) {
                continue;
            }
            ensure!(file.text.len() <= MAX_FILE_BYTES, KnowledgeTooLargeToApproveSnafu { path: &file.path });
            hashes.push(hash);
        }
        if hashes.is_empty() {
            return Ok(());
        }
        approvals.approve(&hashes)
    }

    /// The snapshot's knowledge summary; `None` outside a deck or when nothing is to say.
    ///
    /// From the cached scan the review shares; the cache is not locked while the bundle is
    /// hashed.
    pub(crate) fn knowledge_summary(&self) -> Option<KnowledgeSummary> {
        if !matches!(self.mode, Mode::Deck) {
            return None;
        }
        let scan = self.knowledge_scan();
        let (awaiting, over_limit) = (scan.awaiting.len(), scan.over_limit.clone());
        (awaiting > 0 || !over_limit.is_empty()).then_some(KnowledgeSummary { awaiting, over_limit })
    }

    /// Refuses a review outside a deck tab, whatever the caller: `InHistory` while history
    /// shows an old version, `WrongDocument` in a help, pack or theme tab.
    fn ensure_knowledge_review(&self) -> Result<()> {
        ensure!(!matches!(self.mode, Mode::History { .. }), InHistorySnafu);
        let kind = self.document_kind();
        ensure!(kind == DocumentKind::Deck, WrongDocumentSnafu { kind });
        Ok(())
    }
}

#[cfg(test)]
#[path = "knowledge_review_tests.rs"]
mod knowledge_review_tests;
