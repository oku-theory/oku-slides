//! Forme canonique stockée dans `deck.json` : toutes les données dans
//! `dataset`, pour que la grille puisse les éditer.
//!
//! Le LLM peut écrire des données en ligne (`xAxis.data`, `series.data`),
//! très courant dans les exemples ECharts ; elles sont déplacées dans
//! `dataset` avant stockage. En forme canonique, une série cartésienne n'a ni
//! `encode` ni `name` : la série i lit la colonne i + 1 et prend son nom de
//! l'en-tête, donc renommer une colonne dans la grille renomme la série.

use crate::error::Result;
use crate::labels::ChartLabels;
use crate::model::*;
use crate::option::*;

/// Déplace les données en ligne dans `dataset`. Sans effet si les options
/// utilisent déjà `dataset`. Les en-têtes manquants viennent de `labels`.
pub fn canonicalize(option: &ChartOption, labels: &ChartLabels) -> Result<ChartOption> {
    let model = normalize(option, labels)?;
    let inline = option.series.iter().any(|s| s.data.is_some());
    if option.dataset.is_some() && !inline {
        return Ok(option.clone());
    }
    let mut out = option.clone();
    let text = |s: &str| Cell::Text(s.to_owned());
    let num = |v: Option<f64>| v.map_or(Cell::Null, Cell::Number);
    let or = |s: &str, d: &str| {
        if s.is_empty() { d.to_owned() } else { s.to_owned() }
    };

    let source: Vec<Vec<Cell>> = match &model.kind {
        ChartKind::Cartesian(c) => {
            let mut header = vec![text(&or(&c.category_label, labels.category()))];
            header.extend(c.series.iter().map(|s| text(&s.name)));
            let rows = c.categories.iter().enumerate().map(|(i, cat)| {
                let mut row = vec![text(cat)];
                row.extend(c.series.iter().map(|s| num(s.values.get(i).copied().flatten())));
                row
            });
            for axis in [&mut out.x_axis, &mut out.y_axis].into_iter().flatten() {
                axis.data = None;
                if axis.kind.is_none() {
                    axis.kind = Some(AxisType::Value);
                }
            }
            let cat_axis = if c.horizontal { &mut out.y_axis } else { &mut out.x_axis };
            cat_axis.get_or_insert_with(empty_axis).kind = Some(AxisType::Category);
            for s in &mut out.series {
                s.data = None;
                s.encode = None;
                s.name = None;
            }
            std::iter::once(header).chain(rows).collect()
        }
        ChartKind::Pie(p) => {
            let s = &mut out.series[0];
            s.data = None;
            s.encode = None;
            let value_header = s.name.take().unwrap_or_else(|| or(&p.name, labels.value()));
            let header = vec![text(&or(&p.name_label, labels.name())), text(&value_header)];
            let rows = p.slices.iter().map(|(n, v)| vec![text(n), num(*v)]);
            std::iter::once(header).chain(rows).collect()
        }
        ChartKind::Scatter(sc) => {
            // Une paire de colonnes (x, y) par série.
            let height = sc.series.iter().map(|s| s.points.len()).max().unwrap_or(0);
            let mut header = Vec::new();
            for s in &sc.series {
                header.push(text(&labels.x_of(&s.name)));
                header.push(text(&s.name));
            }
            let rows = (0..height).map(|i| {
                sc.series
                    .iter()
                    .flat_map(|s| match s.points.get(i) {
                        Some((x, y)) => [Cell::Number(*x), Cell::Number(*y)],
                        None => [Cell::Null, Cell::Null],
                    })
                    .collect()
            });
            for (i, s) in out.series.iter_mut().enumerate() {
                s.data = None;
                s.name = None;
                s.encode = Some(Encode {
                    x: Some(ColumnRef::Index(2 * i)),
                    y: Some(ColumnRef::Index(2 * i + 1)),
                    item_name: None,
                    value: None,
                });
            }
            std::iter::once(header).chain(rows).collect()
        }
        ChartKind::Radar(r) => {
            let mut header = vec![text(&or(&r.indicator_label, labels.axis()))];
            header.extend(r.series.iter().map(|s| text(&s.name)));
            let rows = r.indicators.iter().enumerate().map(|(i, (name, _))| {
                let mut row = vec![text(name)];
                row.extend(r.series.iter().map(|s| num(s.values[i])));
                row
            });
            out.series[0].data = None;
            std::iter::once(header).chain(rows).collect()
        }
        ChartKind::Waterfall(w) => {
            let header = vec![text(&or(&w.category_label, labels.category())), text(&w.name)];
            let rows =
                w.categories.iter().enumerate().map(|(i, cat)| vec![text(cat), num(w.steps.get(i).copied().flatten())]);
            for axis in [&mut out.x_axis, &mut out.y_axis].into_iter().flatten() {
                axis.data = None;
            }
            out.x_axis.get_or_insert_with(empty_axis).kind = Some(AxisType::Category);
            let s = &mut out.series[0];
            s.data = None;
            s.encode = None;
            s.name = None;
            std::iter::once(header).chain(rows).collect()
        }
    };
    out.dataset = Some(Dataset { source });
    normalize(&out, labels)?;
    Ok(out)
}

fn empty_axis() -> Axis {
    Axis { kind: None, data: None, min: None, max: None }
}

/// Après ajout ou retrait de colonnes dans la grille : ajuste le nombre de
/// séries cartésiennes (forme canonique) au nombre de colonnes de valeurs.
/// Une série ajoutée reprend le type de la dernière.
pub fn sync_series_with_columns(option: &mut ChartOption) {
    let Some(width) = option.dataset.as_ref().and_then(|d| d.source.first()).map(Vec::len) else {
        return;
    };
    let canonical_cartesian = option
        .series
        .iter()
        .all(|s| matches!(s.kind, SeriesType::Bar | SeriesType::Line) && s.encode.is_none() && s.data.is_none());
    if !canonical_cartesian || option.series.is_empty() {
        return;
    }
    let wanted = width.saturating_sub(1).max(1);
    // `series` n'est pas vide (vérifié plus haut).
    if let Some(template) = option.series.last().cloned() {
        option.series.resize(wanted, template);
    }
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod canonical_tests;
