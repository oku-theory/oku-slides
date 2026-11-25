//! The reference to a data's base: an immutable Parquet file in the base store (written in 4f).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{DataError, IdInvalidSnafu};
use crate::{ColumnType, bounded, limits};

/// What a base id starts with.
const PREFIX: &str = "sha256:";
/// Lowercase hexadecimal characters of a SHA-256 hash.
const HASH_CHARS: usize = 64;

/// A base's content address: `sha256:` and 64 lowercase hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BaseId(String);

impl BaseId {
    /// The id of the file whose SHA-256 hash is `hash`.
    #[must_use]
    pub fn from_sha256(hash: &[u8; 32]) -> Self {
        BaseId(format!("{PREFIX}{}", hex::encode(hash)))
    }

    /// The 64 hexadecimal characters of the hash, the base's file name in the store.
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0[PREFIX.len()..]
    }

    /// The id as written (`sha256:…`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for BaseId {
    type Error = DataError;

    fn try_from(id: String) -> Result<Self, DataError> {
        let valid = id.strip_prefix(PREFIX).is_some_and(|hash| {
            hash.len() == HASH_CHARS && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
        if valid { Ok(BaseId(id)) } else { Err(IdInvalidSnafu { id }.build().into()) }
    }
}

/// JSON schema: a string holding `sha256:` and 64 lowercase hexadecimal characters.
impl JsonSchema for BaseId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "BaseId".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" })
    }
}

impl From<BaseId> for String {
    fn from(id: BaseId) -> String {
        id.0
    }
}

/// A data's base: its file and what it holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaseRef {
    /// Content address of the Parquet file.
    pub id: BaseId,
    /// Size of the file.
    pub bytes: u64,
    /// Rows in the file.
    pub rows: u32,
    /// Columns in the file, in file order.
    #[serde(deserialize_with = "bounded::vec::<_, _, { limits::MAX_COLUMNS }>")]
    pub columns: Vec<BaseColumn>,
}

/// One column of a base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaseColumn {
    /// Id of the column in the table.
    pub id: String,
    /// Type of the column in the file.
    #[serde(rename = "type")]
    pub ty: ColumnType,
    /// UTF-8 bytes of its text cells, to budget a read before it is made.
    pub text_bytes: u64,
}

#[cfg(test)]
#[path = "base_ref_tests.rs"]
mod base_ref_tests;
