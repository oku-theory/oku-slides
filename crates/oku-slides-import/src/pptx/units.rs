//! Unités OOXML.

/// EMU → pt (12 700 EMU par pt).
pub(crate) fn emu(v: i64) -> f64 {
    v as f64 / 12_700.0
}

/// 60 000ᵉ de degré → degrés.
pub(crate) fn angle(v: i64) -> f64 {
    v as f64 / 60_000.0
}

/// 100 000ᵉ → fraction.
pub(crate) fn pct(v: i64) -> f64 {
    v as f64 / 100_000.0
}

/// Centièmes → unités (`sz`, `spc`, `spcPts`).
pub(crate) fn hundredths(v: i64) -> f64 {
    v as f64 / 100.0
}
