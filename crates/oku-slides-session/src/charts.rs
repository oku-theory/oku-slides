//! Graphiques reçus dans des ops (de l'interface ou d'un LLM) : rangés sous
//! forme canonique avant d'entrer dans le deck. Leurs données passent dans
//! `dataset`, où la grille les édite ; les en-têtes qui manquent sont écrits
//! dans la langue de l'utilisateur.
//!
//! Un graphique est un paramètre `host(chart)` d'un widget du registre.
//! `Session::set_chart_kind` redessine un graphique sous un autre type.

use oku_slides_chart::{ChartLabels, ChartOption, canonicalize};
use oku_slides_core::{Deck, Element, HostType, Op, ParamValue, Params, WidgetRegistry, WidgetSchema};
use snafu::{OptionExt, ResultExt};

use crate::error::{ChartSnafu, NoChartSnafu, NoSlideSnafu, ResponseSnafu};
use crate::{Result, Session};

/// Canonicalise les graphiques des éléments et slides ajoutés ou remplacés.
pub(crate) fn canonicalize_charts(ops: &mut [Op], labels: &ChartLabels, registry: &WidgetRegistry) {
    for op in ops {
        match op {
            Op::AddElement { element, .. } | Op::ReplaceElement { element, .. } => {
                canonicalize_tree(element, labels, registry);
            }
            Op::AddSlide { slide, .. } => canonicalize_slots(&mut slide.params, labels, registry),
            _ => {}
        }
    }
}

/// Canonicalise tous les graphiques d'un deck (à l'ouverture).
pub(crate) fn canonicalize_deck(deck: &mut Deck, labels: &ChartLabels, registry: &WidgetRegistry) {
    for slide in &mut deck.slides {
        canonicalize_slots(&mut slide.params, labels, registry);
    }
}

/// Le premier paramètre `host(chart)` du widget `kind`, s'il en a un.
pub(crate) fn chart_param<'r>(registry: &'r WidgetRegistry, kind: &str) -> Option<&'r str> {
    chart_params(registry.get(kind)?).next()
}

fn chart_params(schema: &WidgetSchema) -> impl Iterator<Item = &str> {
    schema.params.iter().filter(|p| p.ty.host() == Some(HostType::Chart)).map(|p| p.name.as_str())
}

fn canonicalize_slots(params: &mut Params, labels: &ChartLabels, registry: &WidgetRegistry) {
    for (_, children) in params.slots_mut() {
        for child in children {
            canonicalize_tree(child, labels, registry);
        }
    }
}

fn canonicalize_tree(e: &mut Element, labels: &ChartLabels, registry: &WidgetRegistry) {
    if let Some(schema) = registry.get(&e.kind) {
        for name in chart_params(schema) {
            let Some(ParamValue::Value(v)) = e.params.get(name) else { continue };
            let canonical = serde_json::from_value::<ChartOption>(v.clone())
                .map_err(|e| e.to_string())
                .and_then(|option| canonicalize(&option, labels).map_err(|e| e.to_string()))
                .and_then(|option| serde_json::to_value(option).map_err(|e| e.to_string()));
            match canonical {
                Ok(v) => {
                    e.params.set(name, ParamValue::Value(v));
                }
                // L'op refusera ce graphique avec la raison ; rien à ranger ici.
                Err(error) => tracing::debug!(element = %e.id, %error, "graphique non canonicalisé"),
            }
        }
    }
    canonicalize_slots(&mut e.params, labels, registry);
}

impl Session {
    /// Redessine le graphique `element` en `kind` : ses données sont
    /// converties (en-têtes dans la langue de l'utilisateur), l'élément
    /// remplacé en une transaction.
    pub fn set_chart_kind(
        &mut self,
        label: &str,
        slide: String,
        element: &str,
        kind: oku_slides_chart::SeriesType,
    ) -> Result<()> {
        let deck = self.doc.sources.deck();
        let located = deck.slide(&slide).context(NoSlideSnafu { id: &slide })?.locate(element);
        let mut chart = located.map(|l| l.element.clone()).context(NoChartSnafu { id: element })?;
        // L'option est le paramètre `host(chart)` du widget.
        let param = chart_param(&self.registry, &chart.kind).context(NoChartSnafu { id: element })?;
        let Some(ParamValue::Value(value)) = chart.params.get(param) else {
            return Err(NoChartSnafu { id: element }.build().into());
        };
        let option: oku_slides_chart::ChartOption =
            serde_json::from_value(value.clone()).ok().context(NoChartSnafu { id: element })?;
        let option = oku_slides_chart::convert(&option, kind, &self.chart_labels).context(ChartSnafu)?;
        let value = serde_json::to_value(option).context(ResponseSnafu)?;
        chart.params.set(param.to_owned(), ParamValue::Value(value));
        self.apply(label, vec![Op::ReplaceElement { slide, element: chart }], None)
    }
}

#[cfg(test)]
#[path = "charts_tests.rs"]
mod charts_tests;
