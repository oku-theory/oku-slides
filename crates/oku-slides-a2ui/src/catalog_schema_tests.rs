//! The catalog schema's component names, shared by the crate's tests.

use speculoos::prelude::*;

use crate::ComponentKind;

/// The 26 catalog components, in `ComponentKind` declaration order.
pub(crate) const CATALOG: [&str; 26] = [
    "Column",
    "Card",
    "Text",
    "TextField",
    "RichTextField",
    "Stepper",
    "Row",
    "Button",
    "ChoiceChips",
    "Slider",
    "ColorField",
    "Select",
    "Switch",
    "ListTable",
    "SlideThumb",
    "ThemeChoice",
    "DiffText",
    "Prompt",
    "Timeline",
    "Receipt",
    "Disclosure",
    "Badge",
    "Markdown",
    "Notice",
    "ChartData",
    "TableData",
];

/// The component names of the `ComponentKind` schema, in branch order.
pub(crate) fn catalog() -> Vec<String> {
    let schema = serde_json::to_value(schemars::schema_for!(ComponentKind)).unwrap();
    schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| crate::playbook::reference::branch_name(b).unwrap().to_owned())
        .collect()
}

#[test]
fn component_kind_schema_has_one_branch_per_component_in_order() {
    assert_that!(catalog()).is_equal_to(CATALOG.map(str::to_owned).to_vec());
}
