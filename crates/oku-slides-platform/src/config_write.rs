//! Writes a key into `config.toml`, keeping the rest as is.
//!
//! Comments, order and other keys stay as they are (`toml_edit`). The change
//! takes effect on next launch.

use std::path::{Path, PathBuf};

use serde_json::Value;
use snafu::{Backtrace, IntoError, OptionExt, ResultExt, Snafu};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::storage::{Storage, StorageError, StorageErrorKind};

/// `config.toml` n'a pas pu être modifié ; le fichier est intact.
#[derive(Debug, Snafu)]
pub struct ConfigWriteError(ConfigWriteInner);

#[derive(Debug, Snafu)]
enum ConfigWriteInner {
    #[snafu(display("cannot read {}", path.display()))]
    Read { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("{} is not valid TOML", path.display()))]
    Parse { path: PathBuf, source: toml_edit::TomlError, backtrace: Backtrace },
    #[snafu(display("{key} is not a settings key"))]
    Key { key: String, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Write { path: PathBuf, source: StorageError, backtrace: Backtrace },
}

/// Writes `json` at the dotted key `key` (`history.limit`, `stt.vad.pause`);
/// `null` removes the key.
///
/// # Errors
///
/// If the file is unreadable or not TOML, if `key` ends with a dot or goes through a value
/// that is not a table, or if the write fails.
#[expect(
    clippy::result_large_err,
    reason = "the error carries the `TomlError` inline; a settings write is rare, off the hot path"
)]
pub fn write_value(storage: &Storage, path: &Path, key: &str, json: &Value) -> Result<(), ConfigWriteError> {
    let text = match storage.read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(e) if e.kind() == StorageErrorKind::NotFound => String::new(),
        Err(source) => return Err(ReadSnafu { path }.into_error(source).into()),
    };
    let mut doc: DocumentMut = text.parse().context(ParseSnafu { path })?;
    let mut parts: Vec<&str> = key.split('.').collect();
    let leaf = parts.pop().filter(|l| !l.is_empty()).context(KeySnafu { key })?;
    let mut table: &mut Table = doc.as_table_mut();
    for part in parts {
        let item = table.entry(part).or_insert_with(|| Item::Table(Table::new()));
        table = item.as_table_mut().context(KeySnafu { key })?;
    }
    match toml_value(json) {
        Some(v) => {
            table.insert(leaf, value(v));
        }
        None => {
            table.remove(leaf);
        }
    }
    storage.write_atomic(path, doc.to_string()).context(WriteSnafu { path })?;
    Ok(())
}

/// Une valeur JSON simple en TOML ; `null` (et ce qui n'a pas d'équivalent) : rien.
fn toml_value(json: &Value) -> Option<toml_edit::Value> {
    match json {
        Value::Bool(b) => Some((*b).into()),
        Value::Number(n) => n.as_i64().map(Into::into).or_else(|| n.as_f64().map(Into::into)),
        Value::String(s) => Some(s.as_str().into()),
        Value::Array(items) => Some(toml_edit::Value::Array(items.iter().filter_map(toml_value).collect())),
        Value::Null | Value::Object(_) => None,
    }
}

#[cfg(test)]
#[path = "config_write_tests.rs"]
mod config_write_tests;
