//! The bounds of an import: reading the package (zip bombs, huge XML) and the size of the deck made.

use serde::{Deserialize, Serialize};

/// Bounds on reading an Office package or PDF (an imported deck, a knowledge document); the
/// caller derives them from its configuration.
///
/// Serializable: the session sends them to the extraction worker with each document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadLimits {
    /// Largest file received.
    pub max_input_bytes: u64,
    /// Largest decompressed size of one part.
    pub max_part_bytes: u64,
    /// Largest decompressed size of everything read, summed.
    pub max_total_bytes: u64,
    /// Most entries in the zip.
    pub max_parts: usize,
    /// Most nodes of one XML part.
    pub max_xml_nodes: u32,
    /// Most slides read.
    pub max_slides: usize,
    /// Most pixels of one raster image, read from its header before any decoding.
    ///
    /// An image past it, or one whose header does not read, is not imported.
    pub max_image_pixels: u64,
    /// Rough budget of a plan's whole output, checked by `ImportPlan::check_budget`.
    ///
    /// Unique assets, embedded fonts, serialized ops, the generated theme and pack: an early
    /// bound on what is produced, not a promise that the deck will reopen.
    pub max_output_bytes: u64,
    /// Rough budget of the ops serialized as JSON, which end up in the Loro history.
    ///
    /// Checked by `ImportPlan::check_budget`: an early bound, the history's real size may differ.
    pub max_replica_bytes: u64,
}

impl Default for ReadLimits {
    /// Conservative bounds for a real deck.
    fn default() -> Self {
        Self {
            // The default of `[import] max_input_mib`, which the app and the CLI use instead.
            max_input_bytes: 256 << 20,
            max_part_bytes: 64 << 20,
            max_total_bytes: 512 << 20,
            max_parts: 10_000,
            max_xml_nodes: 5_000_000,
            max_slides: 2_000,
            // 64 Mpx: 256 MiB once decoded as RGBA, past any common camera photo.
            max_image_pixels: 1 << 26,
            // The defaults of `[sources] max_sources_mib` and `max_replica_mib`, the reopening bounds.
            max_output_bytes: 1024 << 20,
            max_replica_bytes: 64 << 20,
        }
    }
}

impl ReadLimits {
    /// The limits for reading a knowledge document of at most `max_input_bytes` in a worker of
    /// `memory_bytes`: all it decodes within a quarter of that memory, one part within an
    /// eighth, at most [`Self::EXTRACT_XML_NODES`] nodes per XML part, never past the defaults.
    ///
    /// Past them the document is refused as too large, never left to the worker's memory kill.
    #[must_use]
    pub fn for_extract(max_input_bytes: u64, memory_bytes: u64) -> Self {
        let d = Self::default();
        Self {
            max_input_bytes,
            max_total_bytes: d.max_total_bytes.min(memory_bytes / 4),
            max_part_bytes: d.max_part_bytes.min(memory_bytes / 8),
            max_xml_nodes: d.max_xml_nodes.min(Self::EXTRACT_XML_NODES),
            ..d
        }
    }

    /// Most nodes of one XML part of a knowledge document: a parsed node takes about a hundred
    /// bytes, so this keeps a part's tree near 100 MiB.
    pub const EXTRACT_XML_NODES: u32 = 1_000_000;
}

#[cfg(test)]
#[path = "limits_tests.rs"]
mod limits_tests;
