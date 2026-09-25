//! Le zip des sources (format `echo-sources`) : deck, assets d'origine,
//! polices et historique. C'est ce blob qui est stocké dans le PDF.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use sha2::{Digest, Sha256};
use thiserror::Error;
use zip::write::SimpleFileOptions;

use crate::model::Deck;

pub const SOURCES_FORMAT: &str = "echo-sources";
pub const SOURCES_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum SourcesError {
    #[error("zip illisible : {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON invalide dans {file} : {source}")]
    Json { file: String, source: serde_json::Error },
    #[error("ce n'est pas un zip de sources Écho")]
    NotSlider,
    #[error("version de format non prise en charge : {0}")]
    UnsupportedVersion(u32),
    #[error("nom de fichier refusé : {0}")]
    BadName(String),
    #[error("deck.json absent")]
    MissingDeck,
}

/// Identifiant d'un asset : `sha256:<hex>` de son contenu.
pub fn asset_id(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceBundle {
    pub deck: Option<Deck>,
    /// Assets par identifiant (`sha256:…`).
    pub assets: BTreeMap<String, Vec<u8>>,
    /// Polices par nom de fichier.
    pub fonts: BTreeMap<String, Vec<u8>>,
    /// Journal des transactions (JSON Lines), optionnel.
    pub history: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Manifest {
    format: String,
    version: u32,
}

fn safe_name(name: &str) -> Result<&str, SourcesError> {
    let ok = !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
        && !name.starts_with('.');
    if ok { Ok(name) } else { Err(SourcesError::BadName(name.to_owned())) }
}

impl SourceBundle {
    pub fn new(deck: Deck) -> Self {
        Self { deck: Some(deck), ..Default::default() }
    }

    /// Ajoute un asset et renvoie son identifiant.
    pub fn add_asset(&mut self, bytes: Vec<u8>) -> String {
        let id = asset_id(&bytes);
        self.assets.insert(id.clone(), bytes);
        id
    }

    pub fn to_zip(&self) -> Result<Vec<u8>, SourcesError> {
        let deck = self.deck.as_ref().ok_or(SourcesError::MissingDeck)?;
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let deflate = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            // Images et polices sont déjà compressées : on les stocke tels quels.
            let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

            zw.start_file("manifest.json", deflate)?;
            let manifest = Manifest { format: SOURCES_FORMAT.into(), version: SOURCES_VERSION };
            zw.write_all(serde_json::to_string(&manifest).expect("sérialisable").as_bytes())?;

            zw.start_file("deck.json", deflate)?;
            zw.write_all(deck.to_json().as_bytes())?;

            for (id, bytes) in &self.assets {
                let hex = id.strip_prefix("sha256:").unwrap_or(id);
                zw.start_file(format!("assets/{}", safe_name(hex)?), stored)?;
                zw.write_all(bytes)?;
            }
            for (name, bytes) in &self.fonts {
                zw.start_file(format!("fonts/{}", safe_name(name)?), stored)?;
                zw.write_all(bytes)?;
            }
            if let Some(h) = &self.history {
                zw.start_file("history.jsonl", deflate)?;
                zw.write_all(h.as_bytes())?;
            }
            zw.finish()?;
        }
        Ok(buf.into_inner())
    }

    pub fn from_zip(bytes: &[u8]) -> Result<Self, SourcesError> {
        let mut za = zip::ZipArchive::new(Cursor::new(bytes))?;
        let read = |za: &mut zip::ZipArchive<Cursor<&[u8]>>, i: usize| -> Result<(String, Vec<u8>), SourcesError> {
            let mut f = za.by_index(i)?;
            let name = f.name().to_owned();
            let mut data = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut data)?;
            Ok((name, data))
        };

        let mut manifest: Option<Manifest> = None;
        let mut out = SourceBundle::default();
        for i in 0..za.len() {
            let (name, data) = read(&mut za, i)?;
            match name.as_str() {
                "manifest.json" => {
                    manifest =
                        Some(serde_json::from_slice(&data).map_err(|e| SourcesError::Json { file: name, source: e })?)
                }
                "deck.json" => {
                    out.deck =
                        Some(serde_json::from_slice(&data).map_err(|e| SourcesError::Json { file: name, source: e })?)
                }
                "history.jsonl" => out.history = Some(String::from_utf8_lossy(&data).into_owned()),
                n if n.starts_with("assets/") => {
                    let hex = safe_name(&n["assets/".len()..])?;
                    out.assets.insert(format!("sha256:{hex}"), data);
                }
                n if n.starts_with("fonts/") => {
                    let file = safe_name(&n["fonts/".len()..])?.to_owned();
                    out.fonts.insert(file, data);
                }
                _ => {} // Entrées inconnues : ignorées (compatibilité ascendante).
            }
        }
        let manifest = manifest.ok_or(SourcesError::NotSlider)?;
        if manifest.format != SOURCES_FORMAT {
            return Err(SourcesError::NotSlider);
        }
        if manifest.version > SOURCES_VERSION {
            return Err(SourcesError::UnsupportedVersion(manifest.version));
        }
        if out.deck.is_none() {
            return Err(SourcesError::MissingDeck);
        }
        Ok(out)
    }
}
