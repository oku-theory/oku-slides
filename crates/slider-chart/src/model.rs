//! Normalisation : options ECharts → [`ChartModel`], indépendant du moteur.
//!
//! Tous les moteurs (charts-rs aujourd'hui, charming éventuellement) partent
//! de ce modèle. C'est ici que se résolvent `dataset` + `encode`, les données
//! en ligne et les valeurs par défaut d'ECharts.

use crate::error::{ChartError, InconsistentSnafu, InvalidOptionSnafu, Result};
use crate::labels::ChartLabels;
use crate::option::*;
use slider_i18n::Message;
use snafu::ResultExt;

#[derive(Debug, Clone, PartialEq)]
pub struct ChartModel {
    pub title: String,
    pub subtitle: String,
    pub title_align: HAlign,
    pub legend_show: bool,
    pub legend_align: HAlign,
    pub kind: ChartKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChartKind {
    Cartesian(Cartesian),
    Pie(Pie),
    Scatter(Scatter),
    Radar(RadarModel),
    Waterfall(Waterfall),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cartesian {
    /// Libellé de la colonne des catégories (en-tête du dataset).
    pub category_label: String,
    pub categories: Vec<String>,
    /// Barres horizontales : catégories sur l'axe Y.
    pub horizontal: bool,
    pub series: Vec<CartesianSeries>,
    pub smooth: bool,
    pub area: bool,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartesianMark {
    Bar,
    Line,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CartesianSeries {
    pub name: String,
    pub mark: CartesianMark,
    pub values: Vec<Option<f64>>,
    pub label: bool,
    /// Groupe d'empilement : les séries du même groupe s'empilent.
    pub stack: Option<String>,
}

/// Cascade : une variation par catégorie ; `None` = barre de total.
#[derive(Debug, Clone, PartialEq)]
pub struct Waterfall {
    pub category_label: String,
    pub categories: Vec<String>,
    pub name: String,
    pub steps: Vec<Option<f64>>,
    pub label: bool,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pie {
    pub name: String,
    pub name_label: String,
    pub slices: Vec<(String, Option<f64>)>,
    /// Rayons intérieur et extérieur, non résolus (dépendent de la taille).
    pub inner: Option<Length>,
    pub outer: Option<Length>,
    pub rose: bool,
    pub label: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scatter {
    pub series: Vec<ScatterSeries>,
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSeries {
    pub name: String,
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadarModel {
    pub indicator_label: String,
    /// Nom et maximum (résolu) de chaque axe.
    pub indicators: Vec<(String, f64)>,
    pub series: Vec<RadarSeries>,
    pub area: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadarSeries {
    pub name: String,
    pub values: Vec<Option<f64>>,
}

/// Parse et valide des options brutes (JSON produit par le LLM).
pub fn parse_option(json: &serde_json::Value) -> Result<ChartOption> {
    let option: ChartOption = serde_json::from_value(json.clone()).context(InvalidOptionSnafu)?;
    normalize(&option, ChartLabels::english())?;
    Ok(option)
}

/// Résout les options en modèle. Oku Slidesue si les options sont incohérentes.
/// `labels` nomme les séries qui n'ont pas de nom.
pub fn normalize(option: &ChartOption, labels: &ChartLabels) -> Result<ChartModel> {
    let first = option.series.first().ok_or_else(|| inconsistent(Message::new("chart-no-series")))?;
    let table = option.dataset.as_ref().map(Table::new).transpose()?;

    let kind = match first.kind {
        SeriesType::Bar | SeriesType::Line => ChartKind::Cartesian(cartesian(option, table.as_ref(), labels)?),
        SeriesType::Pie => ChartKind::Pie(pie(option, table.as_ref())?),
        SeriesType::Scatter => ChartKind::Scatter(scatter(option, table.as_ref(), labels)?),
        SeriesType::Radar => ChartKind::Radar(radar(option, table.as_ref())?),
        SeriesType::Waterfall => ChartKind::Waterfall(waterfall(option, table.as_ref(), labels)?),
    };

    let title = option.title.as_ref();
    let legend = option.legend.as_ref();
    let default_legend = !matches!(kind, ChartKind::Pie(_) | ChartKind::Waterfall(_));
    Ok(ChartModel {
        title: title.and_then(|t| t.text.clone()).unwrap_or_default(),
        subtitle: title.and_then(|t| t.subtext.clone()).unwrap_or_default(),
        title_align: title.and_then(|t| t.left).unwrap_or(HAlign::Center),
        legend_show: legend.and_then(|l| l.show).unwrap_or(default_legend),
        legend_align: legend.and_then(|l| l.left).unwrap_or(HAlign::Center),
        kind,
    })
}

fn inconsistent(reason: Message) -> ChartError {
    InconsistentSnafu { reason }.build().into()
}

/// Vue sur `dataset.source` : en-têtes + lignes de données.
pub(crate) struct Table<'a> {
    pub headers: Vec<String>,
    pub rows: &'a [Vec<Cell>],
}

impl<'a> Table<'a> {
    pub fn new(dataset: &'a Dataset) -> Result<Self> {
        let (header, rows) =
            dataset.source.split_first().ok_or_else(|| inconsistent(Message::new("chart-dataset-header")))?;
        Ok(Table { headers: header.iter().map(Cell::as_label).collect(), rows })
    }

    pub fn column(&self, r: &ColumnRef) -> Result<usize> {
        let index = match r {
            ColumnRef::Index(i) => *i,
            ColumnRef::Name(n) => self.headers.iter().position(|h| h == n).ok_or_else(|| {
                inconsistent(
                    Message::new("chart-unknown-column")
                        .arg("name", n.as_str())
                        .arg("columns", self.headers.join(", ")),
                )
            })?,
        };
        if index >= self.headers.len() {
            return Err(inconsistent(
                Message::new("chart-column-out-of-range").arg("index", index).arg("count", self.headers.len()),
            ));
        }
        Ok(index)
    }

    fn resolve(&self, r: Option<&ColumnRef>, default: usize) -> Result<usize> {
        match r {
            Some(r) => self.column(r),
            None if default < self.headers.len() => Ok(default),
            None => Err(inconsistent(Message::new("chart-no-default-column").arg("index", default))),
        }
    }

    fn labels(&self, col: usize) -> Vec<String> {
        self.rows.iter().map(|r| r.get(col).map(Cell::as_label).unwrap_or_default()).collect()
    }

    fn numbers(&self, col: usize) -> Vec<Option<f64>> {
        self.rows.iter().map(|r| r.get(col).and_then(Cell::as_f64)).collect()
    }
}

fn is_horizontal(option: &ChartOption) -> bool {
    let y_cat = option.y_axis.as_ref().is_some_and(|a| a.kind == Some(AxisType::Category) || a.data.is_some());
    let x_val = option.x_axis.as_ref().is_none_or(|a| a.kind != Some(AxisType::Category) && a.data.is_none());
    y_cat && x_val
}

/// Libellé et valeurs de la colonne des catégories : celle de la première
/// série dans `dataset` (0 par défaut), sinon `data` de l'axe des catégories.
fn categories(option: &ChartOption, table: Option<&Table>, horizontal: bool) -> Result<(String, Vec<String>)> {
    match table {
        Some(t) => {
            let cat_ref =
                option.series[0].encode.as_ref().and_then(|e| if horizontal { e.y.as_ref() } else { e.x.as_ref() });
            let cat_col = t.resolve(cat_ref, 0)?;
            Ok((t.headers[cat_col].clone(), t.labels(cat_col)))
        }
        None => {
            let cat_axis = if horizontal { option.y_axis.as_ref() } else { option.x_axis.as_ref() };
            let cats = cat_axis
                .and_then(|a| a.data.clone())
                .ok_or_else(|| inconsistent(Message::new("chart-category-data")))?;
            Ok((String::new(), cats))
        }
    }
}

/// Nom et valeurs de la série `i` sur un axe de catégories : ses données en
/// ligne, sinon sa colonne de `dataset` (i + 1 par défaut).
fn series_values(
    s: &Series,
    i: usize,
    table: Option<&Table>,
    horizontal: bool,
    labels: &ChartLabels,
) -> Result<(String, Vec<Option<f64>>)> {
    match (table, &s.data) {
        (_, Some(data)) => {
            let values = data
                .iter()
                .map(|d| match d {
                    DataItem::Value(v) => Ok(Some(*v)),
                    DataItem::Null => Ok(None),
                    DataItem::Named(NamedItem { value: ItemValue::Number(v), .. }) => Ok(Some(*v)),
                    _ => Err(inconsistent(Message::new("chart-series-numbers").arg("index", i))),
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((s.name.clone().unwrap_or_else(|| labels.series(i)), values))
        }
        (Some(t), None) => {
            let val_ref = s.encode.as_ref().and_then(|e| if horizontal { e.x.as_ref() } else { e.y.as_ref() });
            let col = t.resolve(val_ref, i + 1)?;
            Ok((s.name.clone().unwrap_or_else(|| t.headers[col].clone()), t.numbers(col)))
        }
        (None, None) => Err(inconsistent(Message::new("chart-series-empty").arg("index", i))),
    }
}

fn cartesian(option: &ChartOption, table: Option<&Table>, labels: &ChartLabels) -> Result<Cartesian> {
    let horizontal = is_horizontal(option);
    let val_axis = if horizontal { option.x_axis.as_ref() } else { option.y_axis.as_ref() };
    let (category_label, categories) = categories(option, table, horizontal)?;

    let mut series = Vec::with_capacity(option.series.len());
    for (i, s) in option.series.iter().enumerate() {
        let mark = match s.kind {
            SeriesType::Bar => CartesianMark::Bar,
            SeriesType::Line => CartesianMark::Line,
            other => {
                return Err(inconsistent(Message::new("chart-mixed-series").arg("kind", format!("{other:?}"))));
            }
        };
        if horizontal && mark == CartesianMark::Line {
            return Err(inconsistent(Message::new("chart-horizontal-lines")));
        }
        let (name, values) = series_values(s, i, table, horizontal, labels)?;
        series.push(CartesianSeries {
            name,
            mark,
            values,
            label: s.label.as_ref().and_then(|l| l.show).unwrap_or(false),
            stack: s.stack.clone(),
        });
    }

    Ok(Cartesian {
        category_label,
        categories,
        horizontal,
        smooth: option.series.iter().any(|s| s.smooth == Some(true)),
        area: option.series.iter().any(|s| s.area_style.is_some()),
        value_min: val_axis.and_then(|a| a.min),
        value_max: val_axis.and_then(|a| a.max),
        series,
    })
}

/// Cascade : toujours verticale, une seule série ; les valeurs s'affichent
/// sauf `label.show: false`.
fn waterfall(option: &ChartOption, table: Option<&Table>, labels: &ChartLabels) -> Result<Waterfall> {
    let s = single(option, "cascade")?;
    let (category_label, categories) = categories(option, table, false)?;
    let (name, steps) = series_values(s, 0, table, false, labels)?;
    let val_axis = option.y_axis.as_ref();
    Ok(Waterfall {
        category_label,
        categories,
        name,
        steps,
        label: s.label.as_ref().and_then(|l| l.show).unwrap_or(true),
        value_min: val_axis.and_then(|a| a.min),
        value_max: val_axis.and_then(|a| a.max),
    })
}

fn single<'a>(option: &'a ChartOption, what: &str) -> Result<&'a Series> {
    match option.series.as_slice() {
        [s] => Ok(s),
        _ => Err(inconsistent(Message::new("chart-single-series").arg("kind", what))),
    }
}

fn pie(option: &ChartOption, table: Option<&Table>) -> Result<Pie> {
    let s = single(option, "camembert")?;
    let (name_label, slices, default_name) = match (table, &s.data) {
        (_, Some(data)) => {
            let slices = data
                .iter()
                .map(|d| match d {
                    DataItem::Named(NamedItem { name, value: ItemValue::Number(v) }) => Ok((name.clone(), Some(*v))),
                    _ => Err(inconsistent(Message::new("chart-pie-item"))),
                })
                .collect::<Result<Vec<_>>>()?;
            (String::new(), slices, String::new())
        }
        (Some(t), None) => {
            let enc = s.encode.as_ref();
            let name_col = t.resolve(enc.and_then(|e| e.item_name.as_ref()), 0)?;
            let value_col = t.resolve(enc.and_then(|e| e.value.as_ref()), 1)?;
            let slices = t.labels(name_col).into_iter().zip(t.numbers(value_col)).collect();
            (t.headers[name_col].clone(), slices, t.headers[value_col].clone())
        }
        (None, None) => return Err(inconsistent(Message::new("chart-pie-empty"))),
    };
    let (inner, outer) = match &s.radius {
        None => (None, None),
        Some(Radius::Single(r)) => (None, Some(r.clone())),
        Some(Radius::Pair([i, o])) => (Some(i.clone()), Some(o.clone())),
    };
    Ok(Pie {
        name: s.name.clone().unwrap_or(default_name),
        name_label,
        slices,
        inner,
        outer,
        rose: s.rose_type.as_ref().is_some_and(RoseType::enabled),
        label: s.label.as_ref().and_then(|l| l.show).unwrap_or(true),
    })
}

fn scatter(option: &ChartOption, table: Option<&Table>, labels: &ChartLabels) -> Result<Scatter> {
    let mut series = Vec::new();
    for (i, s) in option.series.iter().enumerate() {
        if s.kind != SeriesType::Scatter {
            return Err(inconsistent(Message::new("chart-scatter-mixed")));
        }
        let (name, points) = match (table, &s.data) {
            (_, Some(data)) => {
                let points = data
                    .iter()
                    .filter_map(|d| match d {
                        DataItem::Point(p) if p.len() == 2 => Some(Ok((p[0], p[1]))),
                        DataItem::Null => None,
                        _ => Some(Err(inconsistent(Message::new("chart-scatter-point").arg("index", i)))),
                    })
                    .collect::<Result<Vec<_>>>()?;
                (s.name.clone().unwrap_or_else(|| labels.series(i)), points)
            }
            (Some(t), None) => {
                let enc = s.encode.as_ref();
                let xc = t.resolve(enc.and_then(|e| e.x.as_ref()), 0)?;
                let yc = t.resolve(enc.and_then(|e| e.y.as_ref()), i + 1)?;
                let points = t.numbers(xc).into_iter().zip(t.numbers(yc)).filter_map(|(x, y)| Some((x?, y?))).collect();
                (s.name.clone().unwrap_or_else(|| t.headers[yc].clone()), points)
            }
            (None, None) => return Err(inconsistent(Message::new("chart-series-empty").arg("index", i))),
        };
        series.push(ScatterSeries { name, points });
    }
    let x = option.x_axis.as_ref();
    let y = option.y_axis.as_ref();
    Ok(Scatter {
        series,
        x_min: x.and_then(|a| a.min),
        x_max: x.and_then(|a| a.max),
        y_min: y.and_then(|a| a.min),
        y_max: y.and_then(|a| a.max),
    })
}

fn radar(option: &ChartOption, table: Option<&Table>) -> Result<RadarModel> {
    let s = single(option, "radar")?;
    let declared = option.radar.as_ref().map(|r| r.indicator.as_slice()).unwrap_or(&[]);

    let (indicator_label, names, series) = match (table, &s.data) {
        (_, Some(data)) => {
            if declared.is_empty() {
                return Err(inconsistent(Message::new("chart-radar-indicators")));
            }
            let series = data
                .iter()
                .map(|d| match d {
                    DataItem::Named(NamedItem { name, value: ItemValue::Values(v) }) => {
                        Ok(RadarSeries { name: name.clone(), values: v.iter().copied().map(Some).collect() })
                    }
                    _ => Err(inconsistent(Message::new("chart-radar-item"))),
                })
                .collect::<Result<Vec<_>>>()?;
            let names = declared.iter().map(|d| d.name.clone()).collect();
            (String::new(), names, series)
        }
        // Avec dataset : colonne 0 = axes, chaque autre colonne = un polygone.
        (Some(t), None) => {
            let series = (1..t.headers.len())
                .map(|c| RadarSeries { name: t.headers[c].clone(), values: t.numbers(c) })
                .collect();
            (t.headers[0].clone(), t.labels(0), series)
        }
        (None, None) => return Err(inconsistent(Message::new("chart-radar-empty"))),
    };

    for rs in &series {
        if rs.values.len() != names.len() {
            return Err(inconsistent(
                Message::new("chart-radar-values")
                    .arg("name", rs.name.as_str())
                    .arg("values", rs.values.len())
                    .arg("axes", names.len()),
            ));
        }
    }

    let indicators = names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let declared_max = declared.iter().find(|d| d.name == name).and_then(|d| d.max);
            let max = declared_max.unwrap_or_else(|| {
                let m = series.iter().filter_map(|s| s.values[i]).fold(0.0_f64, f64::max);
                nice_ceiling(m)
            });
            (name, max)
        })
        .collect();

    Ok(RadarModel { indicator_label, indicators, series, area: s.area_style.is_some() })
}

/// Arrondit au-dessus vers 1, 2, 2.5 ou 5 × 10ⁿ.
fn nice_ceiling(v: f64) -> f64 {
    if v <= 0.0 {
        return 1.0;
    }
    let exp = v.log10().floor();
    let base = 10f64.powf(exp);
    let f = v / base;
    let nice = [1.0, 2.0, 2.5, 5.0, 10.0].into_iter().find(|n| f <= *n).unwrap_or(10.0);
    nice * base
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
