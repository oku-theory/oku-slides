//! Rendu SVG. Le trait [`ChartRenderer`] isole le moteur : l'implémentation
//! v1 est [`ChartsRsRenderer`] ; une implémentation charming pourra partir du
//! même [`ChartModel`].

use charts_rs::{
    Align, BarChart, ChartBase, HorizontalBarChart, LineChart, PieChart, RadarChart, RadarIndicator, ScatterChart,
    Series, SeriesCategory, YAxisConfig,
};

use snafu::ResultExt;

use crate::error::{RenderSnafu, Result};
use crate::labels::ChartLabels;
use crate::model::*;
use crate::option::{ChartOption, HAlign};
use crate::theme::{ChartTheme, color, transparent};

/// Taille du cadre du graphique, en points (1 pt Typst = 1 unité SVG).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

pub trait ChartRenderer: Send + Sync {
    /// Rend les options en SVG autonome, aux dimensions demandées.
    fn render(&self, option: &ChartOption, size: Size) -> Result<String>;
}

/// Moteur v1 : charts-rs, Rust pur.
#[derive(Debug, Clone, Default)]
pub struct ChartsRsRenderer {
    pub theme: ChartTheme,
    /// Noms des séries qui n'en ont pas.
    pub labels: ChartLabels,
}

impl ChartsRsRenderer {
    pub fn new(theme: ChartTheme) -> Self {
        Self { theme, labels: ChartLabels::default() }
    }

    pub fn render_model(&self, model: &ChartModel, size: Size) -> Result<String> {
        let svg = match &model.kind {
            ChartKind::Cartesian(c) if c.horizontal => {
                let series = c.series.iter().map(cartesian_series).collect();
                let mut chart = HorizontalBarChart::new_with_theme(series, c.categories.clone(), "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                set_bounds(&mut axes, c.value_min, c.value_max);
                chart.y_axis_configs = axes;
                chart.svg()
            }
            ChartKind::Cartesian(c) if c.series.iter().all(|s| s.mark == CartesianMark::Line) => {
                let series = c.series.iter().map(cartesian_series).collect();
                let mut chart = LineChart::new_with_theme(series, c.categories.clone(), "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                set_bounds(&mut axes, c.value_min, c.value_max);
                chart.y_axis_configs = axes;
                chart.series_smooth = c.smooth;
                chart.series_fill = c.area;
                chart.svg()
            }
            ChartKind::Cartesian(c) => {
                let series = c.series.iter().map(cartesian_series).collect();
                let mut chart = BarChart::new_with_theme(series, c.categories.clone(), "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                set_bounds(&mut axes, c.value_min, c.value_max);
                chart.y_axis_configs = axes;
                chart.series_smooth = c.smooth;
                chart.series_fill = c.area;
                chart.svg()
            }
            ChartKind::Pie(p) => {
                // Une série charts-rs par part (≠ ECharts : une série, n parts).
                let series =
                    p.slices.iter().map(|(name, v)| Series::new(name.clone(), vec![v.unwrap_or(0.0) as f32])).collect();
                let mut chart = PieChart::new_with_theme(series, "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                chart.y_axis_configs = axes;
                // Défauts charts-rs : rose + trou de 40. Défauts ECharts : ni l'un ni l'autre.
                let reference = f64::from(size.width.min(size.height)) / 2.0;
                chart.rose_type = Some(p.rose);
                chart.inner_radius = p.inner.as_ref().and_then(|l| l.resolve(reference)).unwrap_or(0.0) as f32;
                chart.radius = p.outer.as_ref().and_then(|l| l.resolve(reference)).unwrap_or(reference * 0.75) as f32;
                chart.border_radius = Some(0.0);
                if !p.label {
                    chart.series_label_formatter = " ".into();
                }
                chart.svg()
            }
            ChartKind::Scatter(s) => {
                let series = s
                    .series
                    .iter()
                    .map(|ss| {
                        let flat = ss.points.iter().flat_map(|(x, y)| [*x as f32, *y as f32]).collect();
                        Series::new(ss.name.clone(), flat)
                    })
                    .collect();
                let mut chart = ScatterChart::new_with_theme(series, "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                set_bounds(&mut axes, s.y_min, s.y_max);
                chart.y_axis_configs = axes;
                self.axis_style(&mut chart.x_axis_config);
                chart.x_axis_config.axis_min = s.x_min.map(|v| v as f32);
                chart.x_axis_config.axis_max = s.x_max.map(|v| v as f32);
                chart.svg()
            }
            ChartKind::Radar(r) => {
                let series = r
                    .series
                    .iter()
                    .map(|rs| {
                        let values = rs.values.iter().map(|v| v.map(|x| x as f32)).collect();
                        Series::new_nullable(rs.name.clone(), values)
                    })
                    .collect();
                let indicators = r
                    .indicators
                    .iter()
                    .map(|(name, max)| RadarIndicator { name: name.clone(), max: *max as f32 })
                    .collect();
                let mut chart = RadarChart::new_with_theme(series, indicators, "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                chart.y_axis_configs = axes;
                chart.series_fill = r.area;
                chart.svg()
            }
        };
        Ok(svg.context(RenderSnafu)?)
    }

    /// Taille, titre, légende et thème, communs à tous les graphiques.
    fn frame(&self, base: &mut ChartBase, axes: &mut [YAxisConfig], model: &ChartModel, size: Size) {
        let t = &self.theme;
        base.width = size.width;
        base.height = size.height;
        base.font_family = t.font_family.clone();
        base.is_light = true;
        base.background_color = t.background.as_deref().map(color).unwrap_or_else(transparent);

        base.title_text = model.title.clone();
        base.title_align = align(model.title_align);
        base.title_font_size = t.title_size;
        base.title_font_color = color(&t.text);
        base.sub_title_text = model.subtitle.clone();
        base.sub_title_align = align(model.title_align);
        base.sub_title_font_size = t.subtitle_size;
        base.sub_title_font_color = color(&t.text_muted);

        base.legend_show = Some(model.legend_show);
        base.legend_align = align(model.legend_align);
        base.legend_font_size = t.legend_size;
        base.legend_font_color = color(&t.text);
        if model.legend_show && !model.title.is_empty() {
            // Sous le titre plutôt que par-dessus.
            let mut m = base.legend_margin.unwrap_or_default();
            m.top = base.title_height + if model.subtitle.is_empty() { 0.0 } else { base.sub_title_height };
            base.legend_margin = Some(m);
        }

        base.x_axis_font_size = t.axis_size;
        base.x_axis_font_color = color(&t.text_muted);
        base.x_axis_stroke_color = color(&t.axis);
        base.grid_stroke_color = color(&t.grid);
        base.series_colors = t.palette.iter().map(|c| color(c)).collect();
        base.series_label_font_size = t.label_size;
        base.series_label_font_color = color(&t.text);
        base.series_stroke_width = t.stroke_width;
        // Le symbole des points reprend le fond : transparent par défaut.
        base.series_symbol = Some(charts_rs::Symbol::Circle(t.stroke_width, Some(base.background_color)));
        for a in axes.iter_mut() {
            self.axis_style(a);
        }
    }

    fn axis_style(&self, a: &mut YAxisConfig) {
        a.axis_font_size = self.theme.axis_size;
        a.axis_font_color = color(&self.theme.text_muted);
        a.axis_stroke_color = color(&self.theme.axis);
    }
}

impl ChartRenderer for ChartsRsRenderer {
    fn render(&self, option: &ChartOption, size: Size) -> Result<String> {
        self.render_model(&normalize(option, &self.labels)?, size)
    }
}

fn cartesian_series(s: &CartesianSeries) -> Series {
    let mut out = Series::new_nullable(s.name.clone(), s.values.iter().map(|v| v.map(|x| x as f32)).collect());
    out.label_show = s.label;
    out.stack.clone_from(&s.stack);
    if s.mark == CartesianMark::Line {
        out.category = Some(SeriesCategory::Line);
    }
    out
}

fn set_bounds(axes: &mut [YAxisConfig], min: Option<f64>, max: Option<f64>) {
    if let Some(a) = axes.first_mut() {
        a.axis_min = min.map(|v| v as f32);
        a.axis_max = max.map(|v| v as f32);
    }
}

fn align(a: HAlign) -> Align {
    match a {
        HAlign::Left => Align::Left,
        HAlign::Center => Align::Center,
        HAlign::Right => Align::Right,
    }
}

/// Enregistre des polices (TTF/OTF) auprès de charts-rs. À appeler au
/// démarrage avec les mêmes fichiers que ceux chargés par le World Typst.
pub fn register_fonts(fonts: &[&[u8]]) -> Result<()> {
    Ok(charts_rs::add_fonts(fonts).context(RenderSnafu)?)
}
