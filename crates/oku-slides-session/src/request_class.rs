//! What each request does: changes the deck, edits, ends the widget session, writes the pack.
//!
//! Classifiers of [`Request`], read by `Session::handle` before it answers.

use super::Request;

impl Request {
    /// Vrai si la requête peut changer le deck (slides ajoutées ou retirées).
    ///
    /// `Save` aussi : un fichier changé sur disque est d'abord fusionné.
    /// L'historique (sauf la liste des versions) : le deck montré change.
    /// `SavePack` aussi : la bibliothèque change, le panneau des packs se relit (`prune`).
    /// Exhaustif à dessein : une requête ajoutée doit dire si elle change le deck.
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
            | Self::SetEditedTheme { .. }
            | Self::SetEditedThemeExtends { .. }
            | Self::EditThemeExtends { .. }
            | Self::SavePack { .. }
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
                | Self::ClosePack
                | Self::SetEditedTheme { .. }
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

    /// True for a request that writes the edited pack or a draft of it, in pack mode.
    ///
    /// Such a request first takes the lease (`Session::ensure_pack_lease`).
    /// Exhaustive on purpose: a new request must say whether it writes.
    pub(crate) fn writes_edited_pack(&self) -> bool {
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
            | Self::HostToken { .. }
            | Self::SavePack { as_new: false }
            | Self::SetEditedTheme { .. }
            | Self::SetEditedThemeExtends { .. }
            | Self::EditThemeExtends { .. }
            | Self::RenameEditedTheme { .. }
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. } => true,
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
}

#[cfg(test)]
#[path = "request_class_tests.rs"]
mod request_class_tests;
