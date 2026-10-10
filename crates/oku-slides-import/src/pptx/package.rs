//! Paquet OPC : zip borné, relations (en cache), types de contenu.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::rc::Rc;

use crate::ReadLimits;
use crate::error::{ImportError, io_error, not_a_presentation, protected, structure, too_large, utf8_error, zip_error};
use crate::pptx::xml::parse;

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
const ZIP64_LOCATOR: [u8; 4] = [0x50, 0x4b, 0x06, 0x07];
const PRESENTATION_TYPES: [&str; 4] = [
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
    "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
];

/// Un `.pptx` ouvert sur ses octets ; chaque lecture compte dans les limites.
///
/// L'archive est clonée à chaque lecture (métadonnées en `Arc`, lecteur sur `&[u8]`) plutôt
/// que gardée dans un `RefCell` : `Package<'b>` reste covariant en `'b`, ce qui laisse les
/// contextes de part (`PartCtx<'p>`) raccourcir leurs durées de vie.
#[derive(Debug)]
pub(crate) struct Package<'b> {
    zip: zip::ZipArchive<Cursor<&'b [u8]>>,
    limits: ReadLimits,
    read_total: Cell<u64>,
    /// Une lecture a dépassé une limite : la lecture est perdue, plus rien n'est décompressé.
    exhausted: Cell<bool>,
    defaults: BTreeMap<String, String>,
    overrides: BTreeMap<String, String>,
    rels: RefCell<BTreeMap<String, Rc<Rels>>>,
    main: String,
}

/// Relations d'une part, cibles résolues en chemins du paquet.
#[derive(Debug, Default)]
pub(crate) struct Rels {
    internal: Vec<(String, String)>, // (type, cible), dans l'ordre du fichier
    by_id: BTreeMap<String, usize>,
    external: BTreeMap<String, String>,
}

impl Rels {
    /// Cible interne de la relation `id`.
    pub(crate) fn target(&self, id: &str) -> Option<&str> {
        self.by_id.get(id).and_then(|i| self.internal.get(*i)).map(|(_, t)| t.as_str())
    }

    /// Relations internes dont le type finit par `suffix` (`"/slide"`…), dans l'ordre du fichier.
    pub(crate) fn by_type(&self, suffix: &str) -> Vec<(&str, &str)> {
        let ids: BTreeMap<usize, &str> = self.by_id.iter().map(|(id, i)| (*i, id.as_str())).collect();
        self.internal
            .iter()
            .enumerate()
            .filter(|(_, (ty, _))| ty.ends_with(suffix))
            .filter_map(|(i, (_, t))| Some((*ids.get(&i)?, t.as_str())))
            .collect()
    }

    /// Cible d'une relation externe (lien, média lié).
    pub(crate) fn external(&self, id: &str) -> Option<&str> {
        self.external.get(id).map(String::as_str)
    }
}

/// `%XX` décodés (cibles de relation : ce sont des URI).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| char::from(c).to_digit(16);
        match (b[i], b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(h), Some(l)) => {
                out.push(u8::try_from(h * 16 + l).unwrap_or(b'?'));
                i += 3;
            }
            (c, _, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Chemin d'une cible relative à la part `base_part` (les `..` au-delà de la racine sont ignorés).
pub(crate) fn resolve_path(base_part: &str, target: &str) -> String {
    let target = percent_decode(target);
    if let Some(abs) = target.strip_prefix('/') {
        return abs.to_owned();
    }
    let mut segs: Vec<&str> = base_part.split('/').collect();
    segs.pop();
    for s in target.split('/') {
        match s {
            ".." => {
                segs.pop();
            }
            "." | "" => {}
            s => segs.push(s),
        }
    }
    segs.join("/")
}

/// Nombre d'entrées annoncé par la fin du répertoire central (Zip64 compris), sans l'indexer.
pub(crate) fn declared_entries(bytes: &[u8]) -> Option<u64> {
    let from = bytes.len().saturating_sub(65_557);
    let at = bytes.get(from..)?.windows(4).rposition(|w| w == EOCD)? + from;
    let le16 = |o: usize| bytes.get(o..o + 2).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes);
    let entries = le16(at + 10)?;
    if entries != 0xFFFF {
        return Some(u64::from(entries));
    }
    // Zip64 : le localisateur précède l'EOCD et pointe vers l'enregistrement Zip64.
    let loc = at.checked_sub(20)?;
    if bytes.get(loc..loc + 4)? != ZIP64_LOCATOR {
        return None;
    }
    let off = usize::try_from(u64::from_le_bytes(bytes.get(loc + 8..loc + 16)?.try_into().ok()?)).ok()?;
    Some(u64::from_le_bytes(bytes.get(off.checked_add(32)?..off.checked_add(40)?)?.try_into().ok()?))
}

fn rels_path(part: &str) -> String {
    match part.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{part}.rels"),
    }
}

impl<'b> Package<'b> {
    /// Ouvre le paquet et vérifie que c'est une présentation.
    pub(crate) fn open(bytes: &'b [u8], limits: &ReadLimits) -> Result<Self, ImportError> {
        if bytes.len() as u64 > limits.max_input_bytes {
            return Err(too_large("input"));
        }
        if bytes.starts_with(&CFB_MAGIC) {
            return Err(protected());
        }
        if declared_entries(bytes).is_some_and(|n| n > limits.max_parts as u64) {
            return Err(too_large("entries"));
        }
        let zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| zip_error("archive", e))?;
        if zip.len() > limits.max_parts {
            return Err(too_large("entries"));
        }
        let mut pkg = Self {
            zip,
            limits: *limits,
            read_total: Cell::new(0),
            exhausted: Cell::new(false),
            defaults: BTreeMap::new(),
            overrides: BTreeMap::new(),
            rels: RefCell::new(BTreeMap::new()),
            main: String::new(),
        };
        pkg.read_content_types()?;
        let root = pkg.read_rels("_rels/.rels", "")?;
        let main = root.by_type("/officeDocument").first().map(|(_, t)| (*t).to_owned());
        let main = main.ok_or_else(|| structure("_rels/.rels: no officeDocument"))?;
        let ct = pkg.content_type(&main).unwrap_or_default().to_owned();
        if !PRESENTATION_TYPES.contains(&ct.as_str()) {
            return Err(not_a_presentation(&ct));
        }
        pkg.main = main;
        Ok(pkg)
    }

    /// Les limites de lecture.
    pub(crate) fn limits(&self) -> &ReadLimits {
        &self.limits
    }

    /// Chemin de la part principale (`ppt/presentation.xml` d'ordinaire).
    pub(crate) fn main_part(&self) -> &str {
        &self.main
    }

    /// Vrai si la part existe (sans lecture).
    pub(crate) fn exists(&self, part: &str) -> bool {
        self.zip.index_for_name(part).is_some()
    }

    /// Octets d'une part, bornés par les limites.
    ///
    /// Une lecture ne décompresse jamais plus que `min(max_part_bytes, budget restant) + 1` octets,
    /// qui comptent dans le total même en cas d'échec ; après un dépassement, toute lecture échoue
    /// sans rien décompresser (une bombe référencée mille fois n'est décompressée qu'une fois).
    pub(crate) fn bytes(&self, part: &str) -> Result<Vec<u8>, ImportError> {
        if self.exhausted.get() {
            return Err(too_large("total"));
        }
        let mut zip = self.zip.clone();
        let file = zip.by_name(part).map_err(|e| zip_error(part, e))?;
        let cap = self.limits.max_part_bytes;
        let remaining = self.limits.max_total_bytes.saturating_sub(self.read_total.get());
        let mut out = Vec::new();
        // En cas d'erreur, `read_to_end` garde dans `out` ce qui a déjà été décompressé.
        let read = file.take(cap.min(remaining).saturating_add(1)).read_to_end(&mut out);
        let n = out.len() as u64;
        self.read_total.set(self.read_total.get().saturating_add(n));
        read.map_err(|e| io_error(part, e))?;
        let over = if n > cap {
            Some(too_large(part))
        } else if n > remaining {
            Some(too_large("total"))
        } else {
            None
        };
        if let Some(e) = over {
            self.exhausted.set(true);
            return Err(e);
        }
        Ok(out)
    }

    /// Texte UTF-8 d'une part (BOM retiré).
    pub(crate) fn text(&self, part: &str) -> Result<String, ImportError> {
        let s = String::from_utf8(self.bytes(part)?).map_err(|e| utf8_error(part, e))?;
        Ok(s.strip_prefix('\u{feff}').map(str::to_owned).unwrap_or(s))
    }

    /// Relations de `part` (lues une fois) ; vides si la part n'a pas de fichier de relations.
    pub(crate) fn rels(&self, part: &str) -> Result<Rc<Rels>, ImportError> {
        if let Some(r) = self.rels.borrow().get(part) {
            return Ok(Rc::clone(r));
        }
        let r = self.read_rels(&rels_path(part), part)?;
        self.rels.borrow_mut().insert(part.to_owned(), Rc::clone(&r));
        Ok(r)
    }

    /// Type de contenu d'une part (override, sinon défaut par extension).
    pub(crate) fn content_type(&self, part: &str) -> Option<&str> {
        if let Some(t) = self.overrides.get(part) {
            return Some(t);
        }
        let ext = part.rsplit_once('.')?.1.to_ascii_lowercase();
        self.defaults.get(&ext).map(String::as_str)
    }

    fn read_rels(&self, rels_part: &str, base: &str) -> Result<Rc<Rels>, ImportError> {
        if !self.exists(rels_part) {
            return Ok(Rc::new(Rels::default()));
        }
        let xml = self.text(rels_part)?;
        let doc = parse(&xml, rels_part, self.limits.max_xml_nodes)?;
        let mut rels = Rels::default();
        for r in doc.root_element().children().filter(|n| n.tag_name().name() == "Relationship") {
            let (Some(id), Some(ty), Some(target)) = (r.attribute("Id"), r.attribute("Type"), r.attribute("Target"))
            else {
                continue;
            };
            if r.attribute("TargetMode") == Some("External") {
                rels.external.insert(id.to_owned(), target.to_owned());
            } else if !rels.by_id.contains_key(id) {
                let path = if base.is_empty() {
                    percent_decode(target.trim_start_matches('/'))
                } else {
                    resolve_path(base, target)
                };
                rels.by_id.insert(id.to_owned(), rels.internal.len());
                rels.internal.push((ty.to_owned(), path));
            }
        }
        Ok(Rc::new(rels))
    }

    fn read_content_types(&mut self) -> Result<(), ImportError> {
        let xml = self.text("[Content_Types].xml")?;
        let doc = parse(&xml, "[Content_Types].xml", self.limits.max_xml_nodes)?;
        for n in doc.root_element().children().filter(roxmltree::Node::is_element) {
            match (n.tag_name().name(), n.attribute("ContentType")) {
                ("Default", Some(ct)) => {
                    if let Some(ext) = n.attribute("Extension") {
                        self.defaults.insert(ext.to_ascii_lowercase(), ct.to_owned());
                    }
                }
                ("Override", Some(ct)) => {
                    if let Some(p) = n.attribute("PartName") {
                        self.overrides.insert(p.trim_start_matches('/').to_owned(), ct.to_owned());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "package_tests.rs"]
mod package_tests;
