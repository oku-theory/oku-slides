//! Champs éditables d'une slide pour l'interface : boîte rendue, texte tel
//! que saisi, dans l'ordre de Tab (`Slide::text_fields`).

use std::borrow::Cow;

use serde::Serialize;

use oku_slides_core::{Deck, FieldOwner, ParamType, Slide, TextField, WidgetRegistry};
use oku_slides_render::{FieldBox, FieldSide, TextStyle};

use crate::text::paragraphs_to_text;

/// Objet qui porte un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnerKind {
    /// A parameter of the slide itself.
    Slide,
    /// A parameter of an element placed on the slide.
    Element,
}

/// Nature du texte d'un champ : une ligne brute, ou du markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    /// One line of plain text, without formatting.
    Plain,
    /// Editing markdown, one paragraph per line.
    Rich,
}

/// Un champ éditable en place, posé sur son texte rendu.
#[derive(Debug, Serialize)]
pub struct FieldView<'a> {
    /// Id de la slide ou de l'élément qui porte le champ.
    pub owner: &'a str,
    /// Slide or element: says what `owner` refers to.
    pub owner_kind: OwnerKind,
    /// Widget de l'objet (`kind`).
    pub widget: &'a str,
    /// Nom du champ : le paramètre, `liste/id/colonne` pour un item de liste, ou
    /// `liste/id/sous-liste/sous-id/colonne` pour un item imbriqué.
    pub field: Cow<'a, str>,
    /// Pour un item : le chemin de sa liste, le paramètre `list` ou `liste/id/colonne`
    /// pour un item d'une liste imbriquée.
    pub list: Option<Cow<'a, str>>,
    /// Pour un item : son id.
    pub item: Option<&'a str>,
    /// Paramètre, ou colonne de la liste pour un item.
    pub column: &'a str,
    /// Plain text or markdown: picks the in-place editor.
    pub kind: FieldKind,
    /// Libellé du paramètre dans le schéma du widget.
    pub label: Option<&'a str>,
    /// Texte tel que saisi ; markdown, un paragraphe par ligne, pour `rich`.
    pub text: Cow<'a, str>,
    /// Sans texte : placeholder à dessiner par l'interface.
    pub empty: bool,
    /// Lié à un paramètre du deck (`bind`) : sa valeur est celle du deck.
    pub bound: bool,
    /// Champ vide sans place au rendu : dessiné avant ou après son ancre.
    pub side: Option<FieldSide>,
    /// Left edge of the rendered text, in pt from the page's left edge.
    pub x: f64,
    /// Top edge of the rendered text, in pt from the top of the page.
    pub y: f64,
    /// Width of the rendered text, in pt.
    pub w: f64,
    /// Height of the rendered text, in pt.
    pub h: f64,
    /// Zone disponible pour le texte : début et largeur (contient la boîte).
    pub ax: f64,
    /// Width of the available area, in pt (at least `w`).
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

/// Widget de l'objet qui porte le champ `tf` de `slide`.
fn owner_widget<'a>(slide: &'a Slide, tf: &TextField<'a>) -> &'a str {
    match tf.owner {
        FieldOwner::Slide => &slide.kind,
        FieldOwner::Element(e) => &e.kind,
    }
}

/// Texte du champ `tf` de `slide`, tel que saisi ; markdown pour `rich`.
///
/// Pour un item de liste : la colonne de l'item.
pub(crate) fn field_text<'a>(deck: &'a Deck, tf: &TextField<'a>) -> Cow<'a, str> {
    let column = tf.param.name.as_str();
    match tf.param.bind.as_deref() {
        // Paramètre lié (`plain` seulement, jamais une colonne) : la valeur du deck, où les ops l'écrivent.
        Some(key) if tf.list.is_none() => Cow::Borrowed(deck.params.field(key).unwrap_or_default()),
        _ if tf.param.ty == ParamType::Rich => {
            Cow::Owned(paragraphs_to_text(tf.values.rich(column).unwrap_or_default()))
        }
        _ => Cow::Borrowed(tf.values.text(column).unwrap_or_default()),
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
            let name = tf.name();
            let b = boxes.iter().find(|b| b.owner == tf.owner_id && b.field == name)?;
            let rich = tf.param.ty == ParamType::Rich;
            let owner_kind = match tf.owner {
                FieldOwner::Slide => OwnerKind::Slide,
                FieldOwner::Element(_) => OwnerKind::Element,
            };
            Some(FieldView {
                owner: tf.owner_id,
                owner_kind,
                widget: owner_widget(slide, &tf),
                field: name,
                list: tf.list_path(),
                item: tf.list.map(|(_, i)| i.id.as_str()),
                column: tf.param.name.as_str(),
                kind: if rich { FieldKind::Rich } else { FieldKind::Plain },
                label: tf.param.label.as_deref(),
                text: field_text(deck, &tf),
                empty: b.empty,
                bound: tf.param.bind.is_some() && tf.list.is_none(),
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
