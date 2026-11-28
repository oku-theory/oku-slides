//! Data ops in the session: preparing a batch before it applies, and their undo labels.

use oku_slides_chart::ChartLabels;
use oku_slides_core::{Deck, Op, OpError, Origin, PrepareCtx, Prepared, WidgetRegistry, prepare_ops};
use oku_slides_i18n::Localizer;
use snafu::ResultExt;

use crate::Result;
use crate::error::PrepareSnafu;

/// `ops` ready to apply to `deck`: an external batch checked, every id it leaves out drawn,
/// inline view data lifted with its warnings.
///
/// # Errors
///
/// `Prepare` when the batch is refused before it applies (`op-data-batch-too-large`,
/// `op-data-internal`).
pub(crate) fn prepared(
    deck: &Deck,
    registry: &WidgetRegistry,
    labels: &ChartLabels,
    ops: Vec<Op>,
    origin: Origin,
) -> Result<Prepared> {
    Ok(prepare(deck, registry, labels, ops, origin).context(PrepareSnafu)?)
}

/// `prepared` with the core's error, for the import's own error type.
pub(crate) fn prepare(
    deck: &Deck,
    registry: &WidgetRegistry,
    labels: &ChartLabels,
    ops: Vec<Op>,
    origin: Origin,
) -> std::result::Result<Prepared, OpError> {
    let mut ids = oku_slides_data::new_id;
    let mut ctx = PrepareCtx { deck, registry, labels, ids: &mut ids, origin };
    prepare_ops(ops, &mut ctx)
}

/// The undo label of a batch made only of data ops of one kind, `None` for any other batch.
#[must_use]
pub fn data_undo_label(l: &Localizer, ops: &[Op]) -> Option<String> {
    let first = data_key(ops.first()?)?;
    ops.iter().all(|op| data_key(op) == Some(first)).then(|| l.tr(first))
}

/// The `session-undo-data-*` key of a data op.
fn data_key(op: &Op) -> Option<&'static str> {
    match op {
        Op::AddData { .. } => Some("session-undo-data-add"),
        Op::EditData { .. } => Some("session-undo-data-edit"),
        Op::BindData { .. } => Some("session-undo-data-bind"),
        Op::RemoveData { .. } => Some("session-undo-data-remove"),
        Op::RenameData { .. } => Some("session-undo-data-rename"),
        Op::SetQuery { .. } => Some("session-undo-data-query"),
        Op::SetColumnType { .. } => Some("session-undo-data-type"),
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
        | Op::KnowledgeWrite { .. }
        | Op::KnowledgeEdit { .. }
        | Op::KnowledgeMove { .. }
        | Op::KnowledgeDelete { .. }
        // Made by `prepare_ops` after the label is chosen.
        | Op::PutViewData { .. } => None,
    }
}

#[cfg(test)]
#[path = "data_ops_tests.rs"]
mod data_ops_tests;
