//! Item order of a pack version: resolution, normalisation, check.

use snafu::ensure;

use crate::error::{ManifestOrderSnafu, PackManifestError};
use crate::{PackFiles, WidgetFile};

/// The items of `files`: those of `order` first, then the rest by name.
#[must_use]
pub fn item_order<'a>(order: &[WidgetFile], files: &'a PackFiles) -> Vec<&'a WidgetFile> {
    resolve(order, files.widgets().keys())
}

/// `order` ready to write: missing names dropped, unlisted appended; empty when in name order.
#[must_use]
pub fn normalized_order<'a>(
    order: &[WidgetFile],
    names: impl Iterator<Item = &'a WidgetFile> + Clone,
) -> Vec<WidgetFile> {
    let resolved = resolve(order, names.clone());
    let by_name: Vec<&WidgetFile> = names.filter(|n| !n.is_common()).collect();
    if resolved == by_name { Vec::new() } else { resolved.into_iter().cloned().collect() }
}

/// The items of `names` listed in `order` (first occurrence only), then the others by name.
fn resolve<'n>(order: &[WidgetFile], names: impl Iterator<Item = &'n WidgetFile> + Clone) -> Vec<&'n WidgetFile> {
    let mut out: Vec<&WidgetFile> = Vec::new();
    for o in order {
        if o.is_common() || out.contains(&o) {
            continue;
        }
        out.extend(names.clone().find(|n| *n == o));
    }
    out.extend(names.filter(|n| !n.is_common() && !order.contains(n)));
    out
}

/// Refuses an `order` that repeats a name, lists `_commun` or a file `files` lacks.
///
/// # Errors
///
/// The first offending name ([`PackManifestError::order_name`]).
pub(crate) fn check_order(order: &[WidgetFile], files: &PackFiles) -> Result<(), PackManifestError> {
    for (i, name) in order.iter().enumerate() {
        let known = !name.is_common() && files.widgets().contains_key(name);
        ensure!(known && !order[..i].contains(name), ManifestOrderSnafu { name: name.as_str() });
    }
    Ok(())
}

#[cfg(test)]
#[path = "pack_order_tests.rs"]
mod pack_order_tests;
