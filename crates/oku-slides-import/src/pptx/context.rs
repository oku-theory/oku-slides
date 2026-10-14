//! Contexte de lecture : ressources partagées par tout le deck, et contexte d'une part.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use roxmltree::Document;

use crate::ImportErrorKind;
use crate::collect::{Sink, clip, clip_to};
use crate::error::ImportError;
use crate::pptx::cascade::TextEnv;
use crate::pptx::color::ColorCtx;
use crate::pptx::media::{SVG, sniff};
use crate::pptx::package::{Package, Rels};
use crate::pptx::placeholder::{Inherited, PhInheritance, PhKey};
use crate::pptx::svg::sanitize;
use crate::pptx::text::{LevelProps, RunProps};
use crate::pptx::theme::ThemeData;
use crate::pptx::xml::parse;
use crate::source::{DRAWABLE, WarningKind};

const LINK_SCHEMES: [&str; 3] = ["http://", "https://", "mailto:"];
const MAX_LINK_CHARS: usize = 2048;
const MAX_RAW_ID: usize = 64;

/// Ressources lues une fois pour tout le deck.
#[derive(Debug)]
pub(crate) struct Shared<'x> {
    pub pkg: &'x Package<'x>,
    /// `tableStyles.xml`, analysé une fois.
    pub table_styles: Option<Document<'x>>,
    /// `defaultTextStyle` de `presentation.xml` (couleurs non résolues).
    pub pres: LevelProps,
    pub sink: RefCell<Sink>,
}

impl<'x> Shared<'x> {
    pub(crate) fn new(pkg: &'x Package<'x>, table_styles: Option<Document<'x>>, pres: LevelProps) -> Self {
        Self { pkg, table_styles, pres, sink: RefCell::new(Sink::new()) }
    }
}

/// Ce qui distingue une part : propriétaire des ids, slide, couleurs, thème, héritage.
#[derive(Debug)]
pub(crate) struct PartSpec<'p> {
    pub owner: String,
    pub slide: Option<u32>,
    pub colors: ColorCtx,
    pub theme: &'p ThemeData<'p>,
    pub inherit: Option<&'p PhInheritance<'p>>,
}

/// Contexte de lecture d'une part.
#[derive(Debug)]
pub(crate) struct PartCtx<'p> {
    pub shared: &'p Shared<'p>,
    pub rels: Rc<Rels>,
    pub owner: String,
    pub slide: Option<u32>,
    pub colors: ColorCtx,
    pub theme: &'p ThemeData<'p>,
    pub inherit: Option<&'p PhInheritance<'p>>,
    ids: RefCell<BTreeSet<String>>,
    /// Prochain suffixe à essayer par id de base : `unique` reste linéaire face à des milliers de doublons.
    next: RefCell<BTreeMap<String, usize>>,
}

impl<'p> PartCtx<'p> {
    /// Contexte de la part `part` (ses relations sont lues ici).
    pub(crate) fn new(shared: &'p Shared<'p>, part: &str, spec: PartSpec<'p>) -> Result<Self, ImportError> {
        Ok(Self::with_rels(shared, shared.pkg.rels(part)?, spec))
    }

    fn with_rels(shared: &'p Shared<'p>, rels: Rc<Rels>, spec: PartSpec<'p>) -> Self {
        Self {
            shared,
            rels,
            owner: spec.owner,
            slide: spec.slide,
            colors: spec.colors,
            theme: spec.theme,
            inherit: spec.inherit,
            ids: RefCell::new(BTreeSet::new()),
            next: RefCell::new(BTreeMap::new()),
        }
    }

    /// Contexte d'une part rattachée (dessin de SmartArt) : même deck, slide, couleurs et thème ; ids neufs, sans héritage.
    pub(crate) fn sub(&self, rels: Rc<Rels>, owner: String) -> PartCtx<'p> {
        let spec = PartSpec { owner, slide: self.slide, colors: self.colors.clone(), theme: self.theme, inherit: None };
        Self::with_rels(self.shared, rels, spec)
    }

    pub(crate) fn pkg(&self) -> &'p Package<'p> {
        self.shared.pkg
    }

    /// Taille maximale d'une part, qui borne aussi un SVG re-sérialisé.
    pub(crate) fn max_part_bytes(&self) -> usize {
        // Saturé : une borne au-delà de l'espace adressable ne borne rien de plus.
        usize::try_from(self.shared.pkg.limits().max_part_bytes).unwrap_or(usize::MAX)
    }

    /// Limite de nœuds d'une part XML.
    pub(crate) fn nodes(&self) -> u32 {
        self.shared.pkg.limits().max_xml_nodes
    }

    /// Héritage d'une forme ; sans héritage de placeholder : les défauts de la présentation.
    pub(crate) fn lookup(&self, ph: Option<&PhKey>) -> Inherited {
        match self.inherit {
            Some(i) => i.lookup(ph),
            None => Inherited { levels: vec![self.shared.pres.clone()], ..Inherited::default() },
        }
    }

    fn unique(&self, base: String) -> String {
        let mut ids = self.ids.borrow_mut();
        if ids.insert(base.clone()) {
            return base;
        }
        let mut next = self.next.borrow_mut();
        let n = next.entry(base.clone()).or_insert(2);
        loop {
            let candidate = format!("{base}~{n}");
            *n += 1;
            if ids.insert(candidate.clone()) {
                return candidate;
            }
        }
    }

    /// Identifiant d'item `"<owner>/<id>"` : `cNvPr@id` entier (sinon `x`), rendu unique (`~2`, `~3`…).
    pub(crate) fn item_id(&self, cnv_id: &str) -> String {
        let id = cnv_id.trim().parse::<u32>().map_or_else(|_| "x".to_owned(), |n| n.to_string());
        self.unique(format!("{}/{id}", self.owner))
    }

    /// Identifiant d'item depuis une clé libre (`modelId` de SmartArt) : `[A-Za-z0-9{}-]`, 64 caractères.
    pub(crate) fn item_id_raw(&self, key: &str) -> String {
        let safe: String = key
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '{' | '}' | '-'))
            .take(MAX_RAW_ID)
            .collect();
        self.unique(format!("{}/{}", self.owner, if safe.is_empty() { "x" } else { safe.as_str() }))
    }

    /// Note un avertissement.
    pub(crate) fn warn(&self, item: Option<&str>, kind: WarningKind) {
        self.shared.sink.borrow_mut().warn(self.slide, item, kind);
    }

    /// Note une erreur fatale (limite dépassée), rendue après la part.
    pub(crate) fn fail(&self, e: ImportError) {
        self.shared.sink.borrow_mut().set_fatal(e);
    }

    /// Asset de la relation `rid` (lu une fois par chemin, échec compris), typé par son contenu, SVG
    /// assaini ; `None` si absent, illisible ou SVG rejeté.
    pub(crate) fn asset(&self, rid: &str) -> Option<String> {
        let target = self.rels.target(rid)?.to_owned();
        {
            let sink = self.shared.sink.borrow();
            if let Some(id) = sink.cached_asset(&target) {
                return Some(id);
            }
            if sink.asset_failed(&target) {
                return None;
            }
        }
        let bytes = match self.pkg().bytes(&target) {
            Ok(b) => b,
            Err(e) => {
                self.shared.sink.borrow_mut().fail_asset(&target);
                if e.kind() == ImportErrorKind::TooLarge {
                    self.fail(e);
                } else {
                    self.warn(None, WarningKind::MissingPart(clip(&target)));
                }
                return None;
            }
        };
        let mime = sniff(&bytes);
        let bytes = if mime == SVG {
            let clean = sanitize(&bytes, &target, self.nodes(), self.max_part_bytes());
            let Some(clean) = self.soft(clean, None, WarningKind::UnsafeSvg(clip(&target))) else {
                self.shared.sink.borrow_mut().fail_asset(&target);
                return None;
            };
            if clean.altered {
                self.warn(None, WarningKind::UnsafeSvg(clip(&target)));
            }
            clean.bytes
        } else {
            if !DRAWABLE.contains(&mime) {
                self.warn(None, WarningKind::UnsupportedImage(clip(&target)));
            }
            bytes
        };
        Some(self.shared.sink.borrow_mut().intern(&target, bytes, mime))
    }

    /// Cible d'une relation externe (image liée…), rendue sûre pour un message.
    pub(crate) fn external_target(&self, rid: &str) -> Option<String> {
        self.rels.external(rid).map(clip)
    }

    /// Cible d'un lien externe, si elle est sûre : `http`, `https`, `mailto`, ≤ 2048 caractères, sans contrôle ni espace.
    pub(crate) fn link(&self, rid: &str) -> Option<String> {
        let target = self.rels.external(rid)?.trim();
        let lower = target.to_ascii_lowercase();
        let clean = !target.chars().any(|c| c.is_control() || c.is_whitespace());
        if clean && target.chars().count() <= MAX_LINK_CHARS && LINK_SCHEMES.iter().any(|s| lower.starts_with(s)) {
            return Some(target.to_owned());
        }
        self.warn(None, WarningKind::UnsafeLink(clip_to(target, 80)));
        None
    }

    /// Résultat d'une lecture liée : une limite dépassée devient fatale (`None`), toute autre erreur
    /// rend `None` et note `warn` ; seule forme de ce « lire, sinon dégrader » dans les parts.
    pub(crate) fn soft<T>(&self, r: Result<T, ImportError>, item: Option<&str>, warn: WarningKind) -> Option<T> {
        match r {
            Ok(v) => Some(v),
            Err(e) if e.kind() == ImportErrorKind::TooLarge => {
                self.fail(e);
                None
            }
            Err(_) => {
                self.warn(item, warn);
                None
            }
        }
    }

    /// Texte d'une part liée : `None` (et `MissingPart`) si elle manque ; une limite dépassée devient fatale.
    pub(crate) fn part_text(&self, part: &str, item: &str) -> Option<String> {
        self.soft(self.pkg().text(part), Some(item), WarningKind::MissingPart(clip(part)))
    }

    /// Analyse d'une part liée (graphique, SmartArt, notes) : `None` et `warn` si elle est illisible ;
    /// une limite dépassée (nœuds) devient fatale.
    pub(crate) fn linked_doc<'a>(
        &self,
        xml: &'a str,
        part: &str,
        item: Option<&str>,
        warn: WarningKind,
    ) -> Option<Document<'a>> {
        self.soft(parse(xml, part, self.nodes()), item, warn)
    }

    /// Environnement de la cascade pour cette part.
    pub(crate) fn env(&self, font_ref: RunProps) -> TextEnv<'_> {
        TextEnv {
            major: &self.theme.theme.fonts.major,
            minor: &self.theme.theme.fonts.minor,
            colors: &self.colors,
            font_ref,
        }
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
