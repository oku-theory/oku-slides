//! Le blob des sources Écho dans un PDF.
//!
//! Le zip des sources est stocké comme un flux binaire privé, via le
//! dictionnaire de pièce de page du catalogue (ISO 32000, clause 14.5) :
//!
//! ```text
//! Catalog
//! └─ /PieceInfo
//!    └─ /Echo
//!       ├─ /LastModified (D:…)
//!       └─ /Private
//!          ├─ /Format (echo-sources)
//!          ├─ /Version 1
//!          ├─ /SHA256 (hex du zip)
//!          └─ /Sources → flux = zip des sources
//! ```
//!
//! Il n'apparaît pas dans le panneau des pièces jointes des lecteurs PDF.

use lopdf::{Dictionary, Document, Object, Stream, StringFormat};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PIECE_KEY: &[u8] = b"Echo";
pub const FORMAT: &str = "echo-sources";
pub const VERSION: i64 = 1;

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("PDF illisible : {0}")]
    Parse(#[from] lopdf::Error),
    #[error("écriture du PDF : {0}")]
    Io(#[from] std::io::Error),
    #[error("structure PDF inattendue : {0}")]
    Structure(&'static str),
    #[error("version de blob non prise en charge : {0}")]
    UnsupportedVersion(i64),
    #[error("le blob des sources est corrompu (empreinte différente)")]
    Corrupted,
}

/// Informations sur le blob trouvé dans un PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct Sources {
    /// Contenu du zip des sources.
    pub zip: Vec<u8>,
    pub version: i64,
    /// Date `/LastModified` du blob (format PDF `D:…`).
    pub last_modified: Option<String>,
    /// Date `/ModDate` du document, si présente.
    pub document_modified: Option<String>,
}

impl Sources {
    /// Vrai si le document a été modifié après l'écriture du blob par Écho.
    pub fn modified_elsewhere(&self) -> bool {
        match (&self.last_modified, &self.document_modified) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        }
    }
}

/// Date PDF (`D:AAAAMMJJhhmmssZ`) de l'instant présent, en UTC.
fn pdf_date() -> String {
    let format = time::macros::format_description!("D:[year][month][day][hour][minute][second]Z");
    time::OffsetDateTime::now_utc().format(format).unwrap_or_default()
}

fn text(s: &str) -> Object {
    Object::String(s.as_bytes().to_vec(), StringFormat::Literal)
}

fn sha256_hex(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> Result<&'a Object, PdfError> {
    match obj {
        Object::Reference(id) => Ok(doc.get_object(*id)?),
        o => Ok(o),
    }
}

fn catalog_id(doc: &Document) -> Result<lopdf::ObjectId, PdfError> {
    doc.trailer.get(b"Root").and_then(Object::as_reference).map_err(|_| PdfError::Structure("pas de catalogue"))
}

/// Ajoute (ou remplace) le blob des sources dans le PDF et renvoie le PDF modifié.
pub fn embed_sources(pdf: &[u8], zip: &[u8]) -> Result<Vec<u8>, PdfError> {
    let mut doc = Document::load_mem(pdf)?;
    let now = pdf_date();

    // Le zip est déjà compressé : pas de filtre supplémentaire.
    let stream = Stream::new(Dictionary::new(), zip.to_vec()).with_compression(false);
    let stream_id = doc.add_object(stream);

    let mut private = Dictionary::new();
    private.set("Format", text(FORMAT));
    private.set("Version", Object::Integer(VERSION));
    private.set("SHA256", text(&sha256_hex(zip)));
    private.set("Sources", Object::Reference(stream_id));

    let mut piece = Dictionary::new();
    piece.set("LastModified", text(&now));
    piece.set("Private", Object::Dictionary(private));

    let root = catalog_id(&doc)?;
    let catalog = doc.get_object_mut(root)?.as_dict_mut().map_err(|_| PdfError::Structure("catalogue invalide"))?;
    let mut piece_info = match catalog.get(b"PieceInfo") {
        Ok(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    piece_info.set(PIECE_KEY, Object::Dictionary(piece));
    catalog.set("PieceInfo", Object::Dictionary(piece_info));

    // La même date dans /ModDate : une divergence signalera une modification externe.
    let info_id = match doc.trailer.get(b"Info").and_then(Object::as_reference) {
        Ok(id) => id,
        Err(_) => {
            let id = doc.add_object(Dictionary::new());
            doc.trailer.set("Info", Object::Reference(id));
            id
        }
    };
    if let Ok(info) = doc.get_object_mut(info_id).and_then(Object::as_dict_mut) {
        info.set("ModDate", text(&now));
        info.set("EchoFormat", text(&format!("{FORMAT}/{VERSION}")));
    }

    // Supprime l'ancien flux s'il y en avait un (réenregistrement).
    doc.prune_objects();
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

/// Lit le blob des sources. `Ok(None)` : ce PDF n'a pas été créé par Écho.
pub fn extract_sources(pdf: &[u8]) -> Result<Option<Sources>, PdfError> {
    let doc = Document::load_mem(pdf)?;
    let root = catalog_id(&doc)?;
    let catalog = doc.get_object(root)?.as_dict().map_err(|_| PdfError::Structure("catalogue invalide"))?;

    let Ok(piece_info) = catalog.get(b"PieceInfo") else { return Ok(None) };
    let Ok(piece_info) = resolve(&doc, piece_info)?.as_dict() else { return Ok(None) };
    let Ok(piece) = piece_info.get(PIECE_KEY) else { return Ok(None) };
    let piece = resolve(&doc, piece)?.as_dict().map_err(|_| PdfError::Structure("/Echo invalide"))?;
    let private = resolve(&doc, piece.get(b"Private").map_err(|_| PdfError::Structure("/Private absent"))?)?
        .as_dict()
        .map_err(|_| PdfError::Structure("/Private invalide"))?;

    let version = private.get(b"Version").and_then(Object::as_i64).unwrap_or(0);
    if version > VERSION || version < 1 {
        return Err(PdfError::UnsupportedVersion(version));
    }
    let stream = resolve(&doc, private.get(b"Sources").map_err(|_| PdfError::Structure("/Sources absent"))?)?
        .as_stream()
        .map_err(|_| PdfError::Structure("/Sources n'est pas un flux"))?;
    let zip = if stream.dict.get(b"Filter").is_ok() { stream.decompressed_content()? } else { stream.content.clone() };

    if let Ok(expected) = private.get(b"SHA256").and_then(Object::as_str)
        && expected != sha256_hex(&zip).as_bytes()
    {
        return Err(PdfError::Corrupted);
    }

    let as_string = |o: Result<&Object, lopdf::Error>| -> Option<String> {
        o.ok()
            .and_then(|o| resolve(&doc, o).ok())
            .and_then(|o| o.as_str().ok())
            .map(|b| String::from_utf8_lossy(b).into_owned())
    };
    let last_modified = as_string(piece.get(b"LastModified"));
    let document_modified = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| resolve(&doc, o).ok())
        .and_then(|o| o.as_dict().ok())
        .and_then(|info| as_string(info.get(b"ModDate")));

    Ok(Some(Sources { zip, version, last_modified, document_modified }))
}

/// Copie du PDF sans le blob des sources (export « sans sources »).
pub fn strip_sources(pdf: &[u8]) -> Result<Vec<u8>, PdfError> {
    let mut doc = Document::load_mem(pdf)?;
    let root = catalog_id(&doc)?;
    if let Ok(catalog) = doc.get_object_mut(root).and_then(Object::as_dict_mut) {
        let empty = match catalog.get_mut(b"PieceInfo") {
            Ok(Object::Dictionary(d)) => {
                d.remove(PIECE_KEY);
                d.is_empty()
            }
            _ => false,
        };
        if empty {
            catalog.remove(b"PieceInfo");
        }
    }
    doc.prune_objects();
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod lib_tests {
    use super::pdf_date;

    #[test]
    fn pdf_date_now_follows_pdf_date_syntax() {
        let d = pdf_date();
        assert_eq!(d.len(), "D:20260925120000Z".len(), "{d}");
        assert!(d.starts_with("D:20") && d.ends_with('Z'), "{d}");
        assert!(d[2..16].bytes().all(|b| b.is_ascii_digit()), "{d}");
    }
}
