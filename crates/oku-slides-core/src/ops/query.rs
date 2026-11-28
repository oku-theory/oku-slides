//! The query ops: set a data's query, and the reads every query op checks.
//!
//! A query must parse, read only data of the deck and never read its own data back. The checks
//! need the deck as it would be, so the op writes, checks, and puts the previous entries back
//! when it refuses: the deck is left intact.

use std::sync::Arc;

use oku_slides_data::sql::{Query, parse_query};
use oku_slides_data::{Data, DataId, Source};

use crate::error::{DataFault, OpError};
use crate::model::Deck;
use crate::query_order::Reads;

/// `set_query`: a new query for an existing query data.
pub(super) fn set_query(deck: &mut Deck, data: &DataId, sql: String) -> Result<(), OpError> {
    let Some(current) = deck.data.get(data) else {
        return Err(DataFault::Missing { data: data.as_str().to_owned() }.into());
    };
    if current.sql().is_none() {
        return Err(DataFault::NotQuery { data: data.as_str().to_owned() }.into());
    }
    let query = parse_query(&sql)?;
    let next = Data { source: Source::Query { sql }, ..(**current).clone() };
    put_query(deck, data, Arc::new(next), &query)
}

/// Puts the query data `entry` at `id`, refusing it when `query` reads a name no data has or
/// reads `id` back; a refusal puts the previous entry back.
pub(super) fn put_query(deck: &mut Deck, id: &DataId, entry: Arc<Data>, query: &Query) -> Result<(), OpError> {
    let name = entry.name.clone();
    let previous = deck.data.insert(id.clone(), entry);
    let checked = check_reads(deck, id, &name, query);
    if checked.is_err() {
        restore(deck, id, previous);
    }
    checked
}

/// Puts `previous` back at `id`, or removes the entry when there was none.
pub(super) fn restore(deck: &mut Deck, id: &DataId, previous: Option<Arc<Data>>) {
    match previous {
        Some(previous) => {
            deck.data.insert(id.clone(), previous);
        }
        None => {
            deck.data.remove(id);
        }
    }
}

/// Refuses `query`, the query of data `id` named `name`, when it reads a name no data has or
/// reads `id` back.
fn check_reads(deck: &Deck, id: &DataId, name: &str, query: &Query) -> Result<(), OpError> {
    let reads = Reads::new(deck);
    if let Some(name) = reads.unknown_input(query) {
        return Err(DataFault::UnknownInput { name }.into());
    }
    if reads.on_cycle(id) {
        return Err(DataFault::Cycle { data: name.to_owned() }.into());
    }
    Ok(())
}
