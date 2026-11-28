//! A data: a named table of a deck, shared by the views bound to it.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{DataError, IdInvalidSnafu, InlineQuerySnafu, SourceMismatchSnafu};
use crate::limits::MAX_SQL_BYTES;
use crate::{Table, bounded, valid_id};

/// Id of a data in its deck.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DataId(String);

impl DataId {
    /// The id as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Lets a map keyed by `DataId` be read with a `&str`; consistent with the derived order.
impl std::borrow::Borrow<str> for DataId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DataId {
    type Error = DataError;

    fn try_from(id: String) -> Result<Self, DataError> {
        if valid_id(&id) { Ok(DataId(id)) } else { Err(IdInvalidSnafu { id }.build().into()) }
    }
}

/// JSON schema: a string holding a valid id.
impl JsonSchema for DataId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DataId".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "pattern": "^[A-Za-z0-9_-]{1,64}$" })
    }
}

impl From<DataId> for String {
    fn from(id: DataId) -> String {
        id.0
    }
}

/// A data of a deck.
///
/// Its JSON is flat: `{"name", "label"?, "inline"?, "kind": "manual", "table": {…}}` or
/// `{"name", "label"?, "inline"?, "kind": "query", "sql": "…"}`; a `table` without
/// `kind: manual`, or `sql` without `kind: query`, is refused (`data-source-mismatch`).
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(try_from = "DataRepr")]
#[schemars(with = "DataRepr")]
pub struct Data {
    /// Name used by queries, unique in the deck (see [`crate::valid_name`]).
    pub name: String,
    /// Displayed label.
    pub label: Option<String>,
    /// Private to one element, hidden from the data list.
    pub inline: bool,
    /// Where its rows come from.
    pub source: Source,
}

/// Where a data's rows come from.
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "most data is manual and held in an Arc; boxing the table would only add an indirection"
)]
pub enum Source {
    /// Typed or pasted by hand: its table.
    Manual(Table),
    /// Derived from other data by a query: read-only, its rows are the query's result.
    Query {
        /// The query, at most [`MAX_SQL_BYTES`] bytes.
        sql: String,
    },
}

/// The kind of a data's source, as its JSON and the replica name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DataKind {
    /// Typed or pasted by hand.
    Manual,
    /// Derived by a query.
    Query,
}

impl DataKind {
    /// The kind as written in JSON and in the replica.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DataKind::Manual => "manual",
            DataKind::Query => "query",
        }
    }
}

impl Data {
    /// The kind of its source.
    #[must_use]
    pub fn kind(&self) -> DataKind {
        match self.source {
            Source::Manual(_) => DataKind::Manual,
            Source::Query { .. } => DataKind::Query,
        }
    }

    /// Its table, for a manual data; `None` for a query.
    #[must_use]
    pub fn table(&self) -> Option<&Table> {
        match &self.source {
            Source::Manual(table) => Some(table),
            Source::Query { .. } => None,
        }
    }

    /// Its table to edit, for a manual data; `None` for a query.
    #[must_use]
    pub fn table_mut(&mut self) -> Option<&mut Table> {
        match &mut self.source {
            Source::Manual(table) => Some(table),
            Source::Query { .. } => None,
        }
    }

    /// Its query, for a query data; `None` for a manual one.
    #[must_use]
    pub fn sql(&self) -> Option<&str> {
        match &self.source {
            Source::Manual(_) => None,
            Source::Query { sql } => Some(sql),
        }
    }
}

/// The JSON form of a [`Data`], read before its kind and source are checked together.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "Data")]
struct DataRepr {
    /// Name used by queries, unique in the deck (see [`crate::valid_name`]).
    name: String,
    /// Displayed label.
    #[serde(default)]
    label: Option<String>,
    /// Private to one element, hidden from the data list.
    #[serde(default)]
    inline: bool,
    /// Where its rows come from: `manual` with a `table`, `query` with its `sql`.
    kind: DataKind,
    /// The table of a manual data.
    #[serde(default)]
    table: Option<Table>,
    /// The query of a derived data, at most [`MAX_SQL_BYTES`] UTF-8 bytes.
    ///
    /// The schema's `maxLength` counts characters, so it only bounds it loosely: the bytes
    /// are checked on read.
    #[serde(default, deserialize_with = "sql")]
    #[schemars(length(max = MAX_SQL_BYTES))]
    sql: Option<String>,
}

/// A present `sql`, refused past [`MAX_SQL_BYTES`] before it is copied.
fn sql<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    bounded::string::<_, MAX_SQL_BYTES>(deserializer).map(Some)
}

impl TryFrom<DataRepr> for Data {
    type Error = DataError;

    fn try_from(repr: DataRepr) -> Result<Self, DataError> {
        let source = match (repr.kind, repr.table, repr.sql) {
            (DataKind::Manual, Some(table), None) => Source::Manual(table),
            (DataKind::Query, None, Some(sql)) => Source::Query { sql },
            (kind, _, _) => return Err(SourceMismatchSnafu { kind }.build().into()),
        };
        // Inline data is made only from a view's inline table (spec: inline data as creation input).
        if repr.inline && matches!(source, Source::Query { .. }) {
            return Err(InlineQuerySnafu.build().into());
        }
        Ok(Data { name: repr.name, label: repr.label, inline: repr.inline, source })
    }
}

/// The JSON form of a [`Data`] as written, borrowing its table or query.
#[derive(Serialize)]
struct DataOut<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<&'a str>,
    #[serde(skip_serializing_if = "is_false")]
    inline: bool,
    kind: DataKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    table: Option<&'a Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sql: Option<&'a str>,
}

impl Serialize for Data {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        DataOut {
            name: &self.name,
            label: self.label.as_deref(),
            inline: self.inline,
            kind: self.kind(),
            table: self.table(),
            sql: self.sql(),
        }
        .serialize(serializer)
    }
}

/// Skips `inline: false` when serializing.
#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
