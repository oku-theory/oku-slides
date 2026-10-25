//! Application to the deck of the multi-step [`Op`](super::Op) variants.
//!
//! One function per non-trivial op: checks first, writes next, so
//! that a refused op leaves the deck intact.

use std::collections::HashSet;

use serde_json::{Map, Value};
use snafu::OptionExt;

use super::{
    SlotRef, Target, check_accepts, check_containing_accepts, check_depth, check_ids, check_params, containing_slot,
    declared, element_mut, has_filled_flow_slot, host, list, located, not_found, opt_string, param_value, remove_path,
    set_or_remove_map, slide_mut, slide_ref, subtree_ids, target_mut, target_ref, token_path, widget, widget_of,
};
use crate::error::{
    DuplicateIdSnafu, IndexOutOfRangeSnafu, InvalidValueSnafu, OpError, ParamTypeSnafu, Shape, SlideNotFoundSnafu,
    ValueReason,
};
use crate::model::{Cell, Deck, DeckParams, Element, Frame, Slide};
use crate::params::ParamValue;
use crate::widget::{Category, ParamType, SlotMode, WidgetRegistry};

/// `set_deck_param`: known deck field, or free parameter; `null` removes it.
pub(super) fn set_deck_param(deck: &mut Deck, key: &str, value: Value) -> Result<(), OpError> {
    if DeckParams::is_field(key) {
        let text = opt_string(key, value)?;
        deck.params.set_field(key, text);
    } else {
        set_or_remove_map(&mut deck.params.extra, key, value);
    }
    Ok(())
}

/// `set_kind`: same category, slot that accepts the new widget, hosts revalidated.
pub(super) fn set_kind(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    target: &Target,
    kind: String,
) -> Result<(), OpError> {
    let (_, _, category) = target_ref(deck, target)?;
    widget_of(registry, &kind, category)?;
    if let Some(id) = &target.element {
        check_containing_accepts(registry, slide_ref(deck, &target.slide)?, id, &kind)?;
    }
    let subject = target.element.as_deref().unwrap_or(&target.slide);
    let mut normalized = target_ref(deck, target)?.1.clone();
    host::check_host_params(registry, &kind, &mut normalized, subject)?;
    let (k, params) = target_mut(deck, target)?;
    *params = normalized;
    if *k != kind {
        list::seed_lists(registry, &kind, params)?;
        *k = kind;
    }
    Ok(())
}

/// `set_frame`: non-empty frame, checked before looking up the element.
pub(super) fn set_frame(deck: &mut Deck, slide: &str, element: &str, frame: Option<Frame>) -> Result<(), OpError> {
    if let Some(f) = frame
        && !(f.w > 0.0 && f.h > 0.0)
    {
        return Err(InvalidValueSnafu { field: "frame", reason: ValueReason::EmptyFrame }.build().into());
    }
    let s = slide_mut(deck, slide)?;
    let e = element_mut(s, element).ok_or_else(|| not_found(slide, element))?;
    e.frame = frame;
    Ok(())
}

/// `set_cell`: element of a free slot, cell in the slide's resolved grid.
pub(super) fn set_cell(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    slide: &str,
    element: &str,
    cell: Option<Cell>,
) -> Result<(), OpError> {
    if let Some(c) = cell {
        let s = slide_ref(deck, slide)?;
        let l = located(s, element)?;
        // A `flow` slot follows the layout: the grid only places free elements.
        if containing_slot(registry, s, &l).is_some_and(|(mode, _)| mode == SlotMode::Flow) {
            return Err(InvalidValueSnafu { field: "cell", reason: ValueReason::BlocksOffGrid }.build().into());
        }
        if !c.fits(&s.resolved_grid(registry)) {
            return Err(InvalidValueSnafu { field: "cell", reason: ValueReason::OutsideGrid }.build().into());
        }
    }
    let s = slide_mut(deck, slide)?;
    let e = element_mut(s, element).ok_or_else(|| not_found(slide, element))?;
    e.cell = cell;
    Ok(())
}

/// `replace_element`: same depth, slot that accepts it, ids free outside the replaced subtree.
pub(super) fn replace_element(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    slide: &str,
    mut element: Element,
) -> Result<(), OpError> {
    host::check_hosts(registry, &mut element)?;
    {
        let s = slide_ref(deck, slide)?;
        let previous = located(s, &element.id)?;
        check_depth(previous.depth, &element)?;
        check_containing_accepts(registry, s, &element.id, &element.kind)?;
        check_params(registry, &element.kind, Category::Element, &element.params)?;
        // The children of a replaced element must not collide.
        let replaced: HashSet<&str> = subtree_ids(previous.element).collect();
        check_ids(s.element_ids().filter(|id| !replaced.contains(id)).collect(), &element)?;
    }
    let s = slide_mut(deck, slide)?;
    let e = element_mut(s, &element.id).ok_or_else(|| not_found(slide, &element.id))?;
    *e = element;
    Ok(())
}

/// `add_slide`: free id, normalized hosts, unique elements of bounded depth.
pub(super) fn add_slide(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    index: Option<usize>,
    defaults: bool,
    mut slide: Slide,
) -> Result<(), OpError> {
    if deck.slide(&slide.id).is_some() {
        return Err(DuplicateIdSnafu { id: slide.id }.build().into());
    }
    host::check_host_params(registry, &slide.kind, &mut slide.params, &slide.id)?;
    let mut taken = HashSet::new();
    for (_, list) in slide.params.slots_mut() {
        for e in list {
            check_depth(0, e)?;
            host::check_hosts(registry, e)?;
            if let Some(dup) = subtree_ids(e).find(|id| !taken.insert(*id)) {
                return Err(DuplicateIdSnafu { id: dup }.build().into());
            }
        }
    }
    check_params(registry, &slide.kind, Category::Slide, &slide.params)?;
    if defaults {
        list::seed_lists(registry, &slide.kind, &mut slide.params)?;
        list::seed_slots(registry, &mut slide.params)?;
    }
    let at = index.unwrap_or(deck.slides.len());
    if at > deck.slides.len() {
        return Err(IndexOutOfRangeSnafu { index: at, len: deck.slides.len() }.build().into());
    }
    deck.slides.insert(at, slide);
    Ok(())
}

/// `move_slide`: existing slide, final position within bounds.
pub(super) fn move_slide(deck: &mut Deck, slide: &str, index: usize) -> Result<(), OpError> {
    let len = deck.slides.len();
    let i = deck.slide_index(slide).context(SlideNotFoundSnafu { slide })?;
    if index >= len {
        return Err(IndexOutOfRangeSnafu { index, len }.build().into());
    }
    let s = deck.slides.remove(i);
    deck.slides.insert(index, s);
    Ok(())
}

/// `unset_token`: removes the token, then the objects left empty.
///
/// With no token left, `tokens` becomes `null`.
pub(super) fn unset_token(deck: &mut Deck, path: &[String]) -> Result<(), OpError> {
    token_path(path)?;
    remove_path(&mut deck.tokens, path);
    if deck.tokens.as_object().is_some_and(Map::is_empty) {
        deck.tokens = Value::Null;
    }
    Ok(())
}

/// `set_tokens`: an object, or `null` for no tokens.
pub(super) fn set_tokens(deck: &mut Deck, tokens: Value) -> Result<(), OpError> {
    if !(tokens.is_object() || tokens.is_null()) {
        return Err(InvalidValueSnafu { field: "tokens", reason: ValueReason::NotTokens }.build().into());
    }
    deck.tokens = tokens;
    Ok(())
}

/// `set_param`: checks, then writes or removes a parameter.
///
/// Declared parameter, value converted by [`param_value`], link to the deck,
/// grid refused on a slide with a filled `flow` slot.
pub(super) fn set_param(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    target: &Target,
    param: &str,
    value: Value,
) -> Result<(), OpError> {
    let (kind, params, _) = target_ref(deck, target)?;
    let schema = widget(registry, kind)?;
    let p = declared(schema, param)?;
    let converted = param_value(&p.ty, param, value)?;
    if p.ty == ParamType::Grid
        && converted.is_some()
        && target.element.is_none()
        && has_filled_flow_slot(schema, params)
    {
        return Err(InvalidValueSnafu { field: param, reason: ValueReason::BlocksOffGrid }.build().into());
    }
    if let Some(key) = &p.bind {
        let text = match converted {
            None => None,
            Some(ParamValue::Text(s)) => Some(s),
            Some(_) => return Err(ParamTypeSnafu { param, expected: Shape::Text.or_null() }.build().into()),
        };
        deck.params.set_field(key, text);
        return Ok(());
    }
    let (_, params) = target_mut(deck, target)?;
    match converted {
        Some(v) => params.set(param, v),
        None => params.remove(param),
    };
    Ok(())
}

/// `add_element`: checks, then inserts the element.
///
/// Declared slot that accepts the element, known and consistent element,
/// free ids, bounded depth.
pub(super) fn add_element(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    slot: &SlotRef,
    index: Option<usize>,
    defaults: bool,
    mut element: Element,
) -> Result<(), OpError> {
    let (at, _) = check_add(registry, slide_ref(deck, &slot.owner.slide)?, slot, index, &mut element)?;
    if defaults {
        list::seed_element(registry, &mut element)?;
    }
    insert_at(slide_mut(deck, &slot.owner.slide)?, slot, at, element)
}

/// Checks of `add_element` in the slide `s`.
///
/// Returns the position and the mode of the slot.
///
/// Declared slot that accepts the element, known and consistent element (hosts
/// normalized), ids free in `s`, bounded depth, position
/// within the slot's bounds.
pub(super) fn check_add(
    registry: &WidgetRegistry,
    s: &Slide,
    slot: &SlotRef,
    index: Option<usize>,
    element: &mut Element,
) -> Result<(usize, SlotMode), OpError> {
    let SlotRef { owner, param } = slot;
    // Depth of the owner in slots below the slide (0: the slide).
    let (kind, params, owner_slots) = match &owner.element {
        None => (s.kind.as_str(), &s.params, 0),
        Some(id) => {
            let l = located(s, id)?;
            (l.element.kind.as_str(), &l.element.params, l.depth + 1)
        }
    };
    let p = declared(widget(registry, kind)?, param)?;
    let ParamType::Slot { accepts, mode } = &p.ty else {
        return Err(ParamTypeSnafu { param, expected: Shape::SetParamOnly.required() }.build().into());
    };
    // A value that is not a list of elements would be overwritten by `slot_mut`.
    let len = match (params.get(param), params.slot(param)) {
        (None, _) => 0,
        (Some(_), Some(list)) => list.len(),
        (Some(_), None) => {
            return Err(ParamTypeSnafu { param, expected: Shape::Elements.required() }.build().into());
        }
    };
    check_accepts(accepts.as_deref(), param, &element.kind)?;
    widget_of(registry, &element.kind, Category::Element)?;
    host::check_hosts(registry, element)?;
    check_ids(s.element_ids().collect(), element)?;
    check_depth(owner_slots, element)?;
    check_params(registry, &element.kind, Category::Element, &element.params)?;
    let at = index.unwrap_or(len);
    if at > len {
        return Err(IndexOutOfRangeSnafu { index: at, len }.build().into());
    }
    Ok((at, *mode))
}

/// Inserts `element` at `at` in slot `slot` of slide `s` (checked by [`check_add`]).
pub(super) fn insert_at(s: &mut Slide, slot: &SlotRef, at: usize, element: Element) -> Result<(), OpError> {
    let SlotRef { owner, param } = slot;
    let params = match &owner.element {
        None => &mut s.params,
        Some(id) => &mut element_mut(s, id).ok_or_else(|| not_found(&owner.slide, id))?.params,
    };
    let list = params.slot_mut(param).context(InvalidValueSnafu { field: param, reason: ValueReason::NotASlot })?;
    if at > list.len() {
        return Err(IndexOutOfRangeSnafu { index: at, len: list.len() }.build().into());
    }
    list.insert(at, element);
    Ok(())
}
