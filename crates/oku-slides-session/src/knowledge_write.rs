//! Approve on write: what a caller batch writes to the deck knowledge, the user approved.
//!
//! The session follows each file by id through a batch, from the deck before it. A file whose
//! text the user has not approved is tainted: an edit of it is refused, since its result would
//! be approved with the received text inside, and so is a move, since core re-checks the text at
//! the target and its refusal would quote a frontmatter key of it to the agent. A whole-text
//! write clears the taint, a delete stays allowed. Inside an agent call, each file the batch wrote
//! or edited is stamped with a provenance whose id is random (`oku_slides_core::stamp`), in the
//! same batch and undo step: an agent that writes a received file's text gets another text
//! approved, never the received one. The ids the batch wrote or edited are approved
//! once core applied it and the new deck rendered, just before the replica write; that write can
//! still fail, so a refused apply can leave an approved hash of agent-authored text that was
//! never installed. Nothing else approves: open, merge, receive, undo and redo never do.
//!
//! Approvals are global, for every deck, and keyed by hash only: see `knowledge_approvals`.

use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::{Deck, KnowledgePath, Op, Provenance, WidgetRegistry, asset_id, knowledge_file_id, stamp};
use snafu::{ResultExt, ensure};
use time::OffsetDateTime;

use crate::agent::bounds::label_within;
use crate::error::{
    KnowledgeChangeAwaitingSnafu, KnowledgePathInvalidSnafu, KnowledgeStampRandomSnafu, KnowledgeStampSnafu, OpSnafu,
};
use crate::{Result, Session, SessionError};

/// The longest agent label a stamp carries, in characters.
pub(crate) const AGENT_LABEL_MAX: usize = 80;

/// Who a knowledge stamp names as its author: this app at its version.
pub(crate) fn stamp_by() -> String {
    format!("oku-slides/{}", env!("CARGO_PKG_VERSION"))
}

/// The time and random id of a knowledge stamp.
pub(crate) type StampSource =
    Box<dyn Fn() -> std::result::Result<(OffsetDateTime, [u8; 8]), getrandom::Error> + Send + Sync>;

/// The system clock and random source.
pub(crate) fn system_stamp_source() -> StampSource {
    Box::new(|| {
        let mut id = [0u8; 8];
        getrandom::fill(&mut id)?;
        Ok((OffsetDateTime::now_utc(), id))
    })
}

impl Session {
    /// Names the agent of the open turn in the knowledge stamps; `None` clears it.
    ///
    /// The label is cleaned and cut to 80 characters; `end_turn` clears it too.
    pub fn set_agent_label(&mut self, label: Option<&str>) {
        self.agent_label = label.map(|l| label_within(l, AGENT_LABEL_MAX));
    }

    /// Rewrites, in `next`, each file the batch authored with its text stamped, through core.
    ///
    /// Called inside an agent call only, on the deck the batch produced, so the stamp is part of
    /// the same commit and undo step.
    ///
    /// # Errors
    ///
    /// `KnowledgeStampRandom` when the system has no random bytes, `KnowledgeStamp` when a
    /// frontmatter cannot be stamped, `Op` when core refuses a stamped text (over a limit):
    /// the whole batch is refused then.
    pub(crate) fn stamp_knowledge_writes(
        &self,
        next: &mut Deck,
        authored: &BTreeSet<String>,
        registry: &WidgetRegistry,
    ) -> Result<()> {
        if !authored.iter().any(|id| next.knowledge.contains_key(id)) {
            return Ok(());
        }
        let (at, id) = (self.stamp_source)().context(KnowledgeStampRandomSnafu)?;
        let provenance = Provenance::new(stamp_by(), at, id, self.agent_label.clone());
        let mut ops = Vec::new();
        for (file_id, file) in authored.iter().filter_map(|id| next.knowledge.get(id).map(|f| (id, f))) {
            let path = KnowledgePath::parse(&file.path).map_err(|reason| {
                SessionError::from(KnowledgePathInvalidSnafu { path: file.path.clone(), reason }.build())
            })?;
            let text = stamp(&path, &file.text, &provenance).map_err(|reason| {
                SessionError::from(KnowledgeStampSnafu { path: file.path.clone(), reason }.build())
            })?;
            ops.push(Op::KnowledgeWrite { path: file.path.clone(), text, id: Some(file_id.clone()) });
        }
        oku_slides_core::apply_all_moving(next, ops, registry).context(OpSnafu)?;
        Ok(())
    }

    /// Follows the knowledge files through `ops` and returns the ids the batch authored.
    ///
    /// A write to a free path without an id gets a minted one here, so core applies what was
    /// followed. Lookups are by exact path, as core's; a path core will refuse is not followed,
    /// the whole batch being refused then.
    ///
    /// # Errors
    ///
    /// `KnowledgeChangeAwaiting` for an edit or a move of a file whose text the user has not
    /// approved, at the point of the batch it runs: a later whole write of the same file does not
    /// excuse it.
    pub(crate) fn prepare_knowledge_ops(&self, ops: &mut [Op]) -> Result<BTreeSet<String>> {
        let knowledge = &self.doc.sources.deck().knowledge;
        let mut at: BTreeMap<String, String> =
            knowledge.iter().map(|(id, file)| (file.path.clone(), id.clone())).collect();
        let mut tainted: BTreeSet<String> =
            knowledge.iter().filter(|(_, file)| !self.knowledge_approved(file)).map(|(id, _)| id.clone()).collect();
        let mut authored = BTreeSet::new();
        for op in ops.iter_mut() {
            match op {
                Op::KnowledgeWrite { path, id, .. } => {
                    if KnowledgePath::parse(path).is_err() {
                        continue;
                    }
                    let file = at
                        .entry(path.clone())
                        .or_insert_with(|| id.get_or_insert_with(knowledge_file_id).clone())
                        .clone();
                    tainted.remove(&file);
                    authored.insert(file);
                }
                Op::KnowledgeEdit { path, .. } => {
                    if let Some(file) = at.get(path.as_str()) {
                        ensure!(!tainted.contains(file), KnowledgeChangeAwaitingSnafu { path: path.clone() });
                        authored.insert(file.clone());
                    }
                }
                Op::KnowledgeMove { from, to } => {
                    // Refused whatever the target: core's refusal there could quote the text.
                    if let Some(file) = at.get(from.as_str()) {
                        ensure!(!tainted.contains(file), KnowledgeChangeAwaitingSnafu { path: from.clone() });
                    }
                    if KnowledgePath::parse(to).is_err() || at.contains_key(to.as_str()) {
                        continue;
                    }
                    if let Some(file) = at.remove(from.as_str()) {
                        at.insert(to.clone(), file);
                    }
                }
                Op::KnowledgeDelete { path } => {
                    if let Some(file) = at.remove(path.as_str()) {
                        tainted.remove(&file);
                        authored.remove(&file);
                    }
                }
                Op::SetParam { .. }
                | Op::SetDeckParam { .. }
                | Op::SetKind { .. }
                | Op::SetStyle { .. }
                | Op::ReplaceStyle { .. }
                | Op::SetFrame { .. }
                | Op::SetCell { .. }
                | Op::SetGrid { .. }
                | Op::ReplaceElement { .. }
                | Op::SetChartOption { .. }
                | Op::SetTableOption { .. }
                | Op::EditList { .. }
                | Op::AddElement { .. }
                | Op::RemoveElement { .. }
                | Op::MoveElement { .. }
                | Op::AddSlide { .. }
                | Op::RemoveSlide { .. }
                | Op::MoveSlide { .. }
                | Op::SetNotes { .. }
                | Op::SetTheme { .. }
                | Op::AddPack { .. }
                | Op::RemovePack { .. }
                | Op::SetToken { .. }
                | Op::UnsetToken { .. }
                | Op::SetTokens { .. }
                | Op::SetVar { .. }
                | Op::AddData { .. }
                | Op::EditData { .. }
                | Op::BindData { .. }
                | Op::RemoveData { .. }
                | Op::RenameData { .. }
                | Op::SetColumnType { .. }
                | Op::PutViewData { .. } => {}
            }
        }
        Ok(authored)
    }

    /// Approves the text, in `next`, of every id the batch authored, in one write.
    ///
    /// # Errors
    ///
    /// `KnowledgeApprovalsWrite` (or `KnowledgeApprovalsDamaged`) when the approvals file cannot
    /// be read or written: the apply is refused then, before the replica write.
    pub(crate) fn approve_knowledge_writes(&self, next: &Deck, authored: &BTreeSet<String>) -> Result<()> {
        let hashes: Vec<String> = authored
            .iter()
            .filter_map(|id| next.knowledge.get(id))
            .map(|file| asset_id(file.text.as_bytes()))
            .collect();
        if hashes.is_empty() {
            return Ok(());
        }
        self.library.knowledge_approvals().approve(&hashes)
    }
}

#[cfg(test)]
#[path = "knowledge_write_tests.rs"]
mod knowledge_write_tests;
