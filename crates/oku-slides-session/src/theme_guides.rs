//! Theme tab guides: draft writes, and the trust of each draft guide.
//!
//! A write is a draft step like a rename, followed by undo and redo. A draft
//! guide that differs from the one the draft started from was written in this
//! tab: local and active. One kept unchanged takes its base's origin and approval.

use serde_json::Value;

use oku_slides_core::{GuideKind, Guides, Op};

use crate::guide_origin::GuideOrigin;
use crate::theme_edit::{DRAFT_SWITCH_VAR, ThemeEdit, ThemeSwitch, next_switch_rank};
use crate::views::GuidesView;
use crate::{Mode, Result, Session};

/// Both guides, in the order they are checked.
const ROLES: [GuideKind; 2] = [GuideKind::Edit, GuideKind::Use];

/// True if guide `role` of `draft` differs from `start`'s: written in this tab.
fn written(draft: &ThemeEdit, start: Option<&Guides>, role: GuideKind) -> bool {
    start.is_some_and(|g| g.get(role) != draft.guides.get(role))
}

impl Session {
    /// The guides `draft` started from: its base's, none for a child of a built-in.
    ///
    /// `None` when the base is unreadable: every draft guide then keeps the base's trust.
    fn draft_start(&self, draft: &ThemeEdit) -> Option<Guides> {
        let start = self.theme_version(&draft.base).map(|base| draft.starting_guides(&base));
        if start.is_none() {
            tracing::warn!(theme = %draft.base.id, "base of the theme draft unreadable: its guides keep its trust");
        }
        start
    }

    /// The origin of guide `role` of `draft`, and whether it may reach the agent.
    ///
    /// Written in this tab: local and active. Kept from the base: the base's.
    pub(crate) fn draft_guide_trust(&self, draft: &ThemeEdit, role: GuideKind) -> (GuideOrigin, bool) {
        if written(draft, self.draft_start(draft).as_ref(), role) {
            return (GuideOrigin::Local, true);
        }
        let base = &draft.base;
        (self.guide_origin(base.id, &base.version, true), self.guide_active(base.id, &base.version, true))
    }

    /// The guides view of a theme tab's draft.
    ///
    /// Approve is offered only while the draft's guides are its base's: the approval covers
    /// the base's guides, so the view must show them. Once a guide is written here, the view
    /// is local unless a non-empty guide kept from an inactive base still awaits.
    pub(crate) fn theme_guides_view(&self, draft: &ThemeEdit) -> GuidesView {
        let base = &draft.base;
        let mut view = self.guides_view_of(base.id, &base.version, true, draft.guides.clone());
        let start = self.draft_start(draft);
        let Some(start) = start.as_ref() else {
            view.approvable = false;
            return view;
        };
        if ROLES.iter().any(|k| written(draft, Some(start), *k)) {
            view.approvable = false;
            let base_active = self.guide_active(base.id, &base.version, true);
            let pending = ROLES
                .iter()
                .any(|k| !written(draft, Some(start), *k) && !draft.guides.get(*k).is_empty() && !base_active);
            if pending {
                view.kept_awaiting = true;
            } else {
                view.origin = GuideOrigin::Local;
            }
        }
        view
    }

    /// Writes guide `which` of the theme tab's draft, in one undo step.
    pub(crate) fn set_theme_guide(&mut self, which: GuideKind, text: String) -> Result<()> {
        // Outside a theme tab, `ensure_theme_tab` says why.
        let Mode::Theme { edit, switches, .. } = &self.mode else { return self.ensure_theme_tab() };
        let from = edit.draft.clone();
        if from.guides.get(which) == text {
            return Ok(());
        }
        let rank = next_switch_rank(self.doc.sources.deck(), switches);
        let mut to = from.clone();
        to.guides.set(which, text);
        let label = self.localizer.tr("session-undo-theme-guide");
        // The draft changes before the step, which writes the resume state; a failure puts it back.
        self.set_theme_edit(to.clone());
        let ops = vec![Op::SetVar { key: DRAFT_SWITCH_VAR.to_owned(), value: Value::from(rank) }];
        if let Err(e) = self.apply_draft_step(&label, ops) {
            self.set_theme_edit(from);
            return Err(e);
        }
        if let Mode::Theme { switches, .. } = &mut self.mode {
            switches.push(ThemeSwitch { rank, from, to });
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "theme_guides_tests.rs"]
mod theme_guides_tests;
