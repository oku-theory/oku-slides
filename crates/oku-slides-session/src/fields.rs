//! Champs éditables d'une slide pour l'interface : boîte rendue, texte tel
//! que saisi, dans l'ordre de Tab (`Slide::text_fields`).

use std::borrow::Cow;

use serde::Serialize;

use oku_slides_core::{Deck, FieldOwner, ParamType, Slide, WidgetRegistry};
use oku_slides_render::{FieldBox, FieldSide, TextStyle};

use crate::text::paragraphs_to_text;

/// Objet qui porte un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnerKind {
    Slide,
    Element,
}

/// Nature du texte d'un champ : une ligne brute, ou du markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Plain,
    Rich,
}

/// Un champ éditable en place, posé sur son texte rendu.
#[derive(Debug, Serialize)]
pub struct FieldView<'a> {
    /// Id de la slide ou de l'élément qui porte le champ.
    pub owner: &'a str,
    pub owner_kind: OwnerKind,
    /// Widget de l'objet (`kind`).
    pub widget: &'a str,
    /// Nom du paramètre.
    pub field: &'a str,
    pub kind: FieldKind,
    /// Libellé du paramètre dans le schéma du widget.
    pub label: Option<&'a str>,
    /// Texte tel que saisi ; markdown, un paragraphe par ligne, pour `rich`.
    pub text: Cow<'a, str>,
    /// Sans texte : placeholder à dessiner par l'interface.
    pub empty: bool,
    /// Champ vide sans place au rendu : dessiné avant ou après son ancre.
    pub side: Option<FieldSide>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Zone disponible pour le texte : début et largeur (contient la boîte).
    pub ax: f64,
    pub aw: f64,
    /// Typographie du rendu, pour un éditeur calé sur le texte.
    pub style: Option<&'a TextStyle>,
}

/// Texte d'un paramètre `plain` de la slide : la valeur liée (`bind`)
/// l'emporte, comme au rendu (`_args`).
pub(crate) fn slide_text<'d>(
    deck: &'d Deck,
    slide: &'d Slide,
    registry: &WidgetRegistry,
    field: &str,
) -> Option<&'d str> {
    let bind = registry.get(&slide.kind).and_then(|w| w.param(field)).and_then(|p| p.bind.as_deref());
    match bind {
        Some(key) => deck.params.field(key),
        None => slide.params.text(field),
    }
}

/// Champs de `slide` dont le rendu a posé une boîte sur `boxes`, dans
/// l'ordre de Tab. Un paramètre sans boîte (`author`, `date` de la
/// couverture) n'est pas éditable en place : il est omis.
pub(crate) fn field_views<'a>(
    deck: &'a Deck,
    slide: &'a Slide,
    registry: &'a WidgetRegistry,
    boxes: &'a [FieldBox],
) -> Vec<FieldView<'a>> {
    let views: Vec<FieldView<'a>> = slide
        .text_fields(registry)
        .into_iter()
        .filter_map(|tf| {
            let name = tf.param.name.as_str();
            let b = boxes.iter().find(|b| b.owner == tf.owner_id && b.field == name)?;
            let rich = tf.param.ty == ParamType::Rich;
            let (owner_kind, widget, params) = match tf.owner {
                FieldOwner::Slide => (OwnerKind::Slide, slide.kind.as_str(), &slide.params),
                FieldOwner::Element(e) => (OwnerKind::Element, e.kind.as_str(), &e.params),
            };
            let text = match tf.param.bind.as_deref() {
                // Paramètre lié (`plain` seulement) : la valeur du deck, où les ops l'écrivent.
                Some(key) => Cow::Borrowed(deck.params.field(key).unwrap_or_default()),
                _ if rich => Cow::Owned(paragraphs_to_text(params.rich(name).unwrap_or_default())),
                _ => Cow::Borrowed(params.text(name).unwrap_or_default()),
            };
            Some(FieldView {
                owner: tf.owner_id,
                owner_kind,
                widget,
                field: name,
                kind: if rich { FieldKind::Rich } else { FieldKind::Plain },
                label: tf.param.label.as_deref(),
                text,
                empty: b.empty,
                side: b.side,
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h,
                ax: b.ax.unwrap_or(b.x).min(b.x),
                aw: b.aw.unwrap_or(b.w).max(b.w),
                style: b.style.as_ref(),
            })
        })
        .collect();
    let ours = boxes.iter().filter(|b| b.owner == slide.id || slide.locate(&b.owner).is_some());
    for b in ours {
        if !views.iter().any(|v| v.owner == b.owner && v.field == b.field) {
            tracing::debug!(owner = %b.owner, field = %b.field, "champ rendu sans paramètre texte déclaré");
        }
    }
    views
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod fields_tests;
