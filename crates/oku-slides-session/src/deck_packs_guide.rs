//! The guides of a pinned pack or theme, as the packs panel shows them.
//!
//! Read-only text with its origin: the use guide, and, for a received version
//! the user has not approved, its edit guide too beside the approval the panel
//! offers, since the approval covers both.

use uuid::Uuid;

use oku_slides_core::{GuideKind, PackFiles, PackVersion};
use snafu::ensure;

use crate::error::NotPinnedSnafu;
use crate::guide_approvals::VersionRef;
use crate::received_review::guides_of;
use crate::{GuideOrigin, Request, Result, Session};

/// The guides of a pinned version the panel shows, with where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedGuideView {
    /// Id of the pack or theme.
    pub id: Uuid,
    /// The pinned version the guides belong to.
    pub version: PackVersion,
    /// The guides shown, as written, never empty: the use guide, then the edit guide while awaiting.
    pub texts: Vec<(GuideKind, String)>,
    /// Where the version came from.
    pub origin: GuideOrigin,
    /// Received and not approved: the agent does not get it yet.
    pub awaiting: bool,
    /// Longest guide the agent gets; the panel shows no more.
    pub max_chars: usize,
}

impl Session {
    /// The guides the panel shows of version `version` of `id` held in `files`; `None` when none.
    ///
    /// The use guide; while the version awaits approval, its edit guide too: the approval
    /// covers both, so the user reads both first.
    pub(crate) fn pinned_guide(
        &self,
        id: Uuid,
        version: &PackVersion,
        files: Option<&PackFiles>,
        theme: bool,
    ) -> Option<PinnedGuideView> {
        let guides = guides_of(files?);
        let awaiting = !self.guide_active(id, version, theme);
        let shown: &[GuideKind] = if awaiting { &[GuideKind::Use, GuideKind::Edit] } else { &[GuideKind::Use] };
        let texts: Vec<(GuideKind, String)> =
            shown.iter().filter(|k| !guides.get(**k).is_empty()).map(|k| (*k, guides.get(*k).to_owned())).collect();
        if texts.is_empty() {
            return None;
        }
        Some(PinnedGuideView {
            id,
            version: version.clone(),
            texts,
            origin: self.guide_origin(id, version, theme),
            awaiting,
            max_chars: self.guide_limit,
        })
    }

    /// Approves the guides of a version the open deck pins, from the packs panel.
    ///
    /// The session checks the pin again: a forged context names no version to approve.
    pub(crate) fn approve_pinned_guides(&mut self, id: Uuid, version: PackVersion) -> Result<()> {
        let held = self.held_versions().iter().any(|h| h.id == id && *h.version == version);
        ensure!(held, NotPinnedSnafu { namespace: id.to_string() });
        self.handle(Request::ApproveGuides { versions: vec![VersionRef { id, version }] })?;
        Ok(())
    }
}
