//! Conteneur Embedded OpenType (EOT), format des `.fntdata` de PowerPoint.
//!
//! Seules les données non compressées sont lues ; le MicroType (compression
//! MTX) n'est pas décodé : la police est alors signalée illisible.

const MAGIC: u16 = 0x504C;
/// `TTEMBED_TTCOMPRESSED` : données compressées en MicroType.
pub(crate) const COMPRESSED: u32 = 0x0000_0004;
/// `TTEMBED_XORENCRYPTDATA` : données XOR 0x50.
pub(crate) const XOR: u32 = 0x1000_0000;

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at.checked_add(4)?)?.try_into().ok().map(u32::from_le_bytes)
}

/// Police (sfnt) d'un conteneur EOT, ou `None` s'il est compressé ou incohérent.
pub(crate) fn decode(data: &[u8]) -> Option<Vec<u8>> {
    let eot_size = usize::try_from(u32_at(data, 0)?).ok()?;
    let font_size = usize::try_from(u32_at(data, 4)?).ok()?;
    let flags = u32_at(data, 12)?;
    let magic = data.get(34..36)?.try_into().ok().map(u16::from_le_bytes)?;
    if magic != MAGIC || eot_size > data.len() || font_size > eot_size || flags & COMPRESSED != 0 {
        return None;
    }
    let font = data.get(eot_size - font_size..eot_size)?;
    Some(if flags & XOR != 0 { font.iter().map(|b| b ^ 0x50).collect() } else { font.to_vec() })
}

#[cfg(test)]
#[path = "eot_tests.rs"]
mod eot_tests;
