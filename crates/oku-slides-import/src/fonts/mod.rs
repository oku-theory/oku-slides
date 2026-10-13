//! Polices du hi-fi : pour chaque famille de la présentation, la police que
//! le deck emploiera et les fichiers à embarquer dans son dossier `fonts/`.
//!
//! Ordre de recherche ([`resolve`]) : faces embarquées dans le PPTX → police
//! disponible ([`FontCatalog`] : polices de l'app et du système, que
//! l'appelant énumère) → substitut aux métriques compatibles ([`SUBSTITUTES`],
//! livrés dans `assets/fonts`) → aucune : le texte prend la police du corps
//! du thème. Une face aux droits « Restricted License » n'est jamais
//! embarquée, ni un fichier trop gros ([`MAX_EMBEDDED_FILE`],
//! [`MAX_EMBEDDED_TOTAL`]) : la recherche passe à l'étape suivante.
//!
//! Si la police du corps du thème est elle-même introuvable,
//! [`FontPlan::settle_fallback`] retient une police de l'app sans empattement
//! ([`FALLBACKS`]) ; sans aucune, le rendu prend la police par défaut de son
//! moteur, et le rapport le dit.
//!
//! Seules les faces absentes de l'app partent dans le deck (embarquées dans
//! le PPTX, ou du système) : les polices de l'app, substituts compris, sont
//! présentes partout où le deck s'ouvre.
//!
//! Module pur : l'appelant lit les fichiers, ce module lit leurs octets.

mod face;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

use serde::Serialize;

use crate::source::{FaceStyle, SourceFont};

/// Substituts aux métriques compatibles, par famille source en minuscules.
///
/// « Calibri Light » n'a pas de pendant maigre : Carlito garde ses métriques,
/// pas sa graisse.
pub const SUBSTITUTES: [(&str, &str); 9] = [
    ("arial", "Liberation Sans"),
    ("calibri", "Carlito"),
    ("calibri light", "Carlito"),
    ("cambria", "Caladea"),
    ("courier", "Liberation Mono"),
    ("courier new", "Liberation Mono"),
    ("helvetica", "Liberation Sans"),
    ("times", "Liberation Serif"),
    ("times new roman", "Liberation Serif"),
];

/// Repli final, si la police du corps du thème est introuvable : polices de
/// l'app sans empattement, dans l'ordre de préférence.
pub const FALLBACKS: [&str; 3] = ["Liberation Sans", "Carlito", "Inter"];

/// Taille maximale d'un fichier de police embarqué dans le deck (32 Mio) :
/// une collection CJK ou emoji du système en pèse des dizaines.
pub const MAX_EMBEDDED_FILE: usize = 32 * 1024 * 1024;

/// Taille maximale de l'ensemble des polices embarquées dans le deck (64 Mio).
pub const MAX_EMBEDDED_TOTAL: usize = 64 * 1024 * 1024;

/// Longueur maximale du nom de famille dans un nom de fichier.
const MAX_SLUG: usize = 48;

/// D'où vient une police disponible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FontOrigin {
    /// Livrée avec l'app (`assets/fonts`) : jamais embarquée dans le deck.
    App,
    /// Installée sur le système : embarquée dans le deck.
    System,
}

/// Comment une famille source est rendue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FontDecision {
    /// Faces embarquées dans la présentation, reprises dans le deck.
    Embedded,
    /// La même famille est disponible.
    Available {
        /// Provenance.
        origin: FontOrigin,
    },
    /// Remplacée par un substitut aux métriques compatibles.
    Substituted {
        /// Provenance du substitut.
        origin: FontOrigin,
    },
    /// Introuvable : le texte prend la police de repli ([`FontChoice::fallback`]).
    Missing,
}

/// La décision pour une famille source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontChoice {
    /// Famille nommée par la présentation.
    pub family: String,
    /// Famille employée par le deck, telle que le rendu la nomme ; `None` si [`FontDecision::Missing`].
    pub used: Option<String>,
    /// Décision.
    pub decision: FontDecision,
    /// Des faces de cette famille existaient mais leurs droits interdisent de les embarquer.
    pub restricted: bool,
    /// Des faces de cette famille existaient mais leurs fichiers sont trop gros
    /// pour le deck ([`MAX_EMBEDDED_FILE`], [`MAX_EMBEDDED_TOTAL`]).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub too_large: bool,
    /// Pour une famille [`FontDecision::Missing`], la famille employée à sa
    /// place (la police du corps du thème, ou le repli final) ; `None` : la
    /// police par défaut du moteur de rendu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
}

/// Le résultat de [`resolve`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontPlan {
    /// Une décision par famille, triées par nom en minuscules.
    pub choices: Vec<FontChoice>,
    /// Fichiers du dossier `fonts/` du deck : nom sûr et déterministe → octets.
    pub files: BTreeMap<String, Vec<u8>>,
    /// Métriques verticales de chaque famille employée, par clé : celles de
    /// sa face normale (la première retenue), si elle les donne.
    pub(crate) metrics: BTreeMap<String, LineMetrics>,
    /// La famille que prend le texte d'une famille introuvable ([`Self::settle_fallback`]).
    pub fallback: Option<String>,
}

/// Métriques verticales d'une police, en em de sa taille.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineMetrics {
    /// Hauteur des capitales : le bord haut d'une ligne de texte Typst par défaut.
    pub cap_height: f64,
    /// Ascendante, au-dessus de la ligne de base.
    pub ascender: f64,
    /// Descendante, positive sous la ligne de base.
    pub descender: f64,
}

impl LineMetrics {
    /// Celles d'une police qui ne les donne pas : à peu près la moyenne des
    /// polices livrées (Carlito, Liberation Sans, Inter).
    pub const DEFAULT: Self = Self { cap_height: 0.7, ascender: 0.94, descender: 0.24 };
}

impl FontPlan {
    /// Famille employée pour la famille source `family` (casse ignorée).
    pub fn used(&self, family: &str) -> Option<&str> {
        let key = key(family);
        self.choices.iter().find(|c| self::key(&c.family) == key).and_then(|c| c.used.as_deref())
    }

    /// Métriques verticales de la police employée pour la famille source
    /// `family` (celles du repli pour une famille introuvable).
    pub(crate) fn metrics(&self, family: &str) -> Option<LineMetrics> {
        let used = self.used(family).or(self.fallback.as_deref())?;
        self.metrics.get(&key(used)).copied()
    }

    /// Retient la police de repli des familles introuvables : celle employée
    /// pour `body` (la police du corps du thème) ; si elle est introuvable
    /// aussi, la première de [`FALLBACKS`] que l'app livre (`catalog`) ;
    /// sinon aucune (le moteur de rendu prend la sienne). La note dans
    /// chaque choix [`FontDecision::Missing`].
    pub fn settle_fallback(&mut self, body: Option<&str>, catalog: &FontCatalog) {
        let used = body.and_then(|b| self.used(b)).map(str::to_owned);
        let fallback = used.or_else(|| {
            let picks = FALLBACKS.iter().find_map(|f| match catalog.lookup(f).0 {
                Some((FontOrigin::App, picks)) => Some(picks),
                _ => None,
            })?;
            let first = picks.first()?;
            if let Some(m) = first.metrics {
                self.metrics.entry(key(&first.family)).or_insert(m);
            }
            Some(first.family.clone())
        });
        for c in self.choices.iter_mut().filter(|c| c.decision == FontDecision::Missing) {
            c.fallback.clone_from(&fallback);
        }
        self.fallback = fallback;
    }
}

/// Les octets d'un fichier de police, partagés (sans copie : ceux du rendu, par exemple).
type Shared = Arc<dyn AsRef<[u8]> + Send + Sync>;

/// Une face disponible.
#[derive(Clone)]
struct CatalogFace {
    info: face::FaceInfo,
    origin: FontOrigin,
    /// Le fichier entier (une collection est partagée par ses faces).
    data: Shared,
}

impl std::fmt::Debug for CatalogFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = (*self.data).as_ref().len();
        f.debug_struct("CatalogFace")
            .field("info", &self.info)
            .field("origin", &self.origin)
            .field("len", &len)
            .finish()
    }
}

/// Les polices disponibles, que l'appelant énumère (app, système).
#[derive(Debug, Clone, Default)]
pub struct FontCatalog {
    faces: Vec<CatalogFace>,
}

impl FontCatalog {
    /// Catalogue vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute les faces d'un fichier de police (TrueType, OpenType ou
    /// collection) ; rend le nombre de faces lues, 0 si rien ne se lit.
    ///
    /// Les octets sont gardés tels quels, sans copie : un `Vec<u8>`, ou les
    /// octets partagés des polices du rendu.
    pub fn add(&mut self, origin: FontOrigin, data: impl AsRef<[u8]> + Send + Sync + 'static) -> usize {
        let data: Shared = Arc::new(data);
        let bytes = (*data).as_ref();
        let before = self.faces.len();
        let read: Vec<_> = (0..face::count(bytes)).filter_map(|index| face::read(bytes, index)).collect();
        for info in read {
            self.faces.push(CatalogFace { info, origin, data: Arc::clone(&data) });
        }
        self.faces.len() - before
    }

    /// Vrai si une face de la famille `family` (casse ignorée) est disponible.
    pub fn contains(&self, family: &str) -> bool {
        let key = key(family);
        self.faces.iter().any(|f| f.info.aliases.contains(&key))
    }

    /// Faces disponibles de `family` : celles de l'app d'abord, sinon celles
    /// du système que leurs droits laissent embarquer. Rend aussi `true` si
    /// des faces du système ont été écartées pour leurs droits.
    fn lookup(&self, family: &str) -> (Option<(FontOrigin, Vec<Pick<'_>>)>, bool) {
        let key = &key(family);
        let named = || self.faces.iter().filter(move |f| f.info.aliases.contains(key));
        let app: Vec<Pick<'_>> = named().filter(|f| f.origin == FontOrigin::App).map(Pick::from).collect();
        if !app.is_empty() {
            return (Some((FontOrigin::App, best(app))), false);
        }
        let (blocked, system): (Vec<_>, Vec<_>) = named().partition(|f| face::restricted(f.info.fs_type));
        let found =
            (!system.is_empty()).then(|| (FontOrigin::System, best(system.into_iter().map(Pick::from).collect())));
        (found, !blocked.is_empty())
    }
}

/// Une face candidate : sa famille, son style, sa graisse et son fichier.
#[derive(Debug, Clone)]
struct Pick<'a> {
    family: String,
    style: FaceStyle,
    weight: u16,
    metrics: Option<LineMetrics>,
    data: &'a [u8],
}

impl<'a> From<&'a CatalogFace> for Pick<'a> {
    fn from(f: &'a CatalogFace) -> Self {
        let info = &f.info;
        let (style, weight, metrics) = (info.style, info.weight, info.metrics());
        Self { family: info.family.clone(), style, weight, metrics, data: (*f.data).as_ref() }
    }
}

/// Une face par style, la plus proche de 400 (normal) ou 700 (gras), dans l'ordre des styles.
fn best(candidates: Vec<Pick<'_>>) -> Vec<Pick<'_>> {
    [FaceStyle::Regular, FaceStyle::Bold, FaceStyle::Italic, FaceStyle::BoldItalic]
        .into_iter()
        .filter_map(|style| {
            let target: u16 = if matches!(style, FaceStyle::Bold | FaceStyle::BoldItalic) { 700 } else { 400 };
            candidates.iter().filter(|p| p.style == style).min_by_key(|p| p.weight.abs_diff(target)).cloned()
        })
        .collect()
}

/// Clé de comparaison d'un nom de famille.
fn key(family: &str) -> String {
    family.trim().to_lowercase()
}

/// Le substitut de `family` (casse ignorée), s'il en a un.
pub fn substitute(family: &str) -> Option<&'static str> {
    let key = key(family);
    SUBSTITUTES.iter().find(|(source, _)| *source == key).map(|(_, by)| *by)
}

/// Les familles que [`resolve`] cherche au catalogue pour `fonts` et
/// `wanted` : chacune et son substitut, en minuscules. L'appelant n'ajoute
/// au catalogue que les polices du système de ces familles : elles sont
/// nombreuses, leurs octets ne se lisent qu'au besoin.
pub fn sought(fonts: &[SourceFont], wanted: &[&str]) -> BTreeSet<String> {
    let families = fonts.iter().map(|f| f.family.as_str()).chain(wanted.iter().copied());
    families.flat_map(|f| [Some(key(f)), substitute(f).map(key)]).flatten().filter(|k| !k.is_empty()).collect()
}

/// Décide de la police de chaque famille : celles de `fonts` (familles
/// employées ou embarquées par la présentation) et celles de `wanted`
/// (celles du thème, par exemple), sans doublon de casse.
///
/// Les fichiers embarqués sont bornés : [`MAX_EMBEDDED_FILE`] chacun,
/// [`MAX_EMBEDDED_TOTAL`] en tout, dans l'ordre des familles.
pub fn resolve(fonts: &[SourceFont], wanted: &[&str], catalog: &FontCatalog) -> FontPlan {
    resolve_within(fonts, wanted, catalog, Caps { file: MAX_EMBEDDED_FILE, total: MAX_EMBEDDED_TOTAL })
}

/// Bornes des fichiers embarqués, en octets.
#[derive(Debug, Clone, Copy)]
struct Caps {
    /// Par fichier.
    file: usize,
    /// En tout.
    total: usize,
}

/// [`resolve`] aux bornes `caps`.
fn resolve_within(fonts: &[SourceFont], wanted: &[&str], catalog: &FontCatalog, caps: Caps) -> FontPlan {
    let mut families: BTreeMap<String, (&str, Vec<&crate::source::FontFace>)> = BTreeMap::new();
    for f in fonts {
        families.entry(key(&f.family)).or_insert((&f.family, Vec::new())).1.extend(&f.faces);
    }
    for w in wanted {
        families.entry(key(w)).or_insert((w, Vec::new()));
    }
    let mut plan = FontPlan::default();
    let mut embedded = HashSet::new();
    let mut spent = 0usize;
    for (k, (family, faces)) in families {
        if k.is_empty() {
            continue;
        }
        // Les faces à embarquer tiennent-elles : chaque fichier, et les nouveaux avec ceux déjà retenus ?
        let fits = |picks: &[Pick<'_>]| {
            let mut new = HashSet::new();
            let mut size = 0usize;
            for p in picks {
                if p.data.len() > caps.file {
                    return false;
                }
                if !embedded.contains(&p.data.as_ptr().addr()) && new.insert(p.data.as_ptr().addr()) {
                    size = size.saturating_add(p.data.len());
                }
            }
            spent.saturating_add(size) <= caps.total
        };
        let (choice, picks) = choose(family, &faces, catalog, &fits);
        if let (Some(used), Some(m)) = (&choice.used, picks.first().and_then(|p| p.metrics)) {
            plan.metrics.entry(key(used)).or_insert(m);
        }
        let embeds = match choice.decision {
            FontDecision::Embedded
            | FontDecision::Available { origin: FontOrigin::System }
            | FontDecision::Substituted { origin: FontOrigin::System } => picks,
            _ => Vec::new(),
        };
        for p in embeds {
            // Une collection partagée par plusieurs faces ne part qu'une fois.
            if embedded.insert(p.data.as_ptr().addr()) {
                spent = spent.saturating_add(p.data.len());
                insert_file(&mut plan.files, &p);
            }
        }
        plan.choices.push(choice);
    }
    plan
}

/// La décision pour `family` et les faces retenues. Des faces à embarquer
/// (de la présentation ou du système) que `fits` refuse sont passées : la
/// recherche continue, le choix le note (`too_large`).
fn choose<'a>(
    family: &str,
    faces: &[&'a crate::source::FontFace],
    catalog: &'a FontCatalog,
    fits: &dyn Fn(&[Pick<'_>]) -> bool,
) -> (FontChoice, Vec<Pick<'a>>) {
    let mut too_large = false;
    let mut kept = |origin: FontOrigin, picks: &[Pick<'_>]| {
        let ok = origin == FontOrigin::App || fits(picks);
        too_large |= !ok;
        ok
    };
    let readable: Vec<_> = faces.iter().filter_map(|f| face::read(&f.data, 0).map(|i| (i, *f))).collect();
    let mut restricted = readable.iter().any(|(i, f)| face::restricted(f.fs_type.unwrap_or(i.fs_type)));
    let usable: Vec<_> = readable.iter().filter(|(i, f)| !face::restricted(f.fs_type.unwrap_or(i.fs_type))).collect();
    let (found, blocked) = catalog.lookup(family);
    restricted |= blocked;
    // Un sous-ensemble (`saveSubsetFonts`) n'a que les glyphes du texte source : la police complète passe avant.
    let subset = usable.iter().all(|(_, f)| f.subset);
    let mut decided: Option<(FontDecision, Vec<Pick<'a>>)> = None;
    if let Some((info, _)) = usable.first()
        && !(subset && found.is_some())
    {
        let picks: Vec<Pick<'a>> = usable
            .iter()
            .map(|(i, f)| Pick {
                family: info.family.clone(),
                style: i.style,
                weight: i.weight,
                metrics: i.metrics(),
                data: &f.data,
            })
            .collect();
        // Les faces de la présentation partent toujours dans le deck : `System` les borne.
        if kept(FontOrigin::System, &picks) {
            decided = Some((FontDecision::Embedded, picks));
        }
    }
    if decided.is_none()
        && let Some((origin, picks)) = found.filter(|(o, p)| kept(*o, p))
    {
        decided = Some((FontDecision::Available { origin }, picks));
    }
    if decided.is_none()
        && let Some((origin, picks)) =
            substitute(family).and_then(|by| catalog.lookup(by).0).filter(|(o, p)| kept(*o, p))
    {
        decided = Some((FontDecision::Substituted { origin }, picks));
    }
    let (decision, picks) = decided.unwrap_or((FontDecision::Missing, Vec::new()));
    let used = picks.first().map(|p| p.family.clone());
    let choice = FontChoice { family: family.to_owned(), used, decision, restricted, too_large, fallback: None };
    (choice, picks)
}

/// Nom de fichier d'une face : `<famille>-<style>.<ext>`, la famille réduite
/// à `[a-z0-9-]` ; `-2`, `-3`… si le nom est déjà pris par d'autres octets.
fn insert_file(files: &mut BTreeMap<String, Vec<u8>>, p: &Pick<'_>) {
    let style = match p.style {
        FaceStyle::Regular => "regular",
        FaceStyle::Bold => "bold",
        FaceStyle::Italic => "italic",
        FaceStyle::BoldItalic => "bold-italic",
    };
    let stem = format!("{}-{style}", slug(&p.family));
    let ext = face::extension(p.data);
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{stem}.{ext}") } else { format!("{stem}-{n}.{ext}") };
        match files.get(&name) {
            Some(existing) if existing.as_slice() == p.data => return,
            Some(_) => n += 1,
            None => {
                files.insert(name, p.data.to_vec());
                return;
            }
        }
    }
}

/// `family` en minuscules ASCII, chaque suite d'autres caractères devenue un tiret ; `font` si rien ne reste.
fn slug(family: &str) -> String {
    let mut out = String::new();
    for c in family.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= MAX_SLUG {
            break;
        }
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() { "font".to_owned() } else { out.to_owned() }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
