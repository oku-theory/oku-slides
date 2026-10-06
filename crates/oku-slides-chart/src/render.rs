//! Rendu SVG. Le trait [`ChartRenderer`] isole le moteur : l'implémentation
//! v1 est [`ChartsRsRenderer`] ; une implémentation charming pourra partir du
//! même [`ChartModel`].

use charts_rs::{
    BarChart, ChartBase, HorizontalBarChart, LineChart, PieChart, RadarChart, RadarIndicator, ScatterChart, Series,
    SeriesCategory, WaterfallChart, WaterfallData, YAxisConfig,
};
use serde_json::json;

use snafu::ResultExt;

use crate::error::{RenderSnafu, Result};
use crate::header::{Header, append};
use crate::labels::ChartLabels;
use crate::model::*;
use crate::option::ChartOption;
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
        let (svg, header) = match &model.kind {
            ChartKind::Cartesian(c) if c.horizontal => {
                let series = c.series.iter().map(cartesian_series).collect();
                let mut chart = HorizontalBarChart::new_with_theme(series, c.categories.clone(), "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                self.series_colors(&mut chart.base, model, c.series.iter().map(|s| s.color.as_deref()));
                set_bounds(&mut axes, c.value_min, c.value_max);
                chart.y_axis_configs = axes;
                let header = self.header(&mut chart.base, model);
                (chart.svg(), header)
            }
            ChartKind::Cartesian(c) if c.series.iter().all(|s| s.mark == CartesianMark::Line) => {
                let series = c.series.iter().map(cartesian_series).collect();
                let mut chart = LineChart::new_with_theme(series, c.categories.clone(), "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                self.series_colors(&mut chart.base, model, c.series.iter().map(|s| s.color.as_deref()));
                set_bounds(&mut axes, c.value_min, c.value_max);
                chart.y_axis_configs = axes;
                chart.series_smooth = c.smooth;
                chart.series_fill = c.area;
                let header = self.header(&mut chart.base, model);
                (chart.svg(), header)
            }
            ChartKind::Cartesian(c) => {
                let series = c.series.iter().map(cartesian_series).collect();
                let mut chart = BarChart::new_with_theme(series, c.categories.clone(), "light");
                let mut axes = std::mem::take(&mut chart.y_axis_configs);
                self.frame(&mut chart.base, &mut axes, model, size);
                self.series_colors(&mut chart.base, model, c.series.iter().map(|s| s.color.as_deref()));
                set_bounds(&mut axes, c.value_min, c.value_max);
                chart.y_axis_configs = axes;
                chart.series_smooth = c.smooth;
                chart.series_fill = c.area;
                let header = self.header(&mut chart.base, model);
                (chart.svg(), header)
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
                let header = self.header(&mut chart.base, model);
                (chart.svg(), header)
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
                self.series_colors(&mut chart.base, model, s.series.iter().map(|ss| ss.color.as_deref()));
                set_bounds(&mut axes, s.y_min, s.y_max);
                chart.y_axis_configs = axes;
                self.axis_style(&mut chart.x_axis_config);
                chart.x_axis_config.axis_min = s.x_min.map(|v| v as f32);
                chart.x_axis_config.axis_max = s.x_max.map(|v| v as f32);
                let header = self.header(&mut chart.base, model);
                (chart.svg(), header)
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
                let header = self.header(&mut chart.base, model);
                (chart.svg(), header)
            }
            ChartKind::Waterfall(w) => return self.waterfall(w, model, size),
        };
        Ok(append(svg.context(RenderSnafu)?, &header))
    }

    /// Cascade : le WaterfallChart de charts-rs. Son axe n'est réglable que
    /// par `from_json` ; hausse, baisse et total prennent la palette.
    fn waterfall(&self, w: &Waterfall, model: &ChartModel, size: Size) -> Result<String> {
        let t = &self.theme;
        let mut axis = json!({
            "axis_font_size": t.axis_size,
            "axis_font_color": t.text_muted,
            "axis_stroke_color": t.axis,
        });
        if let Some(min) = w.value_min {
            axis["axis_min"] = json!(min);
        }
        if let Some(max) = w.value_max {
            axis["axis_max"] = json!(max);
        }
        let mut chart =
            WaterfallChart::from_json(&json!({ "y_axis_configs": [axis] }).to_string()).context(RenderSnafu)?;
        chart.data = w
            .steps
            .iter()
            .zip(&w.totals)
            .map(|(step, total)| match total {
                // Valeur 0 : charts-rs calcule la somme courante.
                true => WaterfallData { value: 0.0, is_total: true },
                false => WaterfallData { value: step.unwrap_or(0.0) as f32, is_total: false },
            })
            .collect();
        chart.x_axis_data = w.categories.clone();
        self.frame(&mut chart.base, &mut [], model, size);
        chart.label_show = w.label;
        let palette = |i: usize| self.palette(model).get(i).map(|c| color(c));
        if let Some(c) = palette(0) {
            chart.total_color = c;
        }
        if let Some(c) = palette(1) {
            chart.decrease_color = c;
        }
        if let Some(c) = palette(2) {
            chart.increase_color = c;
        }
        let header = self.header(&mut chart.base, model);
        Ok(append(chart.svg().context(RenderSnafu)?, &header))
    }

    /// Palette des séries : celle des options, sinon celle du thème.
    fn palette<'a>(&'a self, model: &'a ChartModel) -> &'a [String] {
        model.palette.as_deref().filter(|p| !p.is_empty()).unwrap_or(&self.theme.palette)
    }

    /// Couleur de chaque série : la sienne, sinon la palette (en boucle).
    fn series_colors<'a>(&self, base: &mut ChartBase, model: &ChartModel, own: impl Iterator<Item = Option<&'a str>>) {
        let palette = self.palette(model);
        base.series_colors = own
            .enumerate()
            .map(|(i, c)| {
                color(c.or_else(|| palette.get(i % palette.len().max(1)).map(String::as_str)).unwrap_or("#000"))
            })
            .collect();
    }

    /// En-tête (titre, sous-titre, légende) dessiné hors de charts-rs, une
    /// fois les séries et leurs couleurs posées ; à ajouter au SVG.
    fn header(&self, base: &mut ChartBase, model: &ChartModel) -> String {
        Header {
            title: &model.title,
            subtitle: &model.subtitle,
            title_align: model.title_align,
            legend: model.legend_show,
            legend_align: model.legend_align,
        }
        .place(base, &self.theme)
    }

    /// Taille, axes, séries et thème, communs à tous les graphiques.
    fn frame(&self, base: &mut ChartBase, axes: &mut [YAxisConfig], model: &ChartModel, size: Size) {
        let t = &self.theme;
        base.width = size.width;
        base.height = size.height;
        base.font_family = t.font_family.clone();
        base.is_light = true;
        base.background_color = t.background.as_deref().map(color).unwrap_or_else(transparent);

        // Hauteur que charts-rs réserve à l'axe X : son défaut (30) vaut pour
        // un texte de 12 ; elle suit la taille du thème avec la même marge.
        base.x_axis_height = t.axis_size + 18.0;

        base.x_axis_font_size = t.axis_size;
        base.x_axis_font_color = color(&t.text_muted);
        base.x_axis_stroke_color = color(&t.axis);
        base.grid_stroke_color = color(&t.grid);
        base.series_colors = self.palette(model).iter().map(|c| color(c)).collect();
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

/// Enregistre des polices (TTF/OTF) auprès de charts-rs. À appeler au
/// démarrage avec les mêmes fichiers que ceux chargés par le World Typst.
pub fn register_fonts(fonts: &[&[u8]]) -> Result<()> {
    Ok(charts_rs::add_fonts(fonts).context(RenderSnafu)?)
}
