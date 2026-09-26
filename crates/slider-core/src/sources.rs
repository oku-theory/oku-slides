//! Le zip des sources (format `oku-slides-sources`) : deck, assets d'origine et
//! polices. C'est ce blob qui est stocké dans le PDF.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use sha2::{Digest, Sha256};
use snafu::{OptionExt, ResultExt, ensure};
use zip::write::SimpleFileOptions;

use crate::error::{
    BadNameSnafu, IoSnafu, JsonSnafu, MissingDeckSnafu, NotSourcesSnafu, SourcesError, SourcesInner,
    UnsupportedVersionSnafu, ZipSnafu,
};
use crate::model::Deck;

pub const SOURCES_FORMAT: &str = "oku-slides-sources";
pub const SOURCES_VERSION: u32 = 1;

/// Identifiant d'un asset : `sha256:<hex>` de son contenu.
pub fn asset_id(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceBundle {
    pub deck: Deck,
    /// Assets par identifiant (`sha256:…`).
    pub assets: BTreeMap<String, Vec<u8>>,
    /// Polices par nom de fichier.
    pub fonts: BTreeMap<String, Vec<u8>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Manifest {
    format: String,
    version: u32,
}

fn safe_name(name: &str) -> Result<&str, SourcesInner> {
    let ok = !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
        && !name.starts_with('.');
    ensure!(ok, BadNameSnafu { name });
    Ok(name)
}

/// Nom de fichier d'un asset : l'empreinte hex de `sha256:<hex>`, rien d'autre.
fn asset_name(id: &str) -> Result<&str, SourcesInner> {
    let hex = id.strip_prefix("sha256:").unwrap_or(id);
    ensure!(!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()), BadNameSnafu { name: id });
    Ok(hex)
}

impl SourceBundle {
    pub fn new(deck: Deck) -> Self {
        Self { deck, assets: BTreeMap::new(), fonts: BTreeMap::new() }
    }

    /// Ajoute un asset et renvoie son identifiant.
    pub fn add_asset(&mut self, bytes: Vec<u8>) -> String {
        let id = asset_id(&bytes);
        self.assets.insert(id.clone(), bytes);
        id
    }

    pub fn to_zip(&self) -> Result<Vec<u8>, SourcesError> {
        Ok(self.write_zip()?)
    }

    fn write_zip(&self) -> Result<Vec<u8>, SourcesInner> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let deflate = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            // Images et polices sont déjà compressées : on les stocke tels quels.
            let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

            zw.start_file("manifest.json", deflate).context(ZipSnafu)?;
            let manifest = Manifest { format: SOURCES_FORMAT.into(), version: SOURCES_VERSION };
            let manifest = serde_json::to_vec(&manifest).context(JsonSnafu { file: "manifest.json" })?;
            zw.write_all(&manifest).context(IoSnafu)?;

            zw.start_file("deck.json", deflate).context(ZipSnafu)?;
            zw.write_all(self.deck.to_json().context(JsonSnafu { file: "deck.json" })?.as_bytes()).context(IoSnafu)?;

            for (id, bytes) in &self.assets {
                zw.start_file(format!("assets/{}", asset_name(id)?), stored).context(ZipSnafu)?;
                zw.write_all(bytes).context(IoSnafu)?;
            }
            for (name, bytes) in &self.fonts {
                zw.start_file(format!("fonts/{}", safe_name(name)?), stored).context(ZipSnafu)?;
                zw.write_all(bytes).context(IoSnafu)?;
            }
            zw.finish().context(ZipSnafu)?;
        }
        Ok(buf.into_inner())
    }

    pub fn from_zip(bytes: &[u8]) -> Result<Self, SourcesError> {
        Ok(Self::read_zip(bytes)?)
    }

    fn read_zip(bytes: &[u8]) -> Result<Self, SourcesInner> {
        let mut za = zip::ZipArchive::new(Cursor::new(bytes)).context(ZipSnafu)?;
        let read = |za: &mut zip::ZipArchive<Cursor<&[u8]>>, i: usize| -> Result<(String, Vec<u8>), SourcesInner> {
            let mut f = za.by_index(i).context(ZipSnafu)?;
            let name = f.name().to_owned();
            let mut data = Vec::with_capacity(usize::try_from(f.size()).unwrap_or_default());
            f.read_to_end(&mut data).context(IoSnafu)?;
            Ok((name, data))
        };

        let mut manifest: Option<Manifest> = None;
        let mut deck: Option<Deck> = None;
        let (mut assets, mut fonts) = (BTreeMap::new(), BTreeMap::new());
        for i in 0..za.len() {
            let (name, data) = read(&mut za, i)?;
            match name.as_str() {
                "manifest.json" => {
                    manifest = Some(serde_json::from_slice(&data).context(JsonSnafu { file: "manifest.json" })?)
                }
                "deck.json" => deck = Some(serde_json::from_slice(&data).context(JsonSnafu { file: "deck.json" })?),
                n if n.starts_with("assets/") => {
                    let hex = asset_name(&n["assets/".len()..])?;
                    assets.insert(format!("sha256:{hex}"), data);
                }
                n if n.starts_with("fonts/") => {
                    let file = safe_name(&n["fonts/".len()..])?.to_owned();
                    fonts.insert(file, data);
                }
                _ => {} // Entrées inconnues : ignorées (compatibilité ascendante).
            }
        }
        let manifest = manifest.context(NotSourcesSnafu)?;
        ensure!(manifest.format == SOURCES_FORMAT, NotSourcesSnafu);
        ensure!(manifest.version <= SOURCES_VERSION, UnsupportedVersionSnafu { version: manifest.version });
        let deck = deck.context(MissingDeckSnafu)?;
        Ok(SourceBundle { deck, assets, fonts })
    }
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod sources_tests;
