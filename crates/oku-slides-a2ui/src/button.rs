//! Typed builder of a `Button` component.
//!
//! Its flags (`primary`, `danger`, `icon`) can only be set on a button: no setter on
//! [`Component`] that would do nothing on another kind (t-1090).

use crate::action::{Action, Dynamic};
use crate::component::{Component, ComponentKind};
use crate::props::ButtonIcon;

/// A button being built; [`ButtonBuilder::build`] gives the component.
#[derive(Debug, Clone)]
#[must_use]
pub struct ButtonBuilder {
    id: String,
    label: String,
    action: Action,
    primary: bool,
    danger: bool,
    icon: Option<ButtonIcon>,
}

impl Component {
    /// A button with a literal label, to refine before [`ButtonBuilder::build`].
    pub fn button_builder(id: impl Into<String>, label: impl Into<String>, action: Action) -> ButtonBuilder {
        ButtonBuilder { id: id.into(), label: label.into(), action, primary: false, danger: false, icon: None }
    }

    /// A button with a literal label; `primary` marks it as the main action.
    pub fn button(id: impl Into<String>, label: impl Into<String>, primary: bool, action: Action) -> Self {
        Self::button_builder(id, label, action).primary(primary).build()
    }
}

impl ButtonBuilder {
    /// Marks the button, or not, as the main action.
    pub fn primary(mut self, on: bool) -> Self {
        self.primary = on;
        self
    }

    /// Marks the button, or not, as a committing action.
    pub fn danger(mut self, on: bool) -> Self {
        self.danger = on;
        self
    }

    /// Shows the button by the `icon` pictogram instead of its label.
    pub fn icon(mut self, icon: ButtonIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The button component.
    #[must_use]
    pub fn build(self) -> Component {
        Component::new(
            self.id,
            ComponentKind::Button {
                label: Dynamic::Literal(self.label),
                primary: self.primary.then_some(true),
                danger: self.danger.then_some(true),
                icon: self.icon,
                action: self.action,
            },
        )
    }
}

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
