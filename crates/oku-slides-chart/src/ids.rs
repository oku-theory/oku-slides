//! Row and column identifiers of a `dataset`: aligned, well formed, unique.

use std::collections::HashSet;

use oku_slides_data::{Dataset, valid_id};
use oku_slides_i18n::Message;
use snafu::ensure;

use crate::error::{InconsistentSnafu, Result};

/// True if no identifier appears twice in `list`.
pub(crate) fn all_unique(list: &[String]) -> bool {
    let mut seen = HashSet::with_capacity(list.len());
    list.iter().all(|id| seen.insert(id.as_str()))
}

/// Rejects identifiers that are misaligned, malformed or repeated.
///
/// Empty lists are accepted: the dataset then has no identifiers.
pub(crate) fn validate_ids(d: &Dataset) -> Result<()> {
    let width = d.source.first().map_or(0, Vec::len);
    let aligned =
        (d.ids.is_empty() || d.ids.len() == d.source.len()) && (d.columns.is_empty() || d.columns.len() == width);
    ensure!(aligned, InconsistentSnafu { reason: Message::new("chart-ids-length") });
    ensure!(
        d.ids.iter().chain(&d.columns).all(|id| valid_id(id)),
        InconsistentSnafu { reason: Message::new("chart-ids-invalid") }
    );
    ensure!(
        all_unique(&d.ids) && all_unique(&d.columns),
        InconsistentSnafu { reason: Message::new("chart-ids-duplicate") }
    );
    Ok(())
}
