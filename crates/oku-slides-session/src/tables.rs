//! Graphique ↔ tableau : la requête `show_as`, en une transaction.

use oku_slides_core::{Element, HostType, Op, ParamValue, Params, kinds};
use snafu::{OptionExt, ResultExt};

use crate::Session;
use crate::error::{ChartSnafu, NoChartSnafu, NoSlideSnafu, NoTableSnafu, ResponseSnafu, Result};
use crate::request::View;

impl Session {
    /// Shows element `element` as `view`, with the same data; a single undo step.
    ///
    /// The element's other parameters are dropped: only the dataset carries over.
    ///
    /// # Errors
    ///
    /// Fails if the slide (`NoSlide`) or the element (`NoChart`, `NoTable`) cannot be found, or if
    /// the transaction is rejected.
    pub fn show_as(&mut self, label: &str, slide: String, element: &str, view: View) -> Result<()> {
        let not_found = || match view {
            View::Table => NoChartSnafu { id: element }.build(),
            View::Chart => NoTableSnafu { id: element }.build(),
        };
        let deck = self.doc.sources.deck();
        let located = deck.slide(&slide).context(NoSlideSnafu { id: &slide })?.locate(element);
        let mut next = located.map(|l| l.element.clone()).ok_or_else(not_found)?;
        let (from, to, target_kind) = match view {
            // Le widget de base sous lequel on montre des données.
            View::Table => (HostType::Chart, HostType::Table, kinds::base::TABLE),
            View::Chart => (HostType::Table, HostType::Chart, kinds::base::CHART),
        };
        let value = self.host_value(&next, from).ok_or_else(not_found)?;
        let converted = match view {
            View::Table => {
                let chart = serde_json::from_value(value).ok().context(NoChartSnafu { id: element })?;
                serde_json::to_value(oku_slides_chart::chart_to_table(&chart).context(ChartSnafu)?)
            }
            View::Chart => {
                let table = serde_json::from_value(value).ok().context(NoTableSnafu { id: element })?;
                serde_json::to_value(oku_slides_chart::table_to_chart(&table, &self.chart_labels).context(ChartSnafu)?)
            }
        }
        .context(ResponseSnafu)?;
        let param = self
            .registry
            .get(target_kind)
            .and_then(|w| w.host_param(to))
            .map(|p| p.name.clone())
            .ok_or_else(not_found)?;
        target_kind.clone_into(&mut next.kind);
        next.params = Params::default();
        next.params.set(param, ParamValue::Value(converted));
        self.apply(label, vec![Op::ReplaceElement { slide, element: next }], None)
    }

    /// La valeur du paramètre `host(host)` de `e`, si son widget en a un.
    fn host_value(&self, e: &Element, host: HostType) -> Option<serde_json::Value> {
        let param = self.registry.get(&e.kind)?.host_param(host)?;
        match e.params.get(&param.name) {
            Some(ParamValue::Value(v)) => Some(v.clone()),
            _ => None,
        }
    }
}
