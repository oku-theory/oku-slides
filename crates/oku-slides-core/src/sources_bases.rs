//! Data bases in the sources zip: one `bases/<hex>.parquet` entry per base the deck uses now.
//!
//! A base is never held in memory: it is copied from the store into a saved zip, and streamed
//! from an opened zip into the store, which checks its hash ([`BaseFiles`]).

use std::io::Read;

use oku_slides_data::BaseId;

/// A failure of the base store, as [`BaseFiles`] reports it.
pub type BaseFilesError = Box<dyn std::error::Error + Send + Sync>;

/// Where the bases of a zip are read from and written to: the platform's base store.
pub trait BaseFiles {
    /// Opens base `id` to copy it into a saved zip.
    ///
    /// # Errors
    ///
    /// The store lacks it, or cannot read it.
    fn open(&self, id: &BaseId) -> Result<Box<dyn Read + '_>, BaseFilesError>;

    /// Stores base `id` read from `r`, at most `max` bytes, refusing content that does not
    /// hash to `id`.
    ///
    /// # Errors
    ///
    /// Too large, a hash mismatch, or a store failure.
    fn put(&self, r: &mut dyn Read, id: &BaseId, max: u64) -> Result<(), BaseFilesError>;
}

#[cfg(test)]
#[path = "sources_bases_tests.rs"]
pub(crate) mod sources_bases_tests;
