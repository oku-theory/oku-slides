//! Un zip de sources vient d'un fichier : jamais fiable. Ni la taille
//! annoncée d'une entrée ni un contenu qui se décompresse en masse ne doivent
//! faire allouer sans borne.

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use oku_slides_core::{Deck, SourceBundle, SourceLimits};
use speculoos::prelude::*;
use zip::write::SimpleFileOptions;

/// Retient la plus grosse allocation demandée.
struct Largest;

static LARGEST: AtomicUsize = AtomicUsize::new(0);

// SAFETY : délègue tout à `System`, n'ajoute qu'un compteur atomique.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Largest {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        // SAFETY : même contrat que l'appel reçu.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY : même contrat que l'appel reçu.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LARGEST.fetch_max(new_size, Ordering::Relaxed);
        // SAFETY : même contrat que l'appel reçu.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Largest = Largest;

const MIB: usize = 1024 * 1024;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn sources_zip(assets: &[(&str, Vec<u8>)], method: zip::CompressionMethod) -> Res<Vec<u8>> {
    let deck = Deck::from_json(r#"{"slides":[]}"#)?;
    let loro = oku_slides_core::DeckReplica::new(&deck, common::shared())?.snapshot()?;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = SimpleFileOptions::default().compression_method(method);
        zw.start_file("manifest.json", opts)?;
        zw.write_all(br#"{"format":"oku-slides-sources","version":9}"#)?;
        zw.start_file("deck.loro", opts)?;
        zw.write_all(&loro)?;
        for (name, data) in assets {
            zw.start_file(format!("assets/{name}"), opts)?;
            zw.write_all(data)?;
        }
        zw.finish()?;
    }
    Ok(buf.into_inner())
}

/// Remplace la taille décompressée annoncée des entrées `assets/…` (en-têtes
/// locaux et répertoire central) par `size`.
fn lie_about_asset_sizes(zip: &mut [u8], size: u32) {
    let mut i = 0;
    while i + 4 <= zip.len() {
        // (champ taille, champ longueur du nom, début du nom)
        let fields = match &zip[i..i + 4] {
            [0x50, 0x4b, 0x03, 0x04] => Some((i + 22, i + 26, i + 30)),
            [0x50, 0x4b, 0x01, 0x02] => Some((i + 24, i + 28, i + 46)),
            _ => None,
        };
        if let Some((at, len_at, name_at)) = fields {
            let len = usize::from(u16::from_le_bytes([zip[len_at], zip[len_at + 1]]));
            if zip[name_at..name_at + len].starts_with(b"assets/") {
                zip[at..at + 4].copy_from_slice(&size.to_le_bytes());
            }
        }
        i += 1;
    }
}

#[test]
fn from_zip_entry_announcing_huge_size_reads_without_huge_allocation() {
    let mut zip = sources_zip(&[("ab", vec![7; 1000])], zip::CompressionMethod::Deflated).unwrap();
    lie_about_asset_sizes(&mut zip, u32::MAX);
    LARGEST.store(0, Ordering::Relaxed);
    let read = SourceBundle::from_zip(
        &zip,
        SourceLimits { replica_bytes: 32 * MIB, total_bytes: 256 * MIB },
        common::shared(),
    );
    let largest = LARGEST.load(Ordering::Relaxed);
    assert_that!(largest).is_less_than(16 * MIB);
    let (b, _) = read.unwrap();
    assert_that!(b.assets.get("sha256:ab").map(Vec::len)).is_equal_to(Some(1000));
}
