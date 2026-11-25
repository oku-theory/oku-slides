//! Charts and tables of an opened deck: inline data lifted into deck data by `prepare_ops` in core.
//!
//! A chart is a `host(chart)` param of a registry widget.
//! `Session::set_chart_kind` redraws a chart as another type.

use oku_slides_chart::{ChartLabels, ChartOption};
use oku_slides_core::{
    Deck, Element, HostType, Issue, Op, OpError, Origin, ParamValue, Params, SourceBundle, Target, WidgetRegistry,
    WidgetSchema, apply_all, brings_inline, strip_inline,
};
use oku_slides_data::DataId;
use oku_slides_data::limits::MAX_TEXT_BYTES;
use oku_slides_i18n::Message;
use snafu::{OptionExt, ResultExt, ensure};

use crate::data_ops::prepare;
use crate::error::{
    ChartKindSharedSnafu, ChartSnafu, NoChartSnafu, NoSlideSnafu, PrepareSnafu, ResponseSnafu, SourcesSnafu,
};
use crate::files::{IssuesAt, log_issues};
use crate::{Result, Session};

impl Session {
    /// Lifts the inline data of the views of `sources` with its own registry, the one its
    /// replica reads them with; returns what could not be lifted and was stripped.
    ///
    /// # Errors
    ///
    /// `Sources` when `apply_silent` refuses.
    pub(crate) fn lift_inline_sources(&self, sources: &mut SourceBundle) -> Result<Vec<Issue>> {
        let mut deck = sources.deck().clone();
        let dropped = lift_inline_data(&mut deck, &self.chart_labels, sources.registry());
        // Rewritten options and new data take the canonical form the replica reads back.
        oku_slides_core::canonicalize(&mut deck);
        if deck != *sources.deck() {
            // Lifted or stripped inline data: not a user action, so no undo step.
            sources.apply_silent(deck).context(SourcesSnafu)?;
        }
        Ok(dropped)
    }

    /// The notice telling the user that `dropped` inline data were removed, if any.
    pub(crate) fn dropped_notice(&self, dropped: &[Issue]) -> Option<String> {
        let count = log_issues(dropped, &self.localizer, IssuesAt::Open);
        (count > 0).then(|| self.localizer.tr_with("notice-unreadable-parts", &[("count", count.into())]))
    }
}

/// Lifts the inline data of every chart and table of an opened deck into deck data, one view
/// at a time; returns what could not be lifted.
///
/// A deck stores no inline data (ADR-0015), and the replica drops them: a view whose data do
/// not lift (a ref next to them, inconsistent options) is stripped of them, reported as
/// `issue-data-inline-dropped`, so the deck stays equal to its replica.
fn lift_inline_data(deck: &mut Deck, labels: &ChartLabels, registry: &WidgetRegistry) -> Vec<Issue> {
    for op in inline_view_ops(deck, registry) {
        // What does not lift keeps its inline data, stripped and reported below.
        let Ok(prepared) = prepare(deck, registry, labels, vec![op], Origin::Host) else { continue };
        let mut next = deck.clone();
        if apply_all(&mut next, prepared.ops, registry).is_ok() {
            *deck = next;
        }
    }
    let mut dropped = Vec::new();
    for slide in &mut deck.slides {
        strip_params(registry, &slide.kind, &mut slide.params, &slide.id, None, &mut dropped);
    }
    dropped
}

/// A `set_param` per chart or table param of `deck` with inline data, re-setting it for `prepare_ops`.
fn inline_view_ops(deck: &Deck, registry: &WidgetRegistry) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut push = |target: Target, kind: &str, params: &Params| {
        for name in view_params(registry, kind) {
            let Some(ParamValue::Value(v)) = params.get(name) else { continue };
            if brings_inline(v) {
                ops.push(Op::SetParam { target: target.clone(), param: name.to_owned(), value: v.clone() });
            }
        }
    };
    for slide in &deck.slides {
        push(Target { slide: slide.id.clone(), element: None }, &slide.kind, &slide.params);
        for located in slide.walk() {
            let e = located.element;
            push(Target { slide: slide.id.clone(), element: Some(e.id.clone()) }, &e.kind, &e.params);
        }
    }
    ops
}

/// Strips the inline data left in the view params of `params` and of their slot children.
fn strip_params(
    registry: &WidgetRegistry,
    kind: &str,
    params: &mut Params,
    slide: &str,
    element: Option<&str>,
    dropped: &mut Vec<Issue>,
) {
    let names: Vec<String> = view_params(registry, kind).map(str::to_owned).collect();
    let mut stripped = false;
    for (name, value) in params.iter_mut() {
        if let ParamValue::Value(v) = value
            && names.iter().any(|n| n == name)
            && brings_inline(v)
        {
            strip_inline(v);
            stripped = true;
        }
    }
    if stripped {
        let owner = element.unwrap_or(slide).to_owned();
        dropped.push(Issue {
            slide: Some(slide.to_owned()),
            element: element.map(str::to_owned),
            message: Message::new("issue-data-inline-dropped").arg("element", owner),
        });
    }
    for (_, children) in params.slots_mut() {
        for Element { id, kind, params, .. } in children.iter_mut() {
            strip_params(registry, kind, params, slide, Some(id), dropped);
        }
    }
}

/// The chart and table params of widget `kind`.
fn view_params<'r>(registry: &'r WidgetRegistry, kind: &str) -> impl Iterator<Item = &'r str> {
    registry.get(kind).into_iter().flat_map(|schema: &WidgetSchema| {
        schema
            .params
            .iter()
            .filter(|p| matches!(p.ty.host(), Some(HostType::Chart | HostType::Table)))
            .map(|p| p.name.as_str())
    })
}

impl Session {
    /// Redraws chart `element` as `kind`.
    ///
    /// The data it reads is converted (headers in the user's language) and the element replaced
    /// in one transaction. A conversion reading only columns the data holds keeps the ref,
    /// projected on them; one deriving new columns (a waterfall's Total, a scatter's x) writes
    /// the converted data inline, which `prepare_ops` stores in place of the view's own data.
    ///
    /// # Errors
    ///
    /// Fails if the slide (`NoSlide`) or the chart (`NoChart`) cannot be found, if its data is
    /// missing or does not convert, if the kind derives columns of a data other views show
    /// (`ChartKindShared`), or if the transaction is rejected.
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
        let target = self.data_target(&slide, element).filter(|t| t.host == HostType::Chart);
        let target = target.context(NoChartSnafu { id: element })?;
        // Text is never cut: a converted dataset may be written back.
        let mut read = self.view_dataset(&target, MAX_TEXT_BYTES)?;
        read.ids.clear();
        let mut option: ChartOption =
            serde_json::from_value(target.value.clone()).ok().context(NoChartSnafu { id: element })?;
        option.dataset = Some(read.clone());
        let mut converted = oku_slides_chart::convert(&option, kind, &self.chart_labels).context(ChartSnafu)?;
        let dataset = converted.dataset.take().unwrap_or_default();
        if let Some(columns) = mapped_columns(&read, &dataset) {
            // Every converted column is one the data holds: the ref stays, on those columns.
            if columns != read.columns {
                converted.columns = Some(columns);
            }
        } else {
            // The kind derives columns the data lacks: written inline, so `prepare_ops` puts
            // them in place of the view's own data. A data other views show is never forked.
            let data = DataId::try_from(target.data()?.to_owned()).map_err(OpError::from).context(PrepareSnafu)?;
            ensure!(self.doc.sources.deck().data_users(&data).len() <= 1, ChartKindSharedSnafu { id: element });
            converted.data = None;
            converted.columns = None;
            converted.dataset = Some(oku_slides_data::Dataset { source: dataset.source, ..Default::default() });
        }
        let value = serde_json::to_value(converted).context(ResponseSnafu)?;
        chart.params.set(target.param, ParamValue::Value(value));
        self.apply(label, vec![Op::ReplaceElement { slide, element: chart }], None)
    }
}

/// The column of `read` each column of `converted` is, matched by header and cells in any
/// order, or `None` when a converted column is none of them.
fn mapped_columns(read: &oku_slides_data::Dataset, converted: &oku_slides_data::Dataset) -> Option<Vec<String>> {
    let width = converted.source.first().map_or(0, Vec::len);
    if width == 0 || read.source.len() != converted.source.len() {
        return None;
    }
    let column = |source: &[Vec<oku_slides_data::DataCell>], j: usize| -> Option<Vec<oku_slides_data::DataCell>> {
        source.iter().map(|row| row.get(j).cloned()).collect()
    };
    let mut used = vec![false; read.columns.len()];
    let mut out = Vec::with_capacity(width);
    for j in 0..width {
        let wanted = column(&converted.source, j)?;
        let i = (0..read.columns.len()).find(|&i| !used[i] && column(&read.source, i).as_ref() == Some(&wanted))?;
        used[i] = true;
        out.push(read.columns[i].clone());
    }
    Some(out)
}
