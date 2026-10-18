//! Une slide source → une slide Oku hi-fi : le widget généré de son layout,
//! ses placeholders de texte dans les paramètres de ce widget, le reste en
//! éléments du slot libre `extras`, à sa place.
//!
//! | Source | Slide Oku |
//! |---|---|
//! | layout | `kind` = son widget généré ; sans layout lisible, `blank` (la base) |
//! | placeholder de texte du layout, à sa boîte, sans fond ni trait | le paramètre du placeholder : `rich` sur le style du placeholder de layout, `plain` (date, pied, numéro ; `{n}` pour le champ numéro) |
//! | autre item (placeholder déplacé, rempli ou sans paramètre compris) | éléments de `extras` ([`super::element`]), dans l'ordre de dessin |
//! | fond propre (autre que celui du layout) | premier élément de `extras` : une forme `rect` (ou une image) sur toute la page |
//! | layout sans widget (bornes du pack) | `blank` ; fond hérité, décor du master (s'il est montré) et du layout en premiers éléments de `extras` |
//! | décor du layout masqué (`showMasterSp="0"`) | `decor` à faux |
//! | slide masquée | perte `HiddenSlide` : le cœur n'a pas de slide masquée |
//! | notes | notes |
//!
//! Les paramètres sont dessinés sous `extras` : un placeholder de texte
//! que la source pose au-dessus d'un autre item passe dessous.

use std::collections::BTreeSet;

use oku_slides_core::{Namespace, ParamType, ParamValue, Slide, SlotMode, WidgetRegistry, qualify};

use super::element::{ElementCtx, elements};
use super::element_text::placeholder_text;
use super::pack::GeneratedPack;
use super::widget::{DECOR_PARAM, EXTRAS_PARAM, LayoutWidget, PAGE_NUMBER, PlaceholderParam, is_plain};
use crate::plan::ItemTarget;
use crate::report::{Loss, LossKind, Placement, SlideReport};
use crate::retrofit::text::{note, plain};
use crate::source::{
    Fill, Frame, Geometry, Item, ItemKind, Layout, Placeholder, SLIDE_NUMBER, Shape, SourceDeck, SourceSlide, TextBox,
};

/// Widget de base d'une slide sans layout lisible.
pub(super) const BLANK: &str = "blank";
/// Écart (pt source) sous lequel un placeholder de slide est à la boîte de son placeholder de layout.
const SAME_BOX: f64 = 0.5;

/// Une slide projetée.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Projected {
    /// La slide Oku, notes comprises.
    pub(super) slide: Slide,
    /// Son entrée au rapport.
    pub(super) report: SlideReport,
    /// Les assets que ses éléments dessinent.
    pub(super) assets: BTreeSet<String>,
    /// L'élément `text` de chaque item qui porte du texte (id d'item, id d'élément).
    pub(super) origins: Vec<(String, String)>,
    /// Ce que l'agent corrige pour chaque item projeté.
    ///
    /// Ses éléments, ou le paramètre qu'il remplit.
    pub(super) targets: Vec<(String, ItemTarget)>,
}

/// Ce dont la projection d'une slide a besoin.
#[derive(Debug, Clone, Copy)]
pub(super) struct SlideCx<'a> {
    /// La source.
    pub(super) deck: &'a SourceDeck,
    /// Le pack généré.
    pub(super) pack: &'a GeneratedPack,
    /// Le registre déclaré (base et pack).
    pub(super) registry: &'a WidgetRegistry,
    /// Le contexte des éléments.
    pub(super) elements: ElementCtx<'a>,
}

/// La slide Oku de `src`.
pub(super) fn project_slide(src: &SourceSlide, cx: &SlideCx<'_>) -> Projected {
    let id = format!("s{}", src.number);
    let layout = src.layout.as_deref().and_then(|l| cx.deck.layouts.iter().find(|x| x.id == l));
    let widget = layout.and_then(|l| cx.pack.widget(&l.id));
    let kind = widget.map_or_else(|| BLANK.to_owned(), |w| qualify(&Namespace::deck(), w.name.as_str()));
    let mut slide = Slide::new(id.clone(), &kind);
    let mut out = Out {
        id: id.clone(),
        rank: 0,
        extras: Vec::new(),
        assets: BTreeSet::new(),
        origins: Vec::new(),
        targets: Vec::new(),
    };
    let mut losses = Vec::new();
    if src.hidden {
        losses.push(Loss::slide(LossKind::HiddenSlide));
    }

    if let Some(back) = own_background(src, layout, widget.is_some(), cx) {
        out.place(&back, cx, &mut losses);
    }
    // Un layout sans widget (bornes du pack) : son décor et celui de son master, copiés en éléments.
    if let Some(l) = layout.filter(|_| widget.is_none() && src.show_layout_decor) {
        let master = cx.deck.masters.iter().find(|m| m.id == l.master).filter(|_| l.show_master_decor);
        for item in master.into_iter().flat_map(|m| &m.decor).chain(&l.decor) {
            out.place(item, cx, &mut losses);
        }
    }
    if let Some(w) = widget
        && !src.show_layout_decor
        && w.schema.param(DECOR_PARAM).is_some()
    {
        slide.params.set(DECOR_PARAM, ParamValue::Value(serde_json::Value::Bool(false)));
    }
    let mut filled = BTreeSet::new();
    for item in &src.items {
        let target = widget.zip(layout).and_then(|(w, l)| param_of(item, w, l, &filled));
        match target {
            Some((p, ph)) => {
                filled.insert(p.param.clone());
                if cx.elements.effects.contains(&item.id) {
                    note(&mut losses, Loss::on(&item.id, LossKind::Effects));
                }
                if let Some(value) = param_value(item, p, ph, cx, &mut losses) {
                    slide.params.set(&p.param, value);
                    out.targets.push((item.id.clone(), ItemTarget::Param(p.param.clone())));
                }
            }
            None => out.place(item, cx, &mut losses),
        }
    }
    if !out.extras.is_empty() {
        let slot = free_slot(cx.registry, &kind).unwrap_or(EXTRAS_PARAM);
        slide.params.set(slot, ParamValue::Slot(std::mem::take(&mut out.extras)));
    }
    if !src.notes.trim().is_empty() {
        slide.notes.clone_from(&src.notes);
    }
    let report = SlideReport {
        source: src.number,
        slide: id,
        template: kind,
        placement: Placement::Free,
        losses,
        fallbacks: Vec::new(),
        score: None,
    };
    Projected { slide, report, assets: out.assets, origins: out.origins, targets: out.targets }
}

/// Les éléments de `extras` en cours.
struct Out {
    id: String,
    rank: usize,
    extras: Vec<oku_slides_core::Element>,
    assets: BTreeSet<String>,
    origins: Vec<(String, String)>,
    targets: Vec<(String, ItemTarget)>,
}

impl Out {
    /// Ajoute les éléments de `item` (ids `s<numéro>-<rang>`) et ses pertes.
    fn place(&mut self, item: &Item, cx: &SlideCx<'_>, losses: &mut Vec<Loss>) {
        let (id, rank) = (&self.id, &mut self.rank);
        let placed = elements(item, &cx.elements, &mut || {
            *rank += 1;
            format!("{id}-{rank}")
        });
        self.extras.extend(placed.elements);
        self.assets.extend(placed.assets);
        self.origins.extend(placed.origins);
        self.targets.extend(placed.by_item.into_iter().map(|(item, ids)| (item, ItemTarget::Elements(ids))));
        for l in placed.losses {
            note(losses, l);
        }
    }
}

/// Le fond propre de `src`, s'il diffère de celui de son layout (sinon de
/// son master) : un rectangle sans trait sur toute la page Oku. Si son
/// widget ne dessine pas le fond hérité (`drawn` faux : layout sans widget),
/// ce fond-là à défaut de fond propre.
fn own_background(src: &SourceSlide, layout: Option<&Layout>, drawn: bool, cx: &SlideCx<'_>) -> Option<Item> {
    let master = layout.and_then(|l| cx.deck.masters.iter().find(|m| m.id == l.master));
    let inherited = layout.and_then(|l| l.background.as_ref()).or(master.map(|m| &m.background));
    let fill = match (&src.background, inherited) {
        (Fill::None, Some(f)) if !drawn => f,
        (Fill::None, _) => return None,
        (own, Some(f)) if drawn && own == f => return None,
        (own, _) => own,
    };
    if *fill == Fill::None {
        return None;
    }
    let geom = Geometry::Preset { name: "rect".to_owned(), adj: Default::default() };
    let shape = Shape { geom, fill: fill.clone(), stroke: None, text: None, connection: None };
    let frame: Frame = cx.elements.base.page.cover();
    Some(Item {
        id: format!("{}/fond", src.id),
        name: "fond".to_owned(),
        alt: None,
        frame,
        z: 0,
        role: None,
        placeholder: None,
        kind: ItemKind::Shape(Box::new(shape)),
    })
}

/// Le paramètre du widget `w` (layout `layout`) que remplit `item`, et le
/// placeholder de layout correspondant : un placeholder de texte lié à un
/// placeholder de texte du layout (par id, sinon par index) qui a un style
/// de niveau, à sa boîte, sans fond ni trait, dont le paramètre n'est pas
/// déjà rempli.
fn param_of<'w>(
    item: &Item,
    w: &'w LayoutWidget,
    layout: &'w Layout,
    filled: &BTreeSet<String>,
) -> Option<(&'w PlaceholderParam, &'w Placeholder)> {
    let link = item.placeholder.as_ref().filter(|_| item.role.is_some())?;
    let bare = match &item.kind {
        ItemKind::Text(_) => true,
        ItemKind::Shape(s) => s.fill == Fill::None && s.stroke.is_none(),
        _ => false,
    };
    if !bare {
        return None;
    }
    let p = w.params.iter().find(|p| link.layout.as_deref() == Some(p.placeholder.as_str())).or_else(|| {
        let idx = link.idx?;
        w.params.iter().find(|p| p.idx == Some(idx))
    })?;
    // Sans style de niveau, le widget pose le champ au style du thème : le texte va dans `extras`.
    let ph = layout.placeholders.iter().find(|ph| ph.id == p.placeholder && !ph.levels.is_empty())?;
    (!filled.contains(&p.param) && same_box(&item.frame, &ph.frame)).then_some((p, ph))
}

/// `a` et `b` sont-elles la même boîte, sans rotation ni miroir ?
fn same_box(a: &Frame, b: &Frame) -> bool {
    let near = |x: f64, y: f64| (x - y).abs() <= SAME_BOX;
    near(a.x, b.x) && near(a.y, b.y) && near(a.w, b.w) && near(a.h, b.h) && a.rot == 0.0 && !a.flip_h && !a.flip_v
}

/// La valeur du paramètre `p` pour le texte de `item` ; `None` s'il est vide
/// (ou, pour le numéro, s'il ne dit que le numéro : le défaut du widget).
fn param_value(
    item: &Item,
    p: &PlaceholderParam,
    ph: &Placeholder,
    cx: &SlideCx<'_>,
    losses: &mut Vec<Loss>,
) -> Option<ParamValue> {
    let tb = item.text().filter(|_| item.has_text())?;
    if is_plain(p.role) {
        let (text, lost) = plain(&numbered(tb));
        for l in lost {
            note(losses, Loss::on(&item.id, l));
        }
        return (!text.is_empty() && text != PAGE_NUMBER).then_some(ParamValue::Text(text));
    }
    let (paragraphs, lost) = placeholder_text(tb, ph.levels.first()?, &cx.elements);
    for l in lost {
        note(losses, Loss::on(&item.id, l));
    }
    (!paragraphs.is_empty()).then_some(ParamValue::Rich(paragraphs))
}

/// `tb` dont chaque champ numéro de slide dit `{n}` : le numéro de la page.
fn numbered(tb: &TextBox) -> TextBox {
    let mut tb = tb.clone();
    for r in tb.paragraphs.iter_mut().flat_map(|p| &mut p.runs) {
        if r.field.as_deref() == Some(SLIDE_NUMBER) {
            PAGE_NUMBER.clone_into(&mut r.text);
        }
    }
    tb
}

/// Le premier slot libre du widget `kind`.
fn free_slot<'r>(registry: &'r WidgetRegistry, kind: &str) -> Option<&'r str> {
    registry.get(kind)?.params.iter().find_map(|p| match &p.ty {
        ParamType::Slot { mode: SlotMode::Free, .. } => Some(p.name.as_str()),
        _ => None,
    })
}

#[cfg(test)]
#[path = "slide_tests.rs"]
mod slide_tests;
