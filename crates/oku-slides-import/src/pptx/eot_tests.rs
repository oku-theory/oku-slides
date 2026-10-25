use speculoos::prelude::*;

use super::*;

const HEADER: usize = 82;

fn sfnt() -> Vec<u8> {
    let mut v = vec![0x00, 0x01, 0x00, 0x00];
    v.extend(0u8..60);
    v
}

/// Conteneur EOT minimal autour de `font`.
fn eot(font: &[u8], flags: u32) -> Vec<u8> {
    let mut v = vec![0u8; HEADER];
    v[0..4].copy_from_slice(&u32::try_from(HEADER + font.len()).unwrap().to_le_bytes());
    v[4..8].copy_from_slice(&u32::try_from(font.len()).unwrap().to_le_bytes());
    v[8..12].copy_from_slice(&0x0002_0001u32.to_le_bytes());
    v[12..16].copy_from_slice(&flags.to_le_bytes());
    v[34..36].copy_from_slice(&0x504Cu16.to_le_bytes());
    if flags & XOR != 0 {
        v.extend(font.iter().map(|b| b ^ 0x50));
    } else {
        v.extend_from_slice(font);
    }
    v
}

#[test]
fn decode_plain_eot_returns_the_font() {
    assert_that!(decode(&eot(&sfnt(), 0))).is_equal_to(Some(sfnt()));
}

#[test]
fn decode_xor_eot_undoes_the_xor() {
    assert_that!(decode(&eot(&sfnt(), XOR))).is_equal_to(Some(sfnt()));
}

#[test]
fn decode_compressed_eot_is_none() {
    assert_that!(decode(&eot(&sfnt(), COMPRESSED))).is_none();
}

#[test]
fn decode_without_magic_is_none() {
    let mut v = eot(&sfnt(), 0);
    v[34] = 0;
    assert_that!(decode(&v)).is_none();
}

#[test]
fn decode_with_inconsistent_sizes_is_none() {
    let mut v = eot(&sfnt(), 0);
    v[0..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_that!((decode(&v), decode(&v[..20]))).is_equal_to((None, None));
}

#[test]
fn decode_with_font_size_beyond_data_allocates_nothing_and_is_none() {
    // `font_size` forgé à 4 Gio : la police est une tranche des octets reçus, jamais une allocation annoncée.
    let mut v = eot(&sfnt(), 0);
    v[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_that!(decode(&v)).is_none();
}
