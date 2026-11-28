//! Validation of a deck's data: names, cell types, bounds, bases, truncations and queries.

use std::collections::{BTreeSet, HashMap, HashSet};

use oku_slides_data::DataId;
use oku_slides_data::limits::MAX_DATA;
use oku_slides_i18n::Message;

use crate::model::Deck;
use crate::query_order::QueryFault;
use crate::validate::Issue;

/// Data issues reported by one validation; past it, one `issue-more` counts the rest.
pub(crate) const MAX_DATA_ISSUES: usize = 200;

/// Dropped data ids named in `issue-data-too-many`; the others are counted only.
const NAMED_DROPPED: usize = 20;

/// An issue on the deck's data.
pub(crate) fn data_issue(message: Message) -> Issue {
    Issue { slide: None, element: None, message }
}

/// `issue-data-too-many`: `count` data found, the `dropped` ones not kept.
pub(crate) fn too_many(count: usize, dropped: &[DataId]) -> Message {
    let mut names: Vec<&str> = dropped.iter().take(NAMED_DROPPED).map(DataId::as_str).collect();
    if dropped.len() > NAMED_DROPPED {
        names.push("…");
    }
    Message::new("issue-data-too-many").arg("count", count).arg("dropped", names.join(", "))
}

/// `issue-data-truncated`: the data `name` was cut on reading and is kept read-only.
pub(crate) fn truncated(name: &str) -> Issue {
    data_issue(Message::new("issue-data-truncated").arg("data", name.to_owned()))
}

/// `issue-data-kind-unknown`: the data `name` has a `kind` this version does not know, and is
/// not read.
pub(crate) fn kind_unknown(name: &str, kind: &str) -> Issue {
    data_issue(Message::new("issue-data-kind-unknown").arg("data", name.to_owned()).arg("kind", kind.to_owned()))
}

/// `issue-data-base-missing`: the base of the data `name` is not available.
pub(crate) fn base_missing(name: &str) -> Issue {
    data_issue(Message::new("issue-data-base-missing").arg("data", name.to_owned()))
}

/// `issue-data-edits-lost`: a merge lost edits made to the data `name`.
pub(crate) fn edits_lost(name: &str) -> Issue {
    data_issue(Message::new("issue-data-edits-lost").arg("data", name.to_owned()))
}

/// `issue-data-cell-type`: a column of the data `data` holds cells that do not fit `ty`.
pub(crate) fn cell_type(data: &str, column: &str, ty: &str) -> Issue {
    let message = Message::new("issue-data-cell-type").arg("data", data.to_owned()).arg("column", column.to_owned());
    data_issue(message.arg("type", ty.to_owned()))
}

/// `issue-data-query-*`: the query of the data `name` cannot compute, as `fault` says.
pub(crate) fn query_fault(name: &str, fault: &QueryFault) -> Issue {
    data_issue(fault.message(name))
}

/// Issues kept up to [`MAX_DATA_ISSUES`]; past it, only counted.
#[derive(Default)]
struct Capped {
    kept: Vec<Issue>,
    more: usize,
}

impl Capped {
    /// Keeps the issue `make` builds, or only counts it past the cap.
    fn push(&mut self, make: impl FnOnce() -> Issue) {
        if self.kept.len() < MAX_DATA_ISSUES {
            self.kept.push(make());
        } else {
            self.more += 1;
        }
    }

    /// The kept issues, then one `issue-more` counting the rest, if any.
    fn finish(mut self) -> Vec<Issue> {
        if self.more > 0 {
            self.kept.push(data_issue(Message::new("issue-more").arg("count", self.more)));
        }
        self.kept
    }
}

/// The issues of `deck`'s data, at most [`MAX_DATA_ISSUES`] then `issue-more`.
pub(crate) fn issues(deck: &Deck) -> Vec<Issue> {
    let mut out = Capped::default();
    if deck.data.len() > MAX_DATA {
        out.push(|| {
            let dropped: Vec<DataId> = deck.data.keys().skip(MAX_DATA).cloned().collect();
            data_issue(too_many(deck.data.len(), &dropped))
        });
    }
    let mut names = HashSet::new();
    for (id, data) in &deck.data {
        if deck.truncated.contains(id) {
            out.push(|| truncated(&data.name));
        }
        if !names.insert(data.name.to_lowercase()) {
            out.push(|| data_issue(Message::new("issue-data-name-taken").arg("data", data.name.clone())));
        }
        let Some(table) = data.table() else { continue };
        let index: HashMap<&str, usize> = table.columns.iter().enumerate().map(|(i, c)| (c.id.as_str(), i)).collect();
        let mut misfit = BTreeSet::new();
        for (key, cell) in &table.cells {
            if let Some(&column) = index.get(key.column.as_str())
                && !table.columns[column].ty.fits(cell)
            {
                misfit.insert(column);
            }
        }
        for column in misfit {
            let c = &table.columns[column];
            out.push(|| cell_type(&data.name, &c.name, c.ty.as_str()));
        }
    }
    for (id, fault) in &deck.query_order().broken {
        if let Some(data) = deck.data.get(id) {
            out.push(|| query_fault(&data.name, fault));
        }
    }
    out.finish()
}

#[cfg(test)]
#[path = "validate_data_tests.rs"]
mod validate_data_tests;
