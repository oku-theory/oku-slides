//! What one deck's data hold, shared out over the charts and tables an import brings.
//!
//! The core refuses a batch past the deck bounds; import cuts instead. In deck order, each
//! inline `dataset` keeps the rows that still fit: whole, cut (its header and first rows), or,
//! once no row fits or past [`MAX_DATA`] data, none: the view param is removed (a chart needs
//! data) and the element comes in without data.

use oku_slides_core::{Deck, DeckLoad, Element, HostType, ParamValue, Params, Slide, WidgetRegistry};
use oku_slides_data::limits::{MAX_DATA, MAX_NAME_CHARS};
use oku_slides_data::{Dataset, NewTable};
use serde_json::{Map, Value};

use crate::report::{Loss, LossKind, SlideReport};

/// UTF-8 bytes a data name can take: its name is drawn at prepare time, so the longest.
const NAME_BYTES: usize = MAX_NAME_CHARS * 4;

/// What is left of one deck's data bounds as an import's slides come in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeckBudget {
    /// What the data already brought weigh.
    used: DeckLoad,
    /// How many data they are.
    data: usize,
}

impl DeckBudget {
    /// The budget left on `deck`.
    pub(crate) fn of(deck: &Deck) -> DeckBudget {
        DeckBudget { used: DeckLoad::of_deck(deck), data: deck.data.len() }
    }

    /// Cuts the inline data of `slide`'s charts and tables to what is left, in tree order;
    /// the elements cut, `None` for a view of the slide itself.
    pub(crate) fn fit(&mut self, registry: &WidgetRegistry, slide: &mut Slide) -> Vec<Option<String>> {
        let mut cut = Vec::new();
        self.tree(registry, None, &slide.kind, &mut slide.params, &mut cut);
        cut
    }

    /// The views of one element or slide, then of its slot children, as `prepare_ops` walks.
    fn tree(
        &mut self,
        registry: &WidgetRegistry,
        element: Option<&str>,
        kind: &str,
        params: &mut Params,
        cut: &mut Vec<Option<String>>,
    ) {
        let views: Vec<String> = registry.get(kind).map_or_else(Vec::new, |schema| {
            schema
                .params
                .iter()
                .filter(|p| matches!(p.ty.host(), Some(HostType::Chart | HostType::Table)))
                .map(|p| p.name.clone())
                .collect()
        });
        let mut dropped = Vec::new();
        for (name, value) in params.iter_mut() {
            if !views.iter().any(|v| v == name) {
                continue;
            }
            let ParamValue::Value(Value::Object(option)) = value else { continue };
            match self.fit_view(option) {
                Fit::Whole => continue,
                Fit::Cut => {}
                Fit::None => dropped.push(name.to_owned()),
            }
            cut.push(element.map(str::to_owned));
        }
        // A view without data: a chart needs one, so the element keeps none of its view.
        for name in dropped {
            params.remove(&name);
        }
        for (_, children) in params.slots_mut() {
            for Element { id, kind, params, .. } in children {
                self.tree(registry, Some(id), kind, params, cut);
            }
        }
    }

    /// Cuts the `dataset` of one view option to the rows left.
    fn fit_view(&mut self, option: &mut Map<String, Value>) -> Fit {
        // Not a dataset `prepare_ops` lifts: the apply reports it, with the reason.
        let Some(Ok(dataset)) = option.get("dataset").map(|d| serde_json::from_value::<Dataset>(d.clone())) else {
            return Fit::Whole;
        };
        let table = NewTable::from_dataset(&dataset).table;
        let label = option.get("title").and_then(|t| t.get("text")).and_then(Value::as_str).map_or(0, str::len);
        let names: usize = table.columns.iter().map(|c| c.name.len()).sum();
        let mut load = self.used.plus(DeckLoad { row_ids: 0, overlay: 0, text: NAME_BYTES + label + names });
        if self.data >= MAX_DATA || !load.fits() {
            return Fit::None;
        }
        let mut kept = 0;
        for row in table.row_loads() {
            let next = load.plus(DeckLoad { row_ids: 1, overlay: row.cells, text: row.text });
            if !next.fits() {
                break;
            }
            load = next;
            kept += 1;
        }
        if kept == table.rows.len() {
            self.take(load);
            return Fit::Whole;
        }
        if kept == 0 {
            return Fit::None;
        }
        self.take(load);
        if let Some(Value::Array(rows)) = option.get_mut("dataset").and_then(|d| d.get_mut("source")) {
            rows.truncate(kept + 1);
        }
        Fit::Cut
    }

    /// Counts one more data, the deck's data then weighing `load`.
    fn take(&mut self, load: DeckLoad) {
        self.used = load;
        self.data += 1;
    }
}

/// What a view's inline data keep.
enum Fit {
    /// Every row.
    Whole,
    /// The header and the first rows.
    Cut,
    /// Nothing: the view is removed.
    None,
}

/// Reports each of `cuts` (from [`DeckBudget::fit`]) on `report` as `ChartTruncated`, on the
/// source item `origins` gives for its element (item id, element id), or on the slide.
pub(crate) fn report_cuts(report: &mut SlideReport, cuts: &[Option<String>], origins: &[(String, String)]) {
    for element in cuts {
        let item = element.as_deref().and_then(|e| origins.iter().find(|(_, o)| o == e)).map(|(item, _)| item);
        report.losses.push(match item {
            Some(item) => Loss::on(item, LossKind::ChartTruncated),
            None => Loss::slide(LossKind::ChartTruncated),
        });
    }
}

#[cfg(test)]
#[path = "deck_budget_tests.rs"]
mod deck_budget_tests;
