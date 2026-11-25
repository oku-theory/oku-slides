//! The views of a deck data: the chart and table params that name it.

use oku_slides_data::DataId;
use serde_json::Value;

use crate::model::Deck;
use crate::params::{ParamValue, Params};

/// A view of a data: the param of a slide or element that reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRef {
    /// Slide that holds the view.
    pub slide: String,
    /// Element that holds the view; the slide id for a slide param.
    pub element: String,
    /// Chart or table param that names the data.
    pub param: String,
    /// The view's projection, if it reads only some columns.
    pub columns: Option<Vec<String>>,
}

impl Deck {
    /// The views of data `id`, slide by slide, nested elements in document order.
    ///
    /// A view is any param whose object value has `"data": id`; no widget schema is read.
    ///
    /// The rule is deliberately wider than the widgets' `host(chart)` and `host(table)` params:
    /// only those can hold a ref (the op checks refuse it elsewhere), and a param that matched
    /// by mistake would only make a data look shared, which keeps it (no in-place replace, no
    /// projected column removed). The render reads views through the registry instead
    /// ([`plan_views`](crate::plan_views)).
    #[must_use]
    pub fn data_users(&self, id: &DataId) -> Vec<ViewRef> {
        let mut out = Vec::new();
        for slide in &self.slides {
            push_views(&mut out, id, &slide.id, &slide.id, &slide.params);
            for located in slide.walk() {
                push_views(&mut out, id, &slide.id, &located.element.id, &located.element.params);
            }
        }
        out
    }
}

/// Appends to `out` the params of `params` that name `id`.
fn push_views(out: &mut Vec<ViewRef>, id: &DataId, slide: &str, element: &str, params: &Params) {
    for (param, value) in params.iter() {
        let ParamValue::Value(v) = value else { continue };
        if v.get("data").and_then(Value::as_str) != Some(id.as_str()) {
            continue;
        }
        let columns = v
            .get("columns")
            .and_then(Value::as_array)
            .map(|c| c.iter().filter_map(Value::as_str).map(str::to_owned).collect());
        out.push(ViewRef { slide: slide.to_owned(), element: element.to_owned(), param: param.to_owned(), columns });
    }
}
