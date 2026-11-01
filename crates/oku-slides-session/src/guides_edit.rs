//! Guide writes: the edited pack's or theme's agent guides.
//!
//! A pack tab writes the manifest only, through `commit_manifest` (one undo
//! step, like a widget write); a theme tab writes its draft
//! (`theme_guides.rs`), one undo step under a rank like a rename.

use snafu::{ResultExt, ensure};

use oku_slides_core::{GuideKind, PackManifest};

use crate::error::{GuideHasNulSnafu, GuideTooLongSnafu, PackManifestSnafu, WrongDocumentSnafu};
use crate::views::GuidesView;
use crate::{Mode, Result, Session};

impl Session {
    /// Writes guide `which` of the edited pack or theme; empty `text` clears it.
    ///
    /// # Errors
    ///
    /// `WrongDocument` outside a pack or theme tab, `GuideTooLong`, `GuideHasNul`,
    /// and the failures of a pack write (`commit_manifest`).
    pub fn set_guide(&mut self, which: GuideKind, text: String) -> Result<()> {
        let max = self.guide_limit;
        ensure!(text.chars().count() <= max, GuideTooLongSnafu { max });
        ensure!(!text.contains('\0'), GuideHasNulSnafu);
        match &self.mode {
            Mode::Pack { .. } => self.set_pack_guide(which, text),
            Mode::Theme { .. } => self.set_theme_guide(which, text),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => {
                Ok(WrongDocumentSnafu { kind: self.document_kind() }.fail()?)
            }
        }
    }

    /// Writes guide `which` for the agent: like `set_guide`, without echoing it back next turn.
    ///
    /// The list counts as sent only if the agent held it all before the write: a change the
    /// user made since the last send (an approval, an edit, a pin) still goes next turn.
    ///
    /// # Errors
    ///
    /// Those of [`Session::set_guide`].
    pub fn set_guide_by_agent(&mut self, which: GuideKind, text: String) -> Result<()> {
        let current = self.guides_sent_current();
        self.set_guide(which, text)?;
        if current {
            self.mark_guides_sent();
        }
        Ok(())
    }

    /// The longest guide a write may store, in characters (`[render] max_guide_chars`).
    #[must_use]
    pub fn guide_limit(&self) -> usize {
        self.guide_limit
    }

    /// The pack tab's write: the manifest with the new guide, the same widgets.
    fn set_pack_guide(&mut self, which: GuideKind, text: String) -> Result<()> {
        let t = self.widget_target(None)?;
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        manifest.guides.set(which, text);
        let widgets = t.files.widgets().clone();
        self.commit_manifest(&t, manifest, widgets).map(|_| ())
    }

    /// The guides the tab edits (a theme tab's from its draft), `None` outside a pack or theme tab.
    pub(crate) fn guides_view(&self) -> Option<GuidesView> {
        if let Mode::Theme { edit, .. } = &self.mode {
            return Some(self.theme_guides_view(&edit.draft));
        }
        let Mode::Pack { .. } = &self.mode else { return None };
        let manifest = self.widget_target(None).and_then(|t| {
            let manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
            Ok((t.id, t.version, manifest))
        });
        match manifest {
            Ok((id, version, m)) => Some(self.guides_view_of(id, &version, false, m.guides)),
            Err(e) => {
                tracing::warn!(error = %e.one_line(), "guides of the edited pack unreadable");
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "guides_edit_tests.rs"]
mod guides_edit_tests;
