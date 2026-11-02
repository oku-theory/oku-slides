//! The host-element surface: a deck element's settings at its current values.
//!
//! The interface fetches it (`element_panel`) and renders it in the inspector; its writes
//! (`element_param`, `element_list`) are core ops, one undo step each.

use oku_slides_core::{ListOp, Op, Target};
use serde_json::Value;

use crate::agent::Surface;
use crate::agent::bounds::label;
use crate::agent::forms::params_with;
use crate::panel::HostParams;
use crate::param_fields::param_label;
use crate::{Result, Session};

/// Surface of the selected element's settings.
pub(crate) const HOST_ELEMENT: &str = "host-element";

impl Session {
    /// Settings of `element` on `slide`; `None` outside an editable deck or for an unknown element.
    pub(crate) fn element_panel(&self, slide: &str, element: &str) -> Option<Surface> {
        self.ensure_editable().ok()?;
        let deck = self.doc.sources.deck();
        let located = deck.slide(slide)?.locate(element)?;
        let schema = self.registry.get(&located.element.kind)?;
        let p = HostParams::from_schema(
            schema,
            &located.element.params,
            slide,
            "session-widget-params",
            true,
            &self.localizer,
        );
        Some(params_with(HOST_ELEMENT, &p.spec, &p.extras(), &self.slide_palette(), &self.localizer))
    }

    /// Sets `key` of `element`; core validates the param and its value. One undo step.
    ///
    /// `null` removes the param (`SetParam` semantics): the widget's default applies.
    pub(crate) fn element_param(&mut self, slide: &str, element: &str, key: &str, value: Value) -> Result<()> {
        let target = Target { slide: slide.to_owned(), element: Some(element.to_owned()) };
        let name = self.element_param_label(slide, element, key);
        let op = Op::SetParam { target, param: key.to_owned(), value };
        self.apply(&name, vec![op], None)
    }

    /// Applies one list change to `key` of `element` (`key` may be `param/item/column`). One undo step.
    pub(crate) fn element_list(&mut self, slide: &str, element: &str, key: &str, change: ListOp) -> Result<()> {
        let target = Target { slide: slide.to_owned(), element: Some(element.to_owned()) };
        let name = self.element_param_label(slide, element, key);
        let op = Op::EditList { target, param: key.to_owned(), change };
        self.apply(&name, vec![op], None)
    }

    /// The `agent-undo-param` label with the param's label, or its raw key if the param is unknown.
    fn element_param_label(&self, slide: &str, element: &str, key: &str) -> String {
        let head = key.split('/').next().unwrap_or(key);
        let found = self
            .doc
            .sources
            .deck()
            .slide(slide)
            .and_then(|s| s.locate(element))
            .and_then(|located| self.registry.get(&located.element.kind))
            .and_then(|schema| schema.param(head).map(|p| param_label(&schema.name, p, &self.localizer)));
        let shown = label(&found.unwrap_or_else(|| key.to_owned()));
        self.localizer.tr_with("agent-undo-param", &[("label", shown.into())])
    }
}

#[cfg(test)]
#[path = "element_panel_tests.rs"]
mod element_panel_tests;
