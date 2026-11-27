//! Paquet OPC : zip borné, relations (en cache), types de contenu.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::bounded::BoundedZip;
use crate::bounded::percent_decode;
pub(crate) use crate::bounded::resolve_path;
use crate::error::{ImportError, not_a_presentation, structure};
use crate::pptx::xml::parse;
use crate::{ImportErrorKind, ReadLimits};

const PRESENTATION_TYPES: [&str; 4] = [
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
    "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
];

/// A `.pptx` opened on its bytes, over a [`BoundedZip`]: every read counts against the limits,
/// and `Package<'b>` stays covariant in `'b`, which lets part contexts (`PartCtx<'p>`) shorten
/// their lifetimes.
#[derive(Debug)]
pub(crate) struct Package<'b> {
    zip: BoundedZip<'b>,
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

fn rels_path(part: &str) -> String {
    match part.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{part}.rels"),
    }
}

impl<'b> Package<'b> {
    /// Ouvre le paquet et vérifie que c'est une présentation.
    pub(crate) fn open(bytes: &'b [u8], limits: &ReadLimits) -> Result<Self, ImportError> {
        let mut pkg = Self {
            zip: BoundedZip::open(bytes, limits)?,
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

    /// Octets décompressés jusqu'ici, lectures échouées comprises.
    #[cfg(test)]
    pub(crate) fn decompressed(&self) -> u64 {
        self.zip.decompressed()
    }

    /// Les limites de lecture.
    pub(crate) fn limits(&self) -> &ReadLimits {
        self.zip.limits()
    }

    /// Chemin de la part principale (`ppt/presentation.xml` d'ordinaire).
    pub(crate) fn main_part(&self) -> &str {
        &self.main
    }

    /// Vrai si la part existe (sans lecture).
    pub(crate) fn exists(&self, part: &str) -> bool {
        self.zip.exists(part)
    }

    /// Bytes of a part, bounded by the limits ([`BoundedZip::read`]).
    pub(crate) fn bytes(&self, part: &str) -> Result<Vec<u8>, ImportError> {
        self.zip.read(part)
    }

    /// UTF-8 text of a part, its BOM removed.
    pub(crate) fn text(&self, part: &str) -> Result<String, ImportError> {
        self.zip.text(part)
    }

    /// Relations de `part` (lues une fois) ; vides si la part n'a pas de fichier de relations.
    ///
    /// Un fichier illisible rend l'erreur une seule fois (à signaler) : les appels suivants le
    /// voient vide, sans le relire. Une limite dépassée n'est pas retenue (elle est fatale).
    pub(crate) fn rels(&self, part: &str) -> Result<Rc<Rels>, ImportError> {
        if let Some(r) = self.rels.borrow().get(part) {
            return Ok(Rc::clone(r));
        }
        let r = self.read_rels(&rels_path(part), part);
        let cached = match &r {
            Ok(r) => Rc::clone(r),
            Err(e) if e.kind() == ImportErrorKind::TooLarge => return r,
            Err(_) => Rc::new(Rels::default()),
        };
        self.rels.borrow_mut().insert(part.to_owned(), cached);
        r
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
        let doc = parse(&xml, rels_part, self.zip.limits().max_xml_nodes)?;
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
        let doc = parse(&xml, "[Content_Types].xml", self.zip.limits().max_xml_nodes)?;
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
