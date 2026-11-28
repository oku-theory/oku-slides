//! What the deck's queries read: their readers, the order they compute in, and those a merge
//! left broken.
//!
//! A query names its inputs by data name ([`Query::inputs`], lowercased); a name resolves to
//! every data that has it, case ignored, as a merge may leave two data with one name. Only
//! query data have inputs: data entered by hand are the leaves.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use oku_slides_data::DataId;
use oku_slides_data::sql::{Query, parse_query};
use oku_slides_i18n::Message;

use crate::model::Deck;

/// The order the deck's queries compute in, and those that cannot compute.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryOrder {
    /// The query data that are not broken, each after the queries it reads.
    pub order: Vec<DataId>,
    /// The query data that cannot compute, and why; a merge leaves them, the ops refuse them.
    pub broken: BTreeMap<DataId, QueryFault>,
}

/// Why a query data cannot compute.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum QueryFault {
    /// The data is on a cycle: its query reads it back, through itself or other queries.
    Cycle,
    /// The query reads this name (lowercased), which no data of the deck has.
    MissingInput(String),
    /// The query no longer parses, or the whitelist refuses it: the reason, as a message.
    Invalid(Message),
}

impl QueryFault {
    /// `issue-data-query-*`: the query of the data `name` cannot compute, as this fault says.
    #[must_use]
    pub fn message(&self, name: &str) -> Message {
        let message = match self {
            QueryFault::Cycle => Message::new("issue-data-query-cycle"),
            QueryFault::MissingInput(input) => {
                Message::new("issue-data-query-missing-input").arg("input", input.clone())
            }
            QueryFault::Invalid(reason) => Message::new("issue-data-query-invalid").arg("reason", reason.clone()),
        };
        message.arg("data", name.to_owned())
    }
}

impl Deck {
    /// The query data that read data `id` by its name, in id order; `id` itself if it reads
    /// its own name.
    ///
    /// A query that does not parse (a merge may leave one) reads nothing here: a rename does
    /// not rewrite it, and it keeps naming the old name, which `query_order` then reports.
    #[must_use]
    pub fn data_readers(&self, id: &DataId) -> Vec<DataId> {
        let Some(name) = self.data.get(id).map(|d| d.name.to_ascii_lowercase()) else { return Vec::new() };
        self.data
            .iter()
            .filter(|(_, data)| {
                data.sql().and_then(|sql| parse_query(sql).ok()).is_some_and(|q| q.inputs().contains(&name))
            })
            .map(|(reader, _)| reader.clone())
            .collect()
    }

    /// The query data in compute order, and the broken ones.
    ///
    /// A query that does not parse is `Invalid`, one naming no data `MissingInput` (its first
    /// such name), and every data on a cycle `Cycle`, whatever else it misses. The others are
    /// ordered inputs first, ties by id; a query reading a broken one stays in the order, and
    /// finds that input in error when it computes.
    #[must_use]
    pub fn query_order(&self) -> QueryOrder {
        let reads = Reads::new(self);
        let mut broken = BTreeMap::new();
        let mut edges: BTreeMap<DataId, BTreeSet<DataId>> = BTreeMap::new();
        for (id, data) in &self.data {
            let Some(sql) = data.sql() else { continue };
            match parse_query(sql) {
                Err(error) => {
                    broken.insert(id.clone(), QueryFault::Invalid(error.message()));
                }
                Ok(query) => {
                    let mut inputs = BTreeSet::new();
                    for input in query.inputs() {
                        let ids = reads.resolve(&input);
                        if ids.is_empty() {
                            broken.entry(id.clone()).or_insert(QueryFault::MissingInput(input));
                        }
                        inputs.extend(ids.iter().filter(|i| reads.is_query(i)).cloned());
                    }
                    edges.insert(id.clone(), inputs);
                }
            }
        }
        for id in edges.keys().filter(|id| reaches(&edges, id)) {
            broken.insert(id.clone(), QueryFault::Cycle);
        }
        QueryOrder { order: topological(&edges, &broken), broken }
    }
}

/// Whether `id` reaches itself along `edges`; iterative, each node visited once.
fn reaches(edges: &BTreeMap<DataId, BTreeSet<DataId>>, id: &DataId) -> bool {
    let mut stack: Vec<&DataId> = edges.get(id).into_iter().flatten().collect();
    let mut seen = BTreeSet::new();
    while let Some(next) = stack.pop() {
        if next == id {
            return true;
        }
        if seen.insert(next) {
            stack.extend(edges.get(next).into_iter().flatten());
        }
    }
    false
}

/// The queries of `edges` not in `broken`, inputs first, ties by id (Kahn); an edge to a
/// broken query is ignored, so every cycle being broken, each query left is placed.
fn topological(edges: &BTreeMap<DataId, BTreeSet<DataId>>, broken: &BTreeMap<DataId, QueryFault>) -> Vec<DataId> {
    let live = |id: &DataId| edges.contains_key(id) && !broken.contains_key(id);
    let mut waiting: BTreeMap<&DataId, usize> = BTreeMap::new();
    let mut readers: BTreeMap<&DataId, Vec<&DataId>> = BTreeMap::new();
    for (id, inputs) in edges.iter().filter(|(id, _)| live(id)) {
        let inputs: Vec<&DataId> = inputs.iter().filter(|i| live(i)).collect();
        waiting.insert(id, inputs.len());
        for input in inputs {
            readers.entry(input).or_default().push(id);
        }
    }
    let mut ready: BTreeSet<&DataId> = waiting.iter().filter(|(_, n)| **n == 0).map(|(id, _)| *id).collect();
    let mut order = Vec::with_capacity(waiting.len());
    while let Some(id) = ready.pop_first() {
        order.push(id.clone());
        for reader in readers.get(id).into_iter().flatten() {
            if let Some(n) = waiting.get_mut(reader) {
                *n -= 1;
                if *n == 0 {
                    ready.insert(reader);
                }
            }
        }
    }
    order
}

/// The data of a deck by name, to follow what its queries read.
pub(crate) struct Reads<'a> {
    deck: &'a Deck,
    names: HashMap<String, Vec<DataId>>,
}

impl<'a> Reads<'a> {
    pub(crate) fn new(deck: &'a Deck) -> Self {
        let mut names: HashMap<String, Vec<DataId>> = HashMap::new();
        for (id, data) in &deck.data {
            names.entry(data.name.to_ascii_lowercase()).or_default().push(id.clone());
        }
        Reads { deck, names }
    }

    /// The data named `input` (lowercased), case ignored.
    pub(crate) fn resolve(&self, input: &str) -> &[DataId] {
        self.names.get(input).map_or(&[], Vec::as_slice)
    }

    /// The first input of `query` that no data of the deck has.
    pub(crate) fn unknown_input(&self, query: &Query) -> Option<String> {
        query.inputs().into_iter().find(|input| self.resolve(input).is_empty())
    }

    /// Whether data `id` reads itself back through the queries, its own included.
    pub(crate) fn on_cycle(&self, id: &DataId) -> bool {
        let mut stack = self.query_inputs(id);
        let mut seen = BTreeSet::new();
        while let Some(next) = stack.pop() {
            if next == *id {
                return true;
            }
            if !seen.contains(&next) {
                stack.extend(self.query_inputs(&next));
                seen.insert(next);
            }
        }
        false
    }

    fn is_query(&self, id: &DataId) -> bool {
        self.deck.data.get(id).is_some_and(|d| d.sql().is_some())
    }

    /// The data the query of `id` reads; none for a data entered by hand or a query that does not parse.
    fn query_inputs(&self, id: &DataId) -> Vec<DataId> {
        let Some(query) = self.deck.data.get(id).and_then(|d| d.sql()).and_then(|sql| parse_query(sql).ok()) else {
            return Vec::new();
        };
        query.inputs().iter().flat_map(|input| self.resolve(input)).cloned().collect()
    }
}

#[cfg(test)]
#[path = "query_order_tests.rs"]
mod query_order_tests;
