//! The import of an attached document into the deck knowledge base (`knowledge_import`, t-1759).
//!
//! Three steps, the session lock held only for the first and the last:
//! - [`Session::knowledge_import_job`] snapshots the attachment, the tab and the bounds: the
//!   text gets the bundle's room (the import it replaces counted free) less what the import adds
//!   to it ([`import_overhead`]);
//! - [`ImportJob::run`] extracts the document in a child worker built for it, one import at a
//!   time in the process (another is refused, not queued), with no session lock held;
//! - [`Session::knowledge_import_apply`] rechecks the tab, the attachment and the import it
//!   replaces, stages the images (`import_staged`, cleared by a guard on every return or unwind)
//!   and writes the reference files through the agent apply path: stamped, approved, undoable
//!   with the turn.
//!
//! An import replaces only the files its frontmatter mark ([`import_mark`]) says are parts of
//! the same earlier import, and none edited since it wrote them.

use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, TryLockError};

use oku_slides_core::knowledge::Extracted;
use oku_slides_core::knowledge::limits::MAX_TOTAL_BYTES;
use oku_slides_core::knowledge::{DocumentType, ExtractLimits, ExtractRefusal, KnowledgePath};
use oku_slides_core::num::count_u64;
use oku_slides_core::{KnowledgeFile, Op, asset_id};
use oku_slides_import::ReadLimits;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::{KnowledgeSettings, RenderSettings};
use oku_slides_render::RenderError;
use oku_slides_render_worker::RenderWorker;
use snafu::{OptionExt, ResultExt, ensure};

use crate::agent::AttachmentRef;
use crate::agent::bounds::{has_visible_text, label_within};
use crate::error::{
    AttachmentGoneSnafu, ImportPathInvalidSnafu, ImportTabChangedSnafu, ImportTargetEditedSnafu,
    ImportTargetNotImportSnafu, InHistorySnafu, KnowledgeImportBusySnafu, KnowledgeImportFailedSnafu,
    KnowledgeImportRefusedSnafu, KnowledgeStampRandomSnafu, NotADocumentSnafu, WrongDocumentSnafu,
};
use crate::knowledge_import_files::{
    MAX_IMAGES, PART_BYTES, REFERENCES, Reference, TITLE_MAX, image_link, import_mark, import_overhead, is_unedited,
    part_paths, reference_files, replace_placeholders, slug, split_parts,
};
use crate::{Mode, Result, Session};

/// Most bytes of all images one import keeps; the sources budget still applies.
const MAX_IMAGE_BYTES: u64 = 128 << 20;
/// The worker's memory divided by this bounds the bytes of all images one import keeps: they
/// are held in the worker, then copied into its reply.
const IMAGE_MEMORY_SHARE: u64 = 6;

/// One import at a time in the process: each runs its own worker, under its own bounds; another
/// is refused while one runs, so no thread waits on it. Every session shares this slot (tests
/// give theirs one of its own, so they never refuse each other).
pub(crate) static IMPORTS: LazyLock<Arc<Mutex<()>>> = LazyLock::new(Arc::default);

/// What extracts a document: a child worker in the app, a fake in tests.
pub trait ExtractRunner: Send + Sync + fmt::Debug {
    /// Extracts `bytes` of kind `kind` within `limits` and `read`; `token` names its image
    /// placeholders.
    ///
    /// # Errors
    ///
    /// The worker's failure (start, transport, time, memory); the inner result is the
    /// document's refusal.
    fn extract(
        &self,
        bytes: &[u8],
        kind: DocumentType,
        limits: &ExtractLimits,
        read: &ReadLimits,
        token: &str,
    ) -> std::result::Result<std::result::Result<Extracted, ExtractRefusal>, RenderError>;
}

/// The production runner: a render worker built for each extraction, under `[knowledge]`
/// time and memory bounds, dropped with it.
#[derive(Debug)]
pub(crate) struct ChildExtractRunner {
    launch: RenderLaunch,
    settings: RenderSettings,
}

impl ChildExtractRunner {
    /// A runner launching like `launch`, with `render`'s frame bounds and `knowledge`'s
    /// extraction time and memory.
    pub(crate) fn new(launch: RenderLaunch, render: &RenderSettings, knowledge: &KnowledgeSettings) -> Self {
        let settings = RenderSettings {
            timeout_ms: knowledge.effective_extract_timeout_ms(),
            memory_mib: knowledge.effective_extract_memory_mib(),
            ..render.clone()
        };
        Self { launch, settings }
    }
}

impl ExtractRunner for ChildExtractRunner {
    fn extract(
        &self,
        bytes: &[u8],
        kind: DocumentType,
        limits: &ExtractLimits,
        read: &ReadLimits,
        token: &str,
    ) -> std::result::Result<std::result::Result<Extracted, ExtractRefusal>, RenderError> {
        RenderWorker::new(self.launch.clone(), self.settings.clone()).extract_document(bytes, kind, limits, read, token)
    }
}

/// What an import's apply rechecks after the extraction: the attachment and the tab it came from.
#[derive(Debug)]
pub struct ImportTicket {
    attachment: String,
    name: String,
    kind: DocumentType,
    deck_uid: String,
    doc_path: Option<PathBuf>,
    /// Names the import's image placeholders and, as its series id, its parts.
    token: String,
    /// The first part of the earlier import to replace; `None`: a new reference.
    path: Option<String>,
}

/// An import ready to extract, without the session: run it, then apply its ticket.
pub struct ImportJob {
    ticket: ImportTicket,
    bytes: Arc<[u8]>,
    limits: ExtractLimits,
    read: ReadLimits,
    runner: Arc<dyn ExtractRunner>,
    /// The import slot ([`IMPORTS`]) held while it runs.
    slot: Arc<Mutex<()>>,
}

impl fmt::Debug for ImportJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportJob")
            .field("ticket", &self.ticket)
            .field("bytes", &self.bytes.len())
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl ImportJob {
    /// Extracts the document, blocking, unless another import is running.
    ///
    /// # Errors
    ///
    /// `KnowledgeImportBusy` while another import runs in the process, `KnowledgeImportFailed`
    /// when the worker fails, `KnowledgeImportRefused` when the document is refused.
    pub fn run(&self) -> Result<Extracted> {
        let _one = match self.slot.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => {
                tracing::info!(attachment = %self.ticket.attachment, "knowledge import refused: another one runs");
                return KnowledgeImportBusySnafu.fail()?;
            }
        };
        let t = &self.ticket;
        let extracted = self
            .runner
            .extract(&self.bytes, t.kind, &self.limits, &self.read, &t.token)
            .inspect_err(|e| tracing::info!(attachment = %t.attachment, error = %e, "knowledge import failed"))
            .context(KnowledgeImportFailedSnafu)?;
        let refused = |refusal: ExtractRefusal| {
            tracing::info!(attachment = %t.attachment, ?refusal, "knowledge import refused");
            KnowledgeImportRefusedSnafu { refusal }.build()
        };
        Ok(extracted.map_err(refused)?)
    }

    /// What the apply rechecks.
    #[must_use]
    pub fn ticket(&self) -> &ImportTicket {
        &self.ticket
    }
}

/// What an import wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportOutcome {
    /// The reference files, in part order.
    pub paths: Vec<String>,
    /// The images linked from them.
    pub images: usize,
    /// Something was left out of the document.
    pub cut: bool,
}

impl Session {
    /// The import of the attached document `attachment`, to run without the session lock: at
    /// `path`, replacing the earlier import there, or under a new name.
    ///
    /// # Errors
    ///
    /// `InHistory` or `WrongDocument` outside a deck tab, `AttachmentGone` when the tab no
    /// longer keeps it, `NotADocument` for an image, those of [`Self::earlier_import`] for
    /// `path`, `KnowledgeStampRandom` without random bytes.
    pub fn knowledge_import_job(&self, attachment: &str, path: Option<&str>) -> Result<ImportJob> {
        ensure!(!matches!(self.mode, Mode::History { .. }), InHistorySnafu);
        ensure!(matches!(self.mode, Mode::Deck), WrongDocumentSnafu { kind: self.document_kind() });
        let (r, bytes) = self.attachments.get(attachment).context(AttachmentGoneSnafu { id: attachment })?;
        let AttachmentRef::Document { name, kind, .. } = r else {
            return NotADocumentSnafu { id: attachment }.fail()?;
        };
        let replaced = match path {
            Some(target) => self.earlier_import(target)?.bytes,
            None => 0,
        };
        let used: usize = self.deck().knowledge.values().map(|f| f.text.len()).sum::<usize>() - replaced;
        let room = MAX_TOTAL_BYTES.saturating_sub(used);
        let paste = self.attachments.limits();
        let memory = self.knowledge_settings.effective_extract_memory_mib().saturating_mul(1 << 20);
        let limits = ExtractLimits {
            max_text_bytes: room.saturating_sub(import_overhead(&name, kind, room)),
            sheet_rows: self.knowledge_settings.effective_sheet_rows(),
            sheet_columns: self.knowledge_settings.effective_sheet_columns(),
            max_images: MAX_IMAGES,
            max_image_pixels: paste.max_image_pixels,
            max_image_bytes: MAX_IMAGE_BYTES.min(memory / IMAGE_MEMORY_SHARE),
        };
        let read = ReadLimits::for_extract(count_u64(paste.max_document_bytes()), memory);
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).context(KnowledgeStampRandomSnafu)?;
        let ticket = ImportTicket {
            attachment: attachment.to_owned(),
            name,
            kind,
            deck_uid: self.doc.sources.replica().deck_uid(),
            doc_path: self.doc.path.clone(),
            token: hex::encode(random),
            path: path.map(str::to_owned),
        };
        Ok(ImportJob {
            ticket,
            bytes,
            limits,
            read,
            runner: self.extract_runner.clone(),
            slot: self.import_slot.clone(),
        })
    }

    /// Writes `extracted`, the extraction of `ticket`'s document, as reference files: at the
    /// ticket's path (replacing the earlier import there) or under a free `/references/<slug>` name.
    ///
    /// The writes go through [`Session::apply_by_agent`] under the undo `label`: stamped,
    /// approved, in the turn. Nothing is written on a refusal; the staged images are dropped
    /// whatever the outcome.
    ///
    /// # Errors
    ///
    /// `ImportTabChanged` when the tab no longer shows the same document, `AttachmentGone`,
    /// those of [`Self::earlier_import`] for the path, and those of the apply (budgets of the
    /// knowledge base and the sources).
    pub fn knowledge_import_apply(
        &mut self,
        label: &str,
        ticket: &ImportTicket,
        extracted: Extracted,
    ) -> Result<ImportOutcome> {
        let staging = Unstage(self);
        let outcome = staging.0.import_staged_apply(label, ticket, extracted);
        match &outcome {
            Ok(o) => tracing::info!(
                attachment = %ticket.attachment, parts = o.paths.len(), images = o.images, cut = o.cut,
                "knowledge import"
            ),
            Err(e) => tracing::info!(attachment = %ticket.attachment, error = %e, "knowledge import not written"),
        }
        outcome
    }

    /// [`Self::knowledge_import_apply`], the staging left to its caller to clear.
    fn import_staged_apply(
        &mut self,
        label: &str,
        ticket: &ImportTicket,
        extracted: Extracted,
    ) -> Result<ImportOutcome> {
        let same = matches!(self.mode, Mode::Deck)
            && self.doc.sources.replica().deck_uid() == ticket.deck_uid
            && self.doc.path == ticket.doc_path;
        ensure!(same, ImportTabChangedSnafu);
        ensure!(
            self.attachments.get(&ticket.attachment).is_some(),
            AttachmentGoneSnafu { id: ticket.attachment.as_str() }
        );
        let Extracted { title, markdown, images, cuts } = extracted;
        let mut links = Vec::with_capacity(images.len());
        for image in images {
            let id = asset_id(&image.bytes);
            let link = image_link(&id, &image.mime);
            if link.is_some() {
                self.import_staged.insert(id, image.bytes);
            }
            links.push(link);
        }
        let (body, images) = replace_placeholders(&markdown, &ticket.token, &links);
        let parts = split_parts(&body, PART_BYTES);
        let (base, mut ops) = match ticket.path.as_deref() {
            Some(target) => self.import_target(target, parts.len())?,
            None => (self.free_import_base(&slug(&ticket.name), parts.len()), Vec::new()),
        };
        let title =
            title.filter(|t| has_visible_text(t)).map_or_else(|| ticket.name.clone(), |t| label_within(&t, TITLE_MAX));
        let reference = Reference {
            name: &ticket.name,
            title: &title,
            kind: ticket.kind,
            cut: !cuts.is_empty(),
            series: &ticket.token,
        };
        let paths = part_paths(&base, parts.len());
        let files = reference_files(&reference, &paths, &parts);
        ops.extend(files.into_iter().map(|(path, text)| Op::KnowledgeWrite { path, text, id: None }));
        self.apply_by_agent(label, ops, None)?;
        Ok(ImportOutcome { paths, images, cut: reference.cut })
    }

    /// The base (path without `.md`) of an import replacing the earlier one at `target`, in
    /// `count` parts, and the deletes of that import's parts past them.
    ///
    /// A later path is taken over only when free or a part of the earlier import.
    fn import_target(&self, target: &str, count: usize) -> Result<(String, Vec<Op>)> {
        let earlier = self.earlier_import(target)?;
        let paths = part_paths(&earlier.base, count);
        for part in paths.iter().skip(1) {
            ensure!(
                self.knowledge_at(part).is_none() || earlier.paths.contains(part),
                ImportTargetNotImportSnafu { path: part.as_str() }
            );
        }
        let stale = earlier.paths.into_iter().filter(|p| !paths.contains(p));
        Ok((earlier.base, stale.map(|path| Op::KnowledgeDelete { path }).collect()))
    }

    /// The earlier import whose first part is at `target`: the files its mark says are its
    /// parts, by series and part number at `<base>-part-N.md`.
    ///
    /// # Errors
    ///
    /// `ImportPathInvalid` for a path outside `/references/`, `index.md`, `log.md` or a later
    /// part; `ImportTargetNotImport` for a file without an import's mark; `ImportTargetEdited`
    /// when a part (body or frontmatter) is no longer the one its import wrote.
    fn earlier_import(&self, target: &str) -> Result<EarlierImport> {
        let path = KnowledgePath::parse_writable(target).ok().filter(|p| {
            p.as_str().starts_with(REFERENCES) && !p.as_str().starts_with(oku_slides_core::knowledge::ASSETS_DIR)
        });
        let path = path.filter(|p| !p.is_index() && !p.is_log()).context(ImportPathInvalidSnafu { path: target })?;
        let first = self.knowledge_at(path.as_str()).context(ImportTargetNotImportSnafu { path: target })?;
        let mark = import_mark(&first.text).context(ImportTargetNotImportSnafu { path: target })?;
        ensure!(mark.part == 1, ImportPathInvalidSnafu { path: target });
        let base = path.as_str().strip_suffix(".md").unwrap_or(path.as_str()).to_owned();
        let prefix = format!("{base}-part-");
        let part_of = |f: &KnowledgeFile| {
            let digits = f.path.strip_prefix(&prefix)?.strip_suffix(".md")?;
            let n = digits.parse::<usize>().ok().filter(|n| *n >= 2 && n.to_string() == digits)?;
            import_mark(&f.text).filter(|m| m.part == n && m.series == mark.series)
        };
        let mut parts = vec![(first, mark.clone())];
        parts.extend(self.deck().knowledge.values().filter_map(|f| part_of(f).map(|m| (f, m))));
        let mut earlier = EarlierImport { base, paths: Vec::with_capacity(parts.len()), bytes: 0 };
        for (file, mark) in parts {
            ensure!(is_unedited(&file.text, &mark), ImportTargetEditedSnafu { path: file.path.as_str() });
            earlier.paths.push(file.path.clone());
            earlier.bytes += file.text.len();
        }
        Ok(earlier)
    }

    /// The first base `/references/<slug>`, `<slug>-2`… whose `count` part paths are all free.
    fn free_import_base(&self, slug: &str, count: usize) -> String {
        let candidate =
            |n: usize| if n == 1 { format!("{REFERENCES}{slug}") } else { format!("{REFERENCES}{slug}-{n}") };
        let taken: HashSet<&str> = self.deck().knowledge.values().map(|f| f.path.as_str()).collect();
        let free = |base: &str| part_paths(base, count).iter().all(|p| !taken.contains(p.as_str()));
        // The loop ends: no path is a part of two candidates, so each candidate taken needs a
        // file of its own, and the bundle holds finitely many. No candidate is skipped or made up.
        let mut n = 1;
        while !free(&candidate(n)) {
            n += 1;
        }
        candidate(n)
    }

    /// The knowledge file at `path` in the deck shown.
    fn knowledge_at(&self, path: &str) -> Option<&KnowledgeFile> {
        self.deck().knowledge.values().find(|f| f.path == path)
    }

    /// Replaces the extraction runner with a fake.
    #[cfg(test)]
    pub(crate) fn set_extract_runner(&mut self, runner: Arc<dyn ExtractRunner>) {
        self.extract_runner = runner;
    }

    /// Gives the session an import slot of its own, so that tests running at once in one
    /// process never refuse each other's imports.
    #[cfg(test)]
    pub(crate) fn own_import_slot(&mut self) {
        self.import_slot = Arc::default();
    }
}

/// The files of an earlier import a new one replaces.
#[derive(Debug)]
struct EarlierImport {
    /// Its first part's path without `.md`.
    base: String,
    /// Its parts' paths, the first one first.
    paths: Vec<String>,
    /// Their bytes, which the new import may use again.
    bytes: usize,
}

/// Clears the images an import staged when dropped: on every return of the apply, and on an
/// unwind through it.
struct Unstage<'a>(&'a mut Session);

impl Drop for Unstage<'_> {
    fn drop(&mut self) {
        self.0.import_staged.clear();
    }
}

#[cfg(test)]
#[path = "knowledge_import_tests.rs"]
mod knowledge_import_tests;

#[cfg(test)]
#[path = "knowledge_import_series_tests.rs"]
mod knowledge_import_series_tests;
