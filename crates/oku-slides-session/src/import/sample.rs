//! L'échantillon d'une demande de widget du retrofit, pour l'agent.
//!
//! L'agent dessine le widget d'un motif d'après sa première occurrence :
//! chaque item source y est décrit court (sorte, géométrie d'une forme,
//! cadre en pt de la page, remplissage et contour en `#rrggbb` avec
//! l'emplacement du thème PowerPoint d'où ils viennent, nombre de
//! paragraphes). Le texte n'y est jamais : il vient du fichier importé, non
//! fiable, et l'agent le lit sur les slides. Les enfants d'un groupe suivent
//! le groupe, avec leur profondeur ; au-delà de [`MAX_SAMPLE_ITEMS`] items,
//! le reste est compté.

use serde_json::{Map, Value, json};

use oku_slides_import::source::{Color, Fill, Geometry, Item, ItemKind};

/// Items décrits par échantillon, au plus ; le reste est compté (`omitted`).
pub(crate) const MAX_SAMPLE_ITEMS: usize = 16;

/// L'échantillon des items `items` (première occurrence d'un motif).
pub(crate) fn sample(items: &[Item]) -> Value {
    let mut out = Vec::new();
    let mut omitted = 0;
    walk(items, 0, &mut out, &mut omitted);
    let mut v = json!({ "items": out });
    if omitted > 0 {
        v["omitted"] = json!(omitted);
    }
    v
}

/// Décrit `items` à la profondeur `depth` dans `out`, groupes d'abord puis leurs enfants.
fn walk(items: &[Item], depth: u32, out: &mut Vec<Value>, omitted: &mut usize) {
    for item in items {
        if out.len() < MAX_SAMPLE_ITEMS {
            out.push(describe(item, depth));
        } else {
            *omitted += 1;
        }
        if let ItemKind::Group { items: children, .. } = &item.kind {
            walk(children, depth + 1, out, omitted);
        }
    }
}

/// Une valeur en pt arrondie au dixième.
fn pt(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Une couleur : `#rrggbb` et, si elle vient du thème, son emplacement (`accent1`…).
fn color(c: &Color) -> Value {
    let mut v = json!({ "hex": c.value.to_hex() });
    if let Some(slot) = c.origin.slot() {
        v["slot"] = json!(slot);
    }
    v
}

/// Un remplissage peint ; `None` s'il ne peint rien.
fn fill(f: &Fill) -> Option<Value> {
    match f {
        Fill::None => None,
        Fill::Solid { color: c } => Some(color(c)),
        Fill::Gradient { stops, .. } => {
            let ends = [stops.first(), stops.last()];
            Some(json!({ "gradient": ends.into_iter().flatten().map(|s| color(&s.color)).collect::<Vec<_>>() }))
        }
        Fill::Picture { .. } => Some(json!("picture")),
    }
}

/// Le nom de la sorte de `kind`, comme le sérialise la source.
fn kind_name(kind: &ItemKind) -> &'static str {
    match kind {
        ItemKind::Text(_) => "text",
        ItemKind::Shape(_) => "shape",
        ItemKind::Picture(_) => "picture",
        ItemKind::Table(_) => "table",
        ItemKind::Chart(_) => "chart",
        ItemKind::Group { .. } => "group",
        ItemKind::SmartArt(_) => "smart_art",
        ItemKind::Opaque(_) => "opaque",
    }
}

/// La description courte d'un item à la profondeur `depth` (absente à 0).
fn describe(item: &Item, depth: u32) -> Value {
    let f = &item.frame;
    let mut v = Map::new();
    v.insert("kind".into(), json!(kind_name(&item.kind)));
    if depth > 0 {
        v.insert("depth".into(), json!(depth));
    }
    if let ItemKind::Shape(s) = &item.kind {
        if let Geometry::Preset { name, .. } = &s.geom {
            v.insert("geometry".into(), json!(name));
        }
        if let Some(paint) = fill(&s.fill) {
            v.insert("fill".into(), paint);
        }
        if let Some(stroke) = &s.stroke {
            v.insert("stroke".into(), color(&stroke.color));
        }
    }
    v.insert("frame".into(), json!([pt(f.x), pt(f.y), pt(f.w), pt(f.h)]));
    if let Some(tb) = item.text() {
        v.insert("paragraphs".into(), json!(tb.paragraphs.len()));
    }
    Value::Object(v)
}

#[cfg(test)]
#[path = "sample_tests.rs"]
mod sample_tests;
