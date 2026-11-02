//! What each request does: changes the deck, edits, ends the widget session, writes the draft.
//!
//! Classifiers of [`Request`], read by `Session::handle` before it answers.

use super::Request;

impl Request {
    /// True for a request only a pack tab takes (`Session::ensure_fits`).
    pub(crate) fn pack_only(&self) -> bool {
        matches!(self, Self::SetPreviewTheme { .. })
    }

    /// True if the request may change the deck (slides added or removed).
    ///
    /// `Save` too: a file changed on disk is merged first. History (except
    /// the version list): the deck shown changes. `SavePack` and `SaveTheme`
    /// too: the library changes, the packs panel rereads it (`prune`), as with
    /// `AcceptReceived`. A theme
    /// tab's draft steps too (extends, rename): the specimen deck takes a step.
    /// Exhaustive on purpose: a new request must say whether it changes the deck.
    /// `SetPreviewTheme` does not: the slides are unchanged (`apply_silent_ops` persists).
    #[must_use]
    pub fn changes_deck(&self) -> bool {
        match self {
            Self::Save { .. }
            | Self::Apply { .. }
            | Self::EditTheme { .. }
            | Self::Undo
            | Self::Redo
            | Self::UndoTurn { .. }
            | Self::ChartKind { .. }
            | Self::ShowAs { .. }
            | Self::SetEditedThemeExtends { .. }
            | Self::EditThemeExtends { .. }
            | Self::RenameEditedTheme { .. }
            | Self::SavePack { .. }
            | Self::SaveTheme { .. }
            | Self::Merge { .. }
            | Self::HistoryOpen
            | Self::HistorySeek { .. }
            | Self::HistoryRestore
            | Self::HistoryClose
            | Self::Move { .. }
            | Self::SetGuide { .. }
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::DuplicateWidget { .. }
            | Self::ReorderWidgets { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. }
            | Self::HostToken { .. }
            | Self::DataEdit { .. }
            | Self::ElementParam { .. }
            | Self::ElementList { .. }
            | Self::UsePackVersion { .. }
            | Self::UsePackCopy { .. }
            | Self::UseThemeVersion { .. }
            | Self::AcceptReceived { .. } => true,
            Self::Snapshot
            | Self::Slide { .. }
            | Self::Search { .. }
            | Self::Svg { .. }
            | Self::Schema
            | Self::Widgets
            | Self::HostSettings { .. }
            | Self::HostTheme { .. }
            | Self::DataPanel { .. }
            | Self::ElementPanel { .. }
            | Self::Layouts
            | Self::Packs { .. }
            | Self::DeletePack { .. }
            | Self::SaveCopy { .. }
            | Self::Ui { .. }
            | Self::HistoryVersions
            | Self::WidgetStatus { .. }
            | Self::WidgetSource { .. }
            | Self::SetPreviewTheme { .. }
            | Self::ApproveGuides { .. }
            | Self::LaterReceived { .. }
            | Self::PreviewWidget { .. } => false,
        }
    }

    /// Requests refused while history is open: they would change or save the deck.
    pub(crate) fn edits(&self) -> bool {
        matches!(
            self,
            Self::Apply { .. }
                | Self::SetPreviewTheme { .. }
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
                | Self::SavePack { .. }
                | Self::SaveTheme { .. }
                | Self::SetEditedThemeExtends { .. }
                | Self::EditThemeExtends { .. }
                | Self::RenameEditedTheme { .. }
                | Self::DeletePack { .. }
                | Self::SetGuide { .. }
                | Self::SetWidget { .. }
                | Self::RemoveWidget { .. }
                | Self::RenameWidget { .. }
                | Self::DuplicateWidget { .. }
                | Self::ReorderWidgets { .. }
                | Self::HostToken { .. }
                | Self::DataEdit { .. }
                | Self::ElementParam { .. }
                | Self::ElementList { .. }
                | Self::SetExampleValue { .. }
                | Self::EditExampleList { .. }
                | Self::UsePackVersion { .. }
                | Self::UsePackCopy { .. }
                | Self::UseThemeVersion { .. }
        )
    }

    /// True if the request ends the deck-mode widget session.
    ///
    /// Every request that changes the deck, except a widget edit and `Save`:
    /// a save ends it only when it merges a file changed on disk (see
    /// `Session::absorb`). The interface saves after each write: otherwise
    /// each would make a pack version. `UsePackVersion` and the copy and theme
    /// offers end it themselves once their checks passed: a refused one changes
    /// nothing.
    pub(crate) fn ends_widget_session(&self) -> bool {
        self.changes_deck()
            && !matches!(
                self,
                Self::Save { .. }
                    | Self::UsePackVersion { .. }
                    | Self::UsePackCopy { .. }
                    | Self::UseThemeVersion { .. }
            )
            && !self.is_widget_edit()
    }

    /// Vrai pour une modification de widget (valeurs d'exemple comprises) : elle ne
    /// termine pas la séance des widgets.
    pub(crate) fn is_widget_edit(&self) -> bool {
        matches!(
            self,
            Self::SetGuide { .. }
                | Self::SetWidget { .. }
                | Self::RemoveWidget { .. }
                | Self::RenameWidget { .. }
                | Self::DuplicateWidget { .. }
                | Self::ReorderWidgets { .. }
                | Self::SetExampleValue { .. }
                | Self::EditExampleList { .. }
        )
    }

    /// True for a request that writes the edited pack or theme, or a draft of it.
    ///
    /// `SetPreviewTheme` too: it writes no library version, but takes a free lease and
    /// freezes the base like any edit. Such a request first takes the lease (`Session::ensure_pack_lease` in
    /// pack mode, `Session::ensure_theme_lease` in a theme tab). A copy
    /// (`as_new`) takes its own. Exhaustive on purpose: a new request must
    /// say whether it writes.
    pub(crate) fn writes_edited_draft(&self) -> bool {
        match self {
            Self::Apply { .. }
            | Self::SetPreviewTheme { .. }
            | Self::EditTheme { .. }
            | Self::Undo
            | Self::Redo
            | Self::UndoTurn { .. }
            | Self::ChartKind { .. }
            | Self::ShowAs { .. }
            | Self::DataEdit { .. }
            | Self::ElementParam { .. }
            | Self::ElementList { .. }
            | Self::SavePack { as_new: false }
            | Self::SetGuide { .. }
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::DuplicateWidget { .. }
            | Self::ReorderWidgets { .. }
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
            | Self::ElementPanel { .. }
            | Self::Layouts
            | Self::Packs { .. }
            | Self::SavePack { as_new: true }
            | Self::SaveTheme { as_new: true }
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
            | Self::UsePackVersion { .. }
            | Self::UsePackCopy { .. }
            | Self::ApproveGuides { .. }
            | Self::AcceptReceived { .. }
            | Self::LaterReceived { .. }
            | Self::UseThemeVersion { .. } => false,
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
            | Self::SetGuide { .. }
            | Self::ApproveGuides { .. }
            | Self::LaterReceived { .. }
            | Self::SaveTheme { .. } => true,
            Self::Save { .. }
            | Self::SetPreviewTheme { .. }
            | Self::EditTheme { .. }
            | Self::ChartKind { .. }
            | Self::ShowAs { .. }
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
            | Self::DuplicateWidget { .. }
            | Self::ReorderWidgets { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. }
            | Self::DataEdit { .. }
            | Self::ElementParam { .. }
            | Self::ElementList { .. }
            | Self::UsePackVersion { .. }
            | Self::UsePackCopy { .. }
            | Self::UseThemeVersion { .. }
            | Self::HostSettings { .. }
            | Self::DataPanel { .. }
            | Self::ElementPanel { .. }
            | Self::Layouts
            | Self::Packs { .. }
            | Self::DeletePack { .. }
            | Self::WidgetStatus { .. }
            | Self::WidgetSource { .. }
            | Self::AcceptReceived { .. }
            | Self::PreviewWidget { .. } => false,
        }
    }
}

#[cfg(test)]
#[path = "request_class_tests.rs"]
mod request_class_tests;
