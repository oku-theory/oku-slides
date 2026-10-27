//! What each request does: changes the deck, edits, ends the widget session, writes the draft.
//!
//! Classifiers of [`Request`], read by `Session::handle` before it answers.

use super::Request;

impl Request {
    /// True if the request may change the deck (slides added or removed).
    ///
    /// `Save` too: a file changed on disk is merged first. History (except
    /// the version list): the deck shown changes. `SavePack` and `SaveTheme`
    /// too: the library changes, the packs panel rereads it (`prune`).
    /// Exhaustive on purpose: a new request must say whether it changes the deck.
    #[must_use]
    pub fn changes_deck(&self) -> bool {
        match self {
            Self::Save { .. }
            | Self::Apply { .. }
            | Self::SetTheme { .. }
            | Self::EditTheme { .. }
            | Self::Undo
            | Self::Redo
            | Self::UndoTurn { .. }
            | Self::ChartKind { .. }
            | Self::ShowAs { .. }
            | Self::EditPack { .. }
            | Self::ClosePack
            | Self::SetEditedThemeExtends { .. }
            | Self::EditThemeExtends { .. }
            | Self::SavePack { .. }
            | Self::SaveTheme { .. }
            | Self::Merge { .. }
            | Self::HistoryOpen
            | Self::HistorySeek { .. }
            | Self::HistoryRestore
            | Self::HistoryClose
            | Self::Move { .. }
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. }
            | Self::HostToken { .. }
            | Self::DataEdit { .. }
            | Self::UsePackVersion { .. } => true,
            Self::Snapshot
            | Self::Slide { .. }
            | Self::Search { .. }
            | Self::Svg { .. }
            | Self::Schema
            | Self::Widgets
            | Self::HostSettings { .. }
            | Self::HostTheme { .. }
            | Self::DataPanel { .. }
            | Self::Layouts
            | Self::Packs { .. }
            | Self::RenameEditedTheme { .. }
            | Self::DeletePack { .. }
            | Self::SaveCopy { .. }
            | Self::Ui { .. }
            | Self::HistoryVersions
            | Self::WidgetStatus { .. }
            | Self::WidgetSource { .. }
            | Self::PreviewWidget { .. } => false,
        }
    }

    /// Requêtes refusées pendant l'historique : elles modifieraient le deck,
    /// l'enregistreraient, ou passeraient en mode pack.
    pub(crate) fn edits(&self) -> bool {
        matches!(
            self,
            Self::Apply { .. }
                | Self::SetTheme { .. }
                | Self::EditTheme { .. }
                | Self::Undo
                | Self::Redo
                | Self::UndoTurn { .. }
                | Self::Save { .. }
                | Self::Merge { .. }
                | Self::Move { .. }
                | Self::SaveCopy { .. }
                | Self::ChartKind { .. }
                | Self::ShowAs { .. }
                | Self::EditPack { .. }
                | Self::SavePack { .. }
                | Self::SaveTheme { .. }
                | Self::ClosePack
                | Self::SetEditedThemeExtends { .. }
                | Self::EditThemeExtends { .. }
                | Self::RenameEditedTheme { .. }
                | Self::DeletePack { .. }
                | Self::SetWidget { .. }
                | Self::RemoveWidget { .. }
                | Self::RenameWidget { .. }
                | Self::HostToken { .. }
                | Self::DataEdit { .. }
                | Self::SetExampleValue { .. }
                | Self::EditExampleList { .. }
                | Self::UsePackVersion { .. }
        )
    }

    /// True if the request ends the deck-mode widget session.
    ///
    /// Every request that changes the deck, except a widget edit and `Save`:
    /// a save ends it only when it merges a file changed on disk (see
    /// `Session::absorb`). The interface saves after each write: otherwise
    /// each would make a pack version. `UsePackVersion` ends it itself once
    /// its checks passed: a refused one changes nothing.
    pub(crate) fn ends_widget_session(&self) -> bool {
        self.changes_deck()
            && !matches!(self, Self::Save { .. } | Self::UsePackVersion { .. })
            && !self.is_widget_edit()
    }

    /// Vrai pour une modification de widget (valeurs d'exemple comprises) : elle ne
    /// termine pas la séance des widgets.
    pub(crate) fn is_widget_edit(&self) -> bool {
        matches!(
            self,
            Self::SetWidget { .. }
                | Self::RemoveWidget { .. }
                | Self::RenameWidget { .. }
                | Self::SetExampleValue { .. }
                | Self::EditExampleList { .. }
        )
    }

    /// True for a request that writes the edited pack or theme, or a draft of it.
    ///
    /// Such a request first takes the lease (`Session::ensure_pack_lease` in
    /// pack mode, `Session::ensure_theme_lease` in a theme tab). A copy
    /// (`as_new`) takes its own. Exhaustive on purpose: a new request must
    /// say whether it writes.
    pub(crate) fn writes_edited_draft(&self) -> bool {
        match self {
            Self::Apply { .. }
            | Self::SetTheme { .. }
            | Self::EditTheme { .. }
            | Self::Undo
            | Self::Redo
            | Self::UndoTurn { .. }
            | Self::ChartKind { .. }
            | Self::ShowAs { .. }
            | Self::DataEdit { .. }
            | Self::SavePack { as_new: false }
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. }
            | Self::HostToken { .. }
            | Self::SetEditedThemeExtends { .. }
            | Self::EditThemeExtends { .. }
            | Self::RenameEditedTheme { .. }
            | Self::SaveTheme { as_new: false } => true,
            Self::Snapshot
            | Self::Save { .. }
            | Self::Slide { .. }
            | Self::Search { .. }
            | Self::Svg { .. }
            | Self::Schema
            | Self::Widgets
            | Self::HostSettings { .. }
            | Self::HostTheme { .. }
            | Self::DataPanel { .. }
            | Self::Layouts
            | Self::Packs { .. }
            | Self::EditPack { .. }
            | Self::SavePack { as_new: true }
            | Self::SaveTheme { as_new: true }
            | Self::ClosePack
            | Self::DeletePack { .. }
            | Self::Ui { .. }
            | Self::Merge { .. }
            | Self::HistoryOpen
            | Self::HistorySeek { .. }
            | Self::HistoryRestore
            | Self::HistoryClose
            | Self::HistoryVersions
            | Self::Move { .. }
            | Self::SaveCopy { .. }
            | Self::WidgetStatus { .. }
            | Self::WidgetSource { .. }
            | Self::PreviewWidget { .. }
            | Self::UsePackVersion { .. } => false,
        }
    }

    /// True for a request a theme tab takes: reads, its draft's edits and their undo.
    ///
    /// `Apply` passes here; its ops are then filtered (`Session::ensure_caller_ops`).
    /// Exhaustive on purpose: a new request must say whether a theme tab takes it.
    pub(crate) fn theme_tab_allows(&self) -> bool {
        match self {
            Self::Snapshot
            | Self::Slide { .. }
            | Self::Search { .. }
            | Self::Svg { .. }
            | Self::Schema
            | Self::Widgets
            | Self::HostTheme { .. }
            | Self::HostToken { .. }
            | Self::Ui { .. }
            | Self::Apply { .. }
            | Self::Undo
            | Self::Redo
            | Self::UndoTurn { .. }
            | Self::RenameEditedTheme { .. }
            | Self::SetEditedThemeExtends { .. }
            | Self::EditThemeExtends { .. }
            | Self::SaveTheme { .. } => true,
            Self::Save { .. }
            | Self::SetTheme { .. }
            | Self::EditTheme { .. }
            | Self::ChartKind { .. }
            | Self::ShowAs { .. }
            | Self::EditPack { .. }
            | Self::ClosePack
            | Self::SavePack { .. }
            | Self::Merge { .. }
            | Self::HistoryOpen
            | Self::HistorySeek { .. }
            | Self::HistoryRestore
            | Self::HistoryClose
            | Self::HistoryVersions
            | Self::Move { .. }
            | Self::SaveCopy { .. }
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. }
            | Self::DataEdit { .. }
            | Self::UsePackVersion { .. }
            | Self::HostSettings { .. }
            | Self::DataPanel { .. }
            | Self::Layouts
            | Self::Packs { .. }
            | Self::DeletePack { .. }
            | Self::WidgetStatus { .. }
            | Self::WidgetSource { .. }
            | Self::PreviewWidget { .. } => false,
        }
    }
}

#[cfg(test)]
#[path = "request_class_tests.rs"]
mod request_class_tests;
