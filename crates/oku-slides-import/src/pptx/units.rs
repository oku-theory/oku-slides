//! Unités OOXML.

use oku_slides_core::num::int_f64;

/// EMU → pt (12 700 EMU par pt).
pub(crate) fn emu(v: i64) -> f64 {
    int_f64(v) / 12_700.0
}

/// 60 000ᵉ de degré → degrés.
pub(crate) fn angle(v: i64) -> f64 {
    int_f64(v) / 60_000.0
}

/// 100 000ᵉ → fraction.
pub(crate) fn pct(v: i64) -> f64 {
    int_f64(v) / 100_000.0
}

/// Centièmes → unités (`sz`, `spc`, `spcPts`).
pub(crate) fn hundredths(v: i64) -> f64 {
    int_f64(v) / 100.0
}
