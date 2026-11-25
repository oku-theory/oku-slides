//! Chart ↔ table: the `show_as` request, in one transaction.

use oku_slides_chart::{ChartOption, MAX_TABLE_CELL_CHARS, TableOption};
use oku_slides_core::{HostType, Op, ParamValue, Params, kinds};
use snafu::{OptionExt, ResultExt};

use crate::Session;
use crate::error::{ChartSnafu, NoChartSnafu, NoSlideSnafu, NoTableSnafu, ResponseSnafu, Result};
use crate::request::View;

impl Session {
    /// Shows element `element` as `view`, on the same data; a single undo step.
    ///
    /// The element's other parameters are dropped: only the data ref, its projection and the
    /// options the conversion keeps carry over.
    ///
    /// # Errors
    ///
    /// Fails if the slide (`NoSlide`) or the element (`NoChart`, `NoTable`) cannot be found, if
    /// its data is missing, or if the transaction is rejected.
    pub fn show_as(&mut self, label: &str, slide: String, element: &str, view: View) -> Result<()> {
        let not_found = || match view {
            View::Table => NoChartSnafu { id: element }.build(),
            View::Chart => NoTableSnafu { id: element }.build(),
        };
        let deck = self.doc.sources.deck();
        let located = deck.slide(&slide).context(NoSlideSnafu { id: &slide })?.locate(element);
        let mut next = located.map(|l| l.element.clone()).ok_or_else(not_found)?;
        let (from, to, target_kind) = match view {
            // The base widget showing the data.
            View::Table => (HostType::Chart, HostType::Table, kinds::base::TABLE),
            View::Chart => (HostType::Table, HostType::Chart, kinds::base::CHART),
        };
        let target = self.data_target(&slide, element).filter(|t| t.host == from).ok_or_else(not_found)?;
        // The conversion reads the data the view shows; the result keeps the ref, not the cells.
        let mut dataset = self.view_dataset(&target, MAX_TABLE_CELL_CHARS)?;
        dataset.ids.clear();
        let mut value = target.value.clone();
        value["dataset"] = serde_json::to_value(&dataset).context(ResponseSnafu)?;
        let mut converted = match view {
            View::Table => {
                let chart: ChartOption = serde_json::from_value(value).ok().context(NoChartSnafu { id: element })?;
                serde_json::to_value(oku_slides_chart::chart_to_table(&chart).context(ChartSnafu)?)
            }
            View::Chart => {
                let table: TableOption = serde_json::from_value(value).ok().context(NoTableSnafu { id: element })?;
                serde_json::to_value(oku_slides_chart::table_to_chart(&table, &self.chart_labels).context(ChartSnafu)?)
            }
        }
        .context(ResponseSnafu)?;
        if let Some(o) = converted.as_object_mut() {
            o.remove("dataset");
        }
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
}
