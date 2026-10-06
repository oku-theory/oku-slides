//! Couleurs des jetons de design : la forme hexadécimale admise.

/// Vrai pour `#rgb`, `#rrggbb` ou `#rrggbbaa` (chiffres hexadécimaux ASCII).
pub fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#').is_some_and(|h| matches!(h.len(), 3 | 6 | 8) && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod color_tests;
