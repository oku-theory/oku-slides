//! Contrôle d'un sous-arbre inséré (`add_element`, `add_slide`,
//! `replace_element`) : chaque paramètre déclaré et de la forme de son type,
//! chaque genre connu et de la bonne catégorie, chaque enfant accepté par
//! son slot, chaque liste dans ses bornes de taille. La profondeur est contrôlée avant ([`check_depth`](super)) :
//! la récursion est bornée par [`MAX_SLOT_DEPTH`](crate::MAX_SLOT_DEPTH).

use crate::error::{ChoiceOutOfRangeSnafu, OpError, ParamTypeSnafu, SlotRejectsSnafu};
use crate::params::{ParamValue, Params};
use crate::validate::{Fit, fits};
use crate::widget::{Category, ParamType, WidgetRegistry};

use super::list::ListSpec;
use super::{declared, widget_of};

/// Contrôle `params` contre le widget `kind` (de catégorie `category`), puis
/// chaque élément de ses slots, récursivement.
pub(super) fn check_params(
    registry: &WidgetRegistry,
    kind: &str,
    category: Category,
    params: &Params,
) -> Result<(), OpError> {
    let schema = widget_of(registry, kind, category)?;
    for (name, value) in params.iter() {
        let p = declared(schema, name)?;
        match (fits(&p.ty, value), value) {
            (Fit::Ok, _) => {}
            (Fit::OutOfChoice, ParamValue::Text(s)) => {
                return Err(ChoiceOutOfRangeSnafu { param: name, value: s }.build().into());
            }
            _ => return Err(ParamTypeSnafu { param: name, expected: expected(&p.ty) }.build().into()),
        }
        if let (Some(spec), ParamValue::List(items)) = (ListSpec::of(name, &p.ty), value) {
            spec.sizes_fit(items)?;
        }
        if let (ParamType::Slot { accepts, .. }, ParamValue::Slot(children)) = (&p.ty, value) {
            for child in children {
                check_accepts(accepts.as_deref(), name, &child.kind)?;
                check_params(registry, &child.kind, Category::Element, &child.params)?;
            }
        }
    }
    Ok(())
}

/// Refuse `kind` dans le slot `slot` s'il n'est pas dans ses `accepts`.
pub(super) fn check_accepts(accepts: Option<&[String]>, slot: &str, kind: &str) -> Result<(), OpError> {
    if accepts.is_some_and(|kinds| !kinds.iter().any(|k| k == kind)) {
        return Err(SlotRejectsSnafu { slot, kind }.build().into());
    }
    Ok(())
}

/// Forme attendue d'un paramètre, pour le message de `ParamType`.
fn expected(ty: &ParamType) -> &'static str {
    match ty {
        ParamType::Plain | ParamType::Asset => "a string",
        ParamType::Choice { .. } => "one of its values",
        ParamType::Rich => "a list of paragraphs",
        ParamType::Slot { .. } => "a list of elements",
        ParamType::Flag => "a boolean",
        ParamType::Number { .. } => "a number within its bounds",
        ParamType::Grid => "a grid",
        ParamType::Host { .. } => "a chart option",
        ParamType::List { .. } => "a list of items",
    }
}
