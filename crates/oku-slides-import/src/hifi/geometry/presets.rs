//! Définitions des formes prédéfinies (ECMA-376, `presetShapeDefinitions.xml`),
//! analysées une seule fois, à la première demande.
//!
//! Gardé de chaque preset : `avLst` (ajustements par défaut), `gdLst`
//! (guides), `rect` (zone de texte), `pathLst` (sous-chemins avec leurs
//! `w`/`h`, `fill`, `stroke`). Ignorés : poignées (`ahLst`) et points de
//! connexion (`cxnLst`). Un preset dont une formule ou une commande est
//! illisible est écarté (le test d'exhaustivité garde qu'il n'y en a aucun).
//!
//! `presetShapeDefinitions.xml` : définitions de formes de l'ECMA-376 (Office
//! Open XML, partie 1, annexe D), copie de LibreOffice core
//! `oox/source/drawingml/customshapes/presetShapeDefinitions.xml` ; jeu de
//! données tiers, exempté de la limite des 500 lignes. Le fichier ne porte
//! aucune mention de licence ; le dépôt LibreOffice core
//! porte la MPL 2.0 (`COPYING.MPL`).

use std::collections::HashMap;
use std::sync::LazyLock;

use roxmltree::{Document, Node};

use crate::formula::{Formula, Operand};
use crate::pptx::xml::{attr_bool, attr_str, child, elements};

/// Copie de LibreOffice (`oox/source/drawingml/customshapes/`) ; la balise
/// racine mal orthographiée (`presetShapeDefinitons`) est d'origine.
const DEFINITIONS: &str = include_str!("presetShapeDefinitions.xml");

static PRESETS: LazyLock<HashMap<String, Preset>> = LazyLock::new(|| parse(DEFINITIONS));

/// Le preset `name`, s'il est défini.
pub(super) fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.get(name)
}

/// Noms de tous les presets définis.
#[cfg(test)]
pub(super) fn names() -> impl Iterator<Item = &'static str> {
    PRESETS.keys().map(String::as_str)
}

/// Un preset : guides dans l'ordre d'évaluation, zone de texte, sous-chemins.
#[derive(Debug)]
pub(super) struct Preset {
    /// Ajustements par défaut (`avLst`), remplaçables par ceux de la forme.
    pub(super) adjust: Vec<(String, Formula)>,
    /// Guides (`gdLst`), chacun peut nommer les précédents.
    pub(super) guides: Vec<(String, Formula)>,
    /// Zone de texte : gauche, haut, droite, bas.
    pub(super) text_rect: Option<[Operand; 4]>,
    pub(super) paths: Vec<PresetPath>,
}

/// Un sous-chemin de `pathLst`.
#[derive(Debug)]
pub(super) struct PresetPath {
    /// Espace propre du chemin (`w`, `h`), sinon celui de la forme.
    pub(super) size: Option<(f64, f64)>,
    /// Rempli (`fill` autre que `none` ; `darken`, `lighten`… comptent comme remplis).
    pub(super) fill: bool,
    /// Tracé (`stroke` absent ou vrai, lu comme au lecteur : `attr_bool`).
    pub(super) stroke: bool,
    pub(super) commands: Vec<Command>,
}

/// Commande d'un sous-chemin, opérandes non évalués.
#[derive(Debug)]
pub(super) enum Command {
    Move([Operand; 2]),
    Line([Operand; 2]),
    Cubic([Operand; 6]),
    Quad([Operand; 4]),
    /// `wR hR stAng swAng`.
    Arc([Operand; 4]),
    Close,
}

fn parse(text: &str) -> HashMap<String, Preset> {
    let Ok(doc) = Document::parse(text) else { return HashMap::new() };
    elements(doc.root_element()).filter_map(|n| Some((n.tag_name().name().to_owned(), preset_of(n)?))).collect()
}

fn preset_of(n: Node<'_, '_>) -> Option<Preset> {
    let guides = |list: &str| -> Option<Vec<(String, Formula)>> {
        child(n, list)
            .into_iter()
            .flat_map(elements)
            .map(|gd| Some((gd.attribute("name")?.to_owned(), Formula::parse(gd.attribute("fmla")?)?)))
            .collect()
    };
    let text_rect = match child(n, "rect") {
        Some(r) => Some(operands(r, ["l", "t", "r", "b"])?),
        None => None,
    };
    let paths = elements(child(n, "pathLst")?).map(path_of).collect::<Option<_>>()?;
    Some(Preset { adjust: guides("avLst")?, guides: guides("gdLst")?, text_rect, paths })
}

fn path_of(p: Node<'_, '_>) -> Option<PresetPath> {
    let dim = |a: &str| p.attribute(a).and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0);
    let size = dim("w").zip(dim("h"));
    let commands = elements(p).map(command_of).collect::<Option<_>>()?;
    Some(PresetPath {
        size,
        fill: attr_str(p, "fill") != Some("none"),
        stroke: attr_bool(p, "stroke").unwrap_or(true),
        commands,
    })
}

fn command_of(c: Node<'_, '_>) -> Option<Command> {
    Some(match c.tag_name().name() {
        "moveTo" => Command::Move(points(c)?),
        "lnTo" => Command::Line(points(c)?),
        "cubicBezTo" => Command::Cubic(points(c)?),
        "quadBezTo" => Command::Quad(points(c)?),
        "arcTo" => Command::Arc(operands(c, ["wR", "hR", "stAng", "swAng"])?),
        "close" => Command::Close,
        _ => return None,
    })
}

/// Les `N / 2` points `pt x y` de `c`, à plat.
fn points<const N: usize>(c: Node<'_, '_>) -> Option<[Operand; N]> {
    let flat: Vec<Operand> = elements(c)
        .filter(|pt| pt.tag_name().name() == "pt")
        .flat_map(|pt| [pt.attribute("x"), pt.attribute("y")])
        .map(|v| v.map(Operand::parse))
        .collect::<Option<_>>()?;
    flat.try_into().ok()
}

fn operands<const N: usize>(n: Node<'_, '_>, names: [&str; N]) -> Option<[Operand; N]> {
    let v: Vec<Operand> = names.iter().map(|a| n.attribute(*a).map(Operand::parse)).collect::<Option<_>>()?;
    v.try_into().ok()
}

#[cfg(test)]
#[path = "presets_tests.rs"]
mod presets_tests;
