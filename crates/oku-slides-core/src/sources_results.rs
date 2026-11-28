//! The query result cache of the sources zip: one `data/<key hex>.json` per result, naming its
//! base, which goes in `bases/` with the manual data's bases and counts toward
//! [`MAX_DECK_BASE_BYTES`](oku_slides_data::limits::MAX_DECK_BASE_BYTES).
//!
//! A cache entry is a hint, never trusted: one that does not check (shape, bounds, its base
//! received with the size it states, columns matching the base) is dropped and logged, and its
//! query computes again. The session keeps an entry only when the key it computes from the
//! opened deck names it.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, Write};

use oku_slides_data::limits::{MAX_COLUMNS, MAX_DATA, MAX_ROWS};
use oku_slides_data::{BaseId, BaseRef, Column, bounded, valid_id};
use snafu::ResultExt;
use zip::write::SimpleFileOptions;

use crate::error::{IoSnafu, SourcesInner, ZipSnafu};
use crate::sources_bases::BaseFiles;

/// Folder of the cache entries in the zip: `data/<key hex>.json`.
pub(crate) const RESULTS_DIR: &str = "data/";

/// Largest cache entry, written or read.
pub(crate) const MAX_RESULT_ENTRY: usize = 64 * 1024;

/// A result key as 64 lowercase hexadecimal characters: the name of its cache entry.
pub type ResultKeyHex = String;

/// A query result as the deck caches it: its key, its columns and its base.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CachedResult {
    /// The key it was computed for, as its entry is named.
    pub key: ResultKeyHex,
    /// Its columns, in base order; each name is its id.
    #[serde(deserialize_with = "bounded::vec::<_, _, { MAX_COLUMNS }>")]
    pub columns: Vec<Column>,
    /// Its base, in `bases/`.
    pub base: BaseRef,
}

/// Whether `key` is 64 lowercase hexadecimal characters.
fn key_hex(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Writes the cache entries of `results` in key order, each result's base copied from `bases`
/// into `bases/` unless `written` holds it already. `used` is what the written bases weigh; a
/// result whose base would take it past `budget`, or whose base or entry cannot be written, is
/// left out (logged): its query computes again on open.
pub(crate) fn write_results<W: Write + Seek>(
    zw: &mut zip::ZipWriter<W>,
    results: &BTreeMap<ResultKeyHex, CachedResult>,
    bases: Option<&dyn BaseFiles>,
    options: (SimpleFileOptions, SimpleFileOptions),
    (written, mut used): (&mut BTreeSet<BaseId>, u64),
    budget: u64,
) -> Result<(), SourcesInner> {
    let (stored, deflate) = options;
    for (hex, result) in results.iter().take(MAX_DATA) {
        let skip = |why: &str| tracing::warn!(key = %hex, "query result not cached: {why}");
        if !key_hex(hex) || result.key != *hex {
            skip("bad key");
            continue;
        }
        let Ok(json) = serde_json::to_vec(result) else {
            skip("not serialized");
            continue;
        };
        if json.len() > MAX_RESULT_ENTRY {
            skip("entry too large");
            continue;
        }
        let id = &result.base.id;
        if !written.contains(id) {
            if used.saturating_add(result.base.bytes) > budget {
                skip("past the deck's base budget");
                continue;
            }
            let Some(mut r) = bases.and_then(|store| store.open(id).ok()) else {
                skip("base unreadable");
                continue;
            };
            zw.start_file(format!("bases/{}.parquet", id.hex()), stored).context(ZipSnafu)?;
            used = used.saturating_add(std::io::copy(&mut r, zw).context(IoSnafu)?);
            written.insert(id.clone());
        }
        zw.start_file(format!("{RESULTS_DIR}{hex}.json"), deflate).context(ZipSnafu)?;
        zw.write_all(&json).context(IoSnafu)?;
    }
    Ok(())
}

/// The cache entries an opened zip holds, before they are checked against its bases.
#[derive(Default)]
pub(crate) struct RawResults(BTreeMap<ResultKeyHex, Vec<u8>>);

impl RawResults {
    /// Reads entry `file` (`<key hex>.json`) of announced size `size`, within the zip's
    /// `budget`; a bad name, an entry past [`MAX_RESULT_ENTRY`] or [`MAX_DATA`], or a
    /// duplicate is dropped (logged).
    pub(crate) fn put<R: Read>(
        &mut self,
        file: &str,
        f: R,
        size: usize,
        read: impl FnOnce(R, usize) -> Result<Vec<u8>, SourcesInner>,
    ) -> Result<(), SourcesInner> {
        let drop = |why: &str| tracing::warn!(entry = %file, "query result cache entry dropped: {why}");
        let Some(hex) = file.strip_suffix(".json").filter(|h| key_hex(h)) else {
            drop("bad name");
            return Ok(());
        };
        if self.0.len() >= MAX_DATA {
            drop("too many entries");
            return Ok(());
        }
        if size > MAX_RESULT_ENTRY {
            drop("too large");
            return Ok(());
        }
        let data = read(f, MAX_RESULT_ENTRY + 1)?;
        if data.len() > MAX_RESULT_ENTRY {
            drop("too large");
            return Ok(());
        }
        if self.0.insert(hex.to_owned(), data).is_some() {
            drop("duplicate");
            self.0.remove(hex);
        }
        Ok(())
    }

    /// The entries that check against `received`, the bases the zip put in the store with their
    /// sizes; the others are dropped (logged).
    pub(crate) fn verify(self, received: &BTreeMap<BaseId, u64>) -> BTreeMap<ResultKeyHex, CachedResult> {
        self.0
            .into_iter()
            .filter_map(|(hex, data)| match check(&hex, &data, received) {
                Ok(result) => Some((hex, result)),
                Err(why) => {
                    tracing::warn!(key = %hex, "query result cache entry dropped: {why}");
                    None
                }
            })
            .collect()
    }
}

/// The entry `data` named `hex`, once it checks against `received`; otherwise why not.
fn check(hex: &str, data: &[u8], received: &BTreeMap<BaseId, u64>) -> Result<CachedResult, &'static str> {
    let result: CachedResult = serde_json::from_slice(data).map_err(|_| "unreadable")?;
    let base = &result.base;
    if result.key != hex {
        return Err("key unlike its name");
    }
    if received.get(&base.id) != Some(&base.bytes) {
        return Err("base not received at its size");
    }
    if usize::try_from(base.rows).map_or(true, |rows| rows > MAX_ROWS) {
        return Err("too many rows");
    }
    let fits = result.columns.len() == base.columns.len()
        && result.columns.iter().zip(&base.columns).all(|(column, stored)| {
            valid_id(&column.id) && column.name == column.id && column.id == stored.id && column.ty == stored.ty
        });
    let distinct = result.columns.iter().map(|c| &c.id).collect::<BTreeSet<_>>().len() == result.columns.len();
    if !fits || !distinct {
        return Err("columns unlike the base's");
    }
    Ok(result)
}

#[cfg(test)]
#[path = "sources_results_tests.rs"]
mod sources_results_tests;
