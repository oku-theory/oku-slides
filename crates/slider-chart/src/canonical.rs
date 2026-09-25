//! Forme canonique stockée dans `deck.json` : toutes les données dans
//! `dataset`, pour que la grille puisse les éditer.
//!
//! Le LLM peut écrire des données en ligne (`xAxis.data`, `series.data`),
//! très courant dans les exemples ECharts ; elles sont déplacées dans
//! `dataset` avant stockage. En forme canonique, une série cartésienne n'a ni
//! `encode` ni `name` : la série i lit la colonne i + 1 et prend son nom de
//! l'en-tête, donc renommer une colonne dans la grille renomme la série.

use crate::error::Result;
use crate::model::*;
use crate::option::*;

/// Déplace les données en ligne dans `dataset`. Sans effet si les options
/// utilisent déjà `dataset`.
pub fn canonicalize(option: &ChartOption) -> Result<ChartOption> {
    let model = normalize(option)?;
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
            let mut header = vec![text(&or(&c.category_label, "Catégorie"))];
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
            let value_header = s.name.take().unwrap_or_else(|| or(&p.name, "Valeur"));
            let header = vec![text(&or(&p.name_label, "Nom")), text(&value_header)];
            let rows = p.slices.iter().map(|(n, v)| vec![text(n), num(*v)]);
            std::iter::once(header).chain(rows).collect()
        }
        ChartKind::Scatter(sc) => {
            // Une paire de colonnes (x, y) par série.
            let height = sc.series.iter().map(|s| s.points.len()).max().unwrap_or(0);
            let mut header = Vec::new();
            for s in &sc.series {
                header.push(text(&format!("{} (x)", s.name)));
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
            let mut header = vec![text(&or(&r.indicator_label, "Axe"))];
            header.extend(r.series.iter().map(|s| text(&s.name)));
            let rows = r.indicators.iter().enumerate().map(|(i, (name, _))| {
                let mut row = vec![text(name)];
                row.extend(r.series.iter().map(|s| num(s.values[i])));
                row
            });
            out.series[0].data = None;
            std::iter::once(header).chain(rows).collect()
        }
    };
    out.dataset = Some(Dataset { source });
    normalize(&out)?;
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
    let template = option.series.last().cloned().expect("au moins une série");
    option.series.resize(wanted, template);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_ops::ChartDataOp;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn opt(v: serde_json::Value) -> ChartOption {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn inline_cartesian_moves_to_dataset_with_same_model() {
        let inline = opt(json!({
            "xAxis": { "type": "category", "data": ["Jan", "Fév", "Mar"] },
            "yAxis": { "type": "value" },
            "series": [
                { "type": "bar", "name": "Ventes", "data": [10, 12, null] },
                { "type": "line", "name": "Objectif", "data": [11, 11, 11] }
            ]
        }));
        let canon = canonicalize(&inline).unwrap();
        assert!(canon.series.iter().all(|s| s.data.is_none()));
        let before = normalize(&inline).unwrap();
        let after = normalize(&canon).unwrap();
        let (ChartKind::Cartesian(b), ChartKind::Cartesian(a)) = (&before.kind, &after.kind) else {
            panic!("cartésien attendu")
        };
        assert_eq!(a.categories, b.categories);
        assert_eq!(a.series, b.series);
    }

    /// Noms et valeurs, sans les libellés de colonnes ajoutés par la canonisation.
    fn values(kind: &ChartKind) -> Vec<(String, Vec<Option<f64>>)> {
        match kind {
            ChartKind::Cartesian(c) => c.series.iter().map(|s| (s.name.clone(), s.values.clone())).collect(),
            ChartKind::Pie(p) => p.slices.iter().map(|(n, v)| (n.clone(), vec![*v])).collect(),
            ChartKind::Scatter(sc) => sc
                .series
                .iter()
                .map(|s| (s.name.clone(), s.points.iter().flat_map(|(x, y)| [Some(*x), Some(*y)]).collect()))
                .collect(),
            ChartKind::Radar(r) => r
                .series
                .iter()
                .map(|s| (s.name.clone(), s.values.clone()))
                .chain(r.indicators.iter().map(|(n, m)| (n.clone(), vec![Some(*m)])))
                .collect(),
        }
    }

    #[test]
    fn inline_pie_scatter_radar_keep_their_values() {
        for v in [
            json!({ "series": [{ "type": "pie", "name": "Parts",
                "data": [{ "name": "A", "value": 3 }, { "name": "B", "value": 1 }] }] }),
            json!({ "series": [{ "type": "scatter", "name": "Mesures", "data": [[1, 2], [3, 4]] }] }),
            json!({ "radar": { "indicator": [{ "name": "Vitesse", "max": 10 }, { "name": "Coût" }] },
                "series": [{ "type": "radar", "data": [{ "name": "Offre A", "value": [7, 4] }] }] }),
        ] {
            let o = opt(v);
            let canon = canonicalize(&o).unwrap();
            assert!(canon.dataset.is_some());
            assert_eq!(values(&normalize(&canon).unwrap().kind), values(&normalize(&o).unwrap().kind));
        }
    }

    #[test]
    fn renaming_a_column_renames_the_series() {
        let mut o = canonicalize(&opt(json!({
            "xAxis": { "data": ["Jan"] },
            "series": [{ "type": "bar", "name": "Ventes", "data": [10] }]
        })))
        .unwrap();
        ChartDataOp::SetCell { row: 0, col: 1, value: Cell::Text("CA".into()) }
            .apply(o.dataset.as_mut().unwrap())
            .unwrap();
        let ChartKind::Cartesian(c) = normalize(&o).unwrap().kind else { panic!() };
        assert_eq!(c.series[0].name, "CA");
    }

    #[test]
    fn adding_a_column_adds_a_series() {
        let mut o = opt(json!({
            "dataset": { "source": [["Mois", "Ventes"], ["Jan", 10]] },
            "xAxis": { "type": "category" },
            "series": [{ "type": "line" }]
        }));
        ChartDataOp::InsertColumn { at: 2, cells: vec![Cell::Text("Objectif".into()), Cell::Number(12.0)] }
            .apply(o.dataset.as_mut().unwrap())
            .unwrap();
        sync_series_with_columns(&mut o);
        assert_eq!(o.series.len(), 2);
        assert!(normalize(&o).is_ok());
    }
}
