//! Duplicate and reorder the items of the edited pack.

use std::collections::BTreeSet;

use serde::Serialize;
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{Namespace, WidgetFile, item_order};

use crate::error::{
    InvalidWidgetNameSnafu, NoFreeCopyNameSnafu, PackManifestSnafu, TooManyWidgetsSnafu, UnknownWidgetSnafu,
    WidgetExistsSnafu, WidgetOrderMismatchSnafu, WidgetTooLargeSnafu,
};
use crate::pack::PackManifest;
use crate::widgets_edit::{WidgetStatus, parse_name, renamed_source, with_render};
use crate::{Result, Session};

/// What a duplicate returns: the copy's name and the statuses after it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetDuplicate {
    /// The name of the new item.
    pub name: String,
    /// The status of every file of the pack after the copy.
    pub statuses: Vec<WidgetStatus>,
}

impl Session {
    /// Copies item `from` to `to`, or the first free `<from>-N`, right after it.
    ///
    /// The copy declares its own name and carries the example values and the starting style.
    ///
    /// # Errors
    ///
    /// Fails for `_commun`, an invalid or taken name (`WidgetExists`), an unknown item, a pack at
    /// its limits, no free copy name (`NoFreeCopyName`) or a failed write; like
    /// [`Self::set_widget`] for a leased pack (`PackOpenInTab`, `UnknownPack`, `LeaseMissing`,
    /// `PackVersionMissing`).
    pub fn duplicate_widget(
        &mut self,
        pack: Option<&Namespace>,
        from: &str,
        to: Option<&str>,
    ) -> Result<WidgetDuplicate> {
        self.copy_widget(pack, from, to).map(|(_, copy)| copy)
    }

    /// The duplicate itself, with the copy's file name for the agent's mark.
    fn copy_widget(
        &mut self,
        pack: Option<&Namespace>,
        from: &str,
        to: Option<&str>,
    ) -> Result<(WidgetFile, WidgetDuplicate)> {
        let from = parse_name(from)?;
        ensure!(!from.is_common(), InvalidWidgetNameSnafu { name: from.as_str() });
        let t = self.widget_target(pack)?;
        let source = t.files.widget(&from).context(UnknownWidgetSnafu { name: from.as_str() })?;
        let to = match to {
            Some(name) => {
                let to = parse_name(name)?;
                ensure!(!to.is_common(), InvalidWidgetNameSnafu { name: to.as_str() });
                ensure!(t.files.widget(&to).is_none(), WidgetExistsSnafu { name: to.as_str() });
                to
            }
            None => (2..)
                .map_while(|n| WidgetFile::parse(&format!("{}-{n}", from.as_str())))
                .find(|c| t.files.widget(c).is_none())
                .context(NoFreeCopyNameSnafu { from: from.as_str() })?,
        };
        let limits = self.widget_limits;
        ensure!(t.files.widgets().len() < limits.files, TooManyWidgetsSnafu { max: limits.files });
        let copy = renamed_source(source, from.as_str(), to.as_str());
        ensure!(copy.len() <= limits.bytes, WidgetTooLargeSnafu { name: to.as_str(), max: limits.kib });
        let mut widgets = t.files.widgets().clone();
        widgets.insert(to.clone(), copy);
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        if let Some(examples) = manifest.examples.get(&from).cloned() {
            manifest.examples.insert(to.clone(), examples);
        }
        if let Some(style) = manifest.styles.get(&from).cloned() {
            manifest.styles.insert(to.clone(), style);
        }
        // The copy sits right after its original.
        let mut order: Vec<WidgetFile> = item_order(&manifest.order, &t.files).into_iter().cloned().collect();
        let at = order.iter().position(|n| *n == from).map_or(order.len(), |i| i + 1);
        order.insert(at, to.clone());
        manifest.order = order;
        let statuses = self.commit_manifest(&t, manifest, widgets).map(|s| with_render(s.0, s.1, &to))?;
        let copy = WidgetDuplicate { name: to.as_str().to_owned(), statuses };
        Ok((to, copy))
    }

    /// Agent duplicate (tool `duplicate_widget`): copied, persisted, the copy marked.
    ///
    /// A later user write of the copy then tells the agent, as after `set_widget_by_agent`.
    ///
    /// # Errors
    ///
    /// Fails like [`Self::duplicate_widget`].
    pub fn duplicate_widget_by_agent(
        &mut self,
        pack: Option<&Namespace>,
        from: &str,
        to: Option<&str>,
    ) -> Result<WidgetDuplicate> {
        let (name, copy) = self.copy_widget(pack, from, to)?;
        if let Some(scratch) = self.scratch_mut() {
            scratch.mark_agent(&name);
        }
        self.persist(true);
        Ok(copy)
    }

    /// Stores `order` as the pack's item order; each item must be listed once.
    ///
    /// The same order as the stored one makes no version.
    ///
    /// # Errors
    ///
    /// Fails for an invalid name, or a list missing an item or holding an unknown name,
    /// `_commun` or a repeat (`WidgetOrderMismatch`); like [`Self::set_widget`] for a leased pack
    /// (`PackOpenInTab`, `UnknownPack`, `LeaseMissing`, `PackVersionMissing`).
    pub fn reorder_widgets(&mut self, pack: Option<&Namespace>, order: &[String]) -> Result<Vec<WidgetStatus>> {
        let listed = order.iter().map(|n| parse_name(n)).collect::<Result<Vec<WidgetFile>>>()?;
        let t = self.widget_target(pack)?;
        let items: Vec<&WidgetFile> = item_order(&[], &t.files);
        let mut seen = BTreeSet::new();
        let extra: Vec<String> = listed
            .iter()
            .filter(|n| !items.contains(n) || !seen.insert((*n).clone()))
            .map(|n| n.as_str().to_owned())
            .collect();
        let missing: Vec<String> =
            items.iter().filter(|n| !seen.contains(**n)).map(|n| n.as_str().to_owned()).collect();
        ensure!(missing.is_empty() && extra.is_empty(), WidgetOrderMismatchSnafu { missing, extra });
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        manifest.order = listed;
        let widgets = t.files.widgets().clone();
        self.commit_manifest(&t, manifest, widgets).map(|(statuses, _)| statuses)
    }
}

#[cfg(test)]
#[path = "widgets_order_tests.rs"]
mod widgets_order_tests;
