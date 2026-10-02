//! Le blob des sources Oku Slides dans un PDF.
//!
//! Le zip des sources est stocké comme un flux binaire privé, via le
//! dictionnaire de pièce de page du catalogue (ISO 32000, clause 14.5) :
//!
//! ```text
//! Catalog
//! └─ /PieceInfo
//!    └─ /OkuSlides
//!       ├─ /LastModified (D:…)
//!       └─ /Private
//!          ├─ /Format (oku-slides-sources)
//!          ├─ /Version 2
//!          ├─ /SHA256 (hex du zip)
//!          └─ /Sources → flux = zip des sources
//! ```
//!
//! Le zip des sources (version 4) :
//!
//! ```text
//! manifest.json          { format: "oku-slides-sources", version: 4 }
//! deck.loro              snapshot complet du replica Loro du deck
//! assets/<hex>           tels quels
//! fonts/<nom>            tels quels
//! packs/<hex>/pack.json  version de pack de modèles (hex = sha-256 du contenu canonique)
//! packs/<hex>/widgets.typ  absent si le pack n'a pas de widgets
//! ```
//!
//! Une enveloppe plus ancienne (`/Version 1`, `deck.json`) est ignorée :
//! [`extract_sources`] renvoie `Ok(None)`, comme pour un PDF d'ailleurs.
//!
//! Il n'apparaît pas dans le panneau des pièces jointes des lecteurs PDF.

use lopdf::{Dictionary, Document, Object, Stream, StringFormat};
use oku_slides_i18n::{Localize, Localizer};
use sha2::{Digest, Sha256};
use snafu::{Backtrace, ResultExt, Snafu, ensure};

pub const PIECE_KEY: &[u8] = b"OkuSlides";
pub const FORMAT: &str = "oku-slides-sources";
pub const VERSION: i64 = 2;

/// Un PDF illisible, ou des sources Oku Slides abîmées.
#[derive(Debug, Snafu)]
pub struct PdfError(PdfInner);

/// Nature d'une [`PdfError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PdfErrorKind {
    /// Le PDF ne se lit pas, ou pas comme attendu.
    Unreadable,
    /// Écrit par une version plus récente d'Oku Slides.
    UnsupportedVersion,
    /// Le blob ne correspond plus à son empreinte.
    Corrupted,
    /// Le PDF modifié n'a pas pu être écrit.
    Write,
}

impl PdfError {
    pub fn kind(&self) -> PdfErrorKind {
        match self.0 {
            PdfInner::Parse { .. } | PdfInner::Structure { .. } => PdfErrorKind::Unreadable,
            PdfInner::UnsupportedVersion { .. } => PdfErrorKind::UnsupportedVersion,
            PdfInner::Corrupted { .. } => PdfErrorKind::Corrupted,
            PdfInner::Write { .. } => PdfErrorKind::Write,
        }
    }
}

impl Localize for PdfError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(match self.kind() {
            PdfErrorKind::Unreadable => "pdf-unreadable",
            PdfErrorKind::UnsupportedVersion => "pdf-unsupported-version",
            PdfErrorKind::Corrupted => "pdf-corrupted",
            PdfErrorKind::Write => "pdf-write",
        })
    }
}

#[derive(Debug, Snafu)]
enum PdfInner {
    #[snafu(display("unreadable PDF"))]
    Parse { source: lopdf::Error, backtrace: Backtrace },
    #[snafu(display("unexpected PDF structure: {what}"))]
    Structure { what: &'static str, source: lopdf::Error, backtrace: Backtrace },
    #[snafu(display("unsupported sources version: {version}"))]
    UnsupportedVersion { version: i64, backtrace: Backtrace },
    #[snafu(display("the sources blob is corrupted (fingerprint mismatch)"))]
    Corrupted { backtrace: Backtrace },
    #[snafu(display("could not write the PDF"))]
    Write { source: std::io::Error, backtrace: Backtrace },
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
    /// Vrai si le document a été modifié après l'écriture du blob par Oku Slides.
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
        Object::Reference(id) => Ok(doc.get_object(*id).context(StructureSnafu { what: "dangling reference" })?),
        o => Ok(o),
    }
}

fn catalog_id(doc: &Document) -> Result<lopdf::ObjectId, PdfError> {
    Ok(doc.trailer.get(b"Root").and_then(Object::as_reference).context(StructureSnafu { what: "no catalog" })?)
}

/// Ajoute (ou remplace) le blob des sources dans le PDF et renvoie le PDF modifié.
pub fn embed_sources(pdf: &[u8], zip: &[u8]) -> Result<Vec<u8>, PdfError> {
    let mut doc = Document::load_mem(pdf).context(ParseSnafu)?;
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
    let catalog =
        doc.get_object_mut(root).and_then(Object::as_dict_mut).context(StructureSnafu { what: "invalid catalog" })?;
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
        info.set("OkuSlidesFormat", text(&format!("{FORMAT}/{VERSION}")));
    }

    // Supprime l'ancien flux s'il y en avait un (réenregistrement).
    doc.prune_objects();
    let mut out = Vec::new();
    doc.save_to(&mut out).context(WriteSnafu)?;
    Ok(out)
}

/// Lit le blob des sources. `Ok(None)` : ce PDF n'a pas été créé par Oku Slides,
/// ou ses sources sont d'un format plus ancien, qui ne se lit plus.
pub fn extract_sources(pdf: &[u8]) -> Result<Option<Sources>, PdfError> {
    let doc = Document::load_mem(pdf).context(ParseSnafu)?;
    let root = catalog_id(&doc)?;
    let catalog = doc.get_object(root).and_then(Object::as_dict).context(StructureSnafu { what: "invalid catalog" })?;

    let Ok(piece_info) = catalog.get(b"PieceInfo") else { return Ok(None) };
    let Ok(piece_info) = resolve(&doc, piece_info)?.as_dict() else { return Ok(None) };
    let Ok(piece) = piece_info.get(PIECE_KEY) else { return Ok(None) };
    let piece = resolve(&doc, piece)?.as_dict().context(StructureSnafu { what: "invalid /OkuSlides" })?;
    let private = resolve(&doc, piece.get(b"Private").context(StructureSnafu { what: "missing /Private" })?)?
        .as_dict()
        .context(StructureSnafu { what: "invalid /Private" })?;

    let version = private.get(b"Version").and_then(Object::as_i64).unwrap_or(0);
    // Plus ancien : format ignoré (pas de conversion, spec t-188).
    if version < VERSION {
        return Ok(None);
    }
    ensure!(version == VERSION, UnsupportedVersionSnafu { version });
    let stream = resolve(&doc, private.get(b"Sources").context(StructureSnafu { what: "missing /Sources" })?)?
        .as_stream()
        .context(StructureSnafu { what: "/Sources is not a stream" })?;
    let zip = if stream.dict.get(b"Filter").is_ok() {
        stream.decompressed_content().context(StructureSnafu { what: "undecodable /Sources" })?
    } else {
        stream.content.clone()
    };

    if let Ok(expected) = private.get(b"SHA256").and_then(Object::as_str)
        && expected != sha256_hex(&zip).as_bytes()
    {
        return CorruptedSnafu.fail()?;
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
    let mut doc = Document::load_mem(pdf).context(ParseSnafu)?;
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
    doc.save_to(&mut out).context(WriteSnafu)?;
    Ok(out)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
