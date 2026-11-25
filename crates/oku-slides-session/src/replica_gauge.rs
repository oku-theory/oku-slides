//! The replica-full guard: growing ops stop at 90 % of `[sources] max_replica_mib`.
//!
//! Opening refuses a replica past its cap, so the app keeps a margin below it. Exporting the
//! replica at each op would cost too much: the gauge adds each batch's JSON size to the last
//! measure, and exports again only once that estimate reaches the threshold.

use snafu::ResultExt;

use oku_slides_core::{Op, SourcesError};

use crate::error::{ReplicaFullSnafu, SourcesSnafu};
use crate::{Result, Session};

/// One mebibyte, the unit of `[sources] max_replica_mib`.
const MIB: u64 = 1024 * 1024;

/// Whether a replica of `bytes` is at or past 90 % of `max`.
#[must_use]
pub(crate) fn replica_full(bytes: u64, max: u64) -> bool {
    bytes.saturating_mul(10) >= max.saturating_mul(9)
}

/// The last measured replica size and the JSON bytes applied since.
#[derive(Debug, Default)]
pub(crate) struct ReplicaGauge {
    measured: Option<u64>,
    pending: u64,
}

/// Whether `ops` may grow the replica: anything but removals.
fn grows(ops: &[Op]) -> bool {
    !ops.iter().all(|op| {
        matches!(op, Op::RemoveElement { .. } | Op::RemoveSlide { .. } | Op::RemovePack { .. } | Op::RemoveData { .. })
    })
}

impl Session {
    /// Refuses growing `ops` once the replica nears its cap (`op-replica-full`).
    ///
    /// # Errors
    ///
    /// `ReplicaFull` at 90 % of the cap; `Sources` when the replica does not export.
    pub(crate) fn ensure_replica_room(&mut self, ops: &[Op]) -> Result<()> {
        if !grows(ops) {
            return Ok(());
        }
        let max = self.source_limits.replica_bytes as u64;
        // Unserializable ops (never expected) are weighed by the exact measure alone.
        let added = serde_json::to_vec(ops).map(|v| v.len() as u64);
        if let Err(e) = &added {
            tracing::warn!(error = %e, "ops not serialized: measuring the replica");
        }
        let (exact, added) = (added.is_err(), added.unwrap_or(0));
        let gauge = &mut self.replica_gauge;
        let estimate = gauge.measured.unwrap_or(0).saturating_add(gauge.pending).saturating_add(added);
        if !exact && gauge.measured.is_some() && !replica_full(estimate, max) {
            gauge.pending = gauge.pending.saturating_add(added);
            return Ok(());
        }
        let snapshot = self.doc.sources.replica().snapshot().map_err(SourcesError::from).context(SourcesSnafu)?;
        let measured = snapshot.len() as u64;
        self.replica_gauge = ReplicaGauge { measured: Some(measured), pending: 0 };
        if replica_full(measured.saturating_add(added), max) {
            tracing::warn!(measured, added, max, "growing ops refused: the replica is near its cap");
            return ReplicaFullSnafu { max_mib: max / MIB }.fail()?;
        }
        self.replica_gauge.pending = added;
        Ok(())
    }
}

#[cfg(test)]
#[path = "replica_gauge_tests.rs"]
mod replica_gauge_tests;
