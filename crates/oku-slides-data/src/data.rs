//! A data: a named table of a deck, shared by the views bound to it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{DataError, IdInvalidSnafu};
use crate::{Table, valid_id};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Data {
    /// Name used by formulas, unique in the deck (see [`crate::valid_name`]).
    pub name: String,
    /// Displayed label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Private to one element, hidden from the data list.
    #[serde(default, skip_serializing_if = "is_false")]
    pub inline: bool,
    /// Where its rows come from.
    pub kind: DataKind,
    /// Its table.
    pub table: Table,
}

/// Where a data's rows come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DataKind {
    /// Typed or pasted by hand.
    Manual,
}

/// Skips `inline: false` when serializing.
#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
