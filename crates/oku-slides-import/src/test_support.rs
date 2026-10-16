//! Aides de test du projecteur : jetons effectifs des thèmes intégrés, texte, items,
//! slides, layouts et decks source, registre de base, application d'une slide ou d'un
//! plan par les ops du cœur, modèles des packs intégrés, thème Office, fixtures
//! d'import lues et polices de l'app.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, Element, Op, ParamValue, Slide, Template, WidgetRegistry, apply_all, validate};
use serde_json::{Map, Value};

use crate::fonts::{FontCatalog, FontOrigin};
use crate::retrofit::ColorTokens;
use crate::source::{
    Align, Bullet, Caps, Color, Fill, Frame, Item, ItemKind, Layout, LayoutKind, ParaStyle, Paragraph, Rgba, Role, Run,
    RunStyle, Size, SourceDeck, SourceSlide, SourceTheme, Spacing, Styled, TextBox,
};

/// Jetons par défaut du rendu.
pub(crate) const DEFAULT_TOKENS: &str = include_str!("../../../typst/oku-slides/src/default-tokens.json");
/// Thème intégré Papier (`theme.json`).
pub(crate) const PAPIER: &str = include_str!("../../../typst/oku-slides/themes/papier.json");
/// Thème intégré Pro (`theme.json`).
pub(crate) const PRO: &str = include_str!("../../../typst/oku-slides/themes/pro.json");

/// Les jetons effectifs du thème `theme` (un `theme.json`) : les défauts, puis les siens.
pub(crate) fn tokens(theme: &str) -> Map<String, Value> {
    let Value::Object(defaults) = serde_json::from_str(DEFAULT_TOKENS).unwrap() else { panic!("jetons par défaut") };
    let theme: Value = serde_json::from_str(theme).unwrap();
    effective_tokens(&defaults, theme["tokens"].as_object().unwrap())
}

/// Les couleurs effectives du thème `theme`.
pub(crate) fn colors(theme: &str) -> ColorTokens {
    ColorTokens::from_tokens(&tokens(theme))
}

/// Couleur du texte par défaut d'un thème Office : `tx1`, emplacement `dk1`, noir.
pub(crate) fn tx1() -> Color {
    Styled::mapped(Rgba::new(0, 0, 0), "tx1", "dk1", Vec::new())
}

/// Le thème Office (2013–2022) : ses 12 couleurs, Calibri Light et Calibri ;
/// celui que le lecteur intègre (`ThemeData::office`), sans copie de sa palette.
pub(crate) fn office_theme() -> SourceTheme {
    crate::pptx::theme::ThemeData::office().unwrap().theme
}

/// Style de run neutre : Calibri 18 pt, `tx1`, sans effet.
pub(crate) fn run_style() -> RunStyle {
    RunStyle {
        font: Styled::mapped("Calibri".to_owned(), "+mn-lt", "minor", Vec::new()),
        font_ea: None,
        font_cs: None,
        lang: None,
        size: 18.0,
        bold: false,
        weight: 400,
        italic: false,
        underline: false,
        strike: false,
        color: tx1(),
        highlight: None,
        baseline: 0,
        caps: Caps::None,
        char_spacing: 0.0,
        link: None,
    }
}

/// Run `text` au style neutre.
pub(crate) fn run(text: &str) -> Run {
    Run { text: text.to_owned(), field: None, style: run_style() }
}

/// Run `text` au style `f(neutre)`.
pub(crate) fn run_with(text: &str, f: impl FnOnce(&mut RunStyle)) -> Run {
    let mut r = run(text);
    f(&mut r.style);
    r
}

/// Paragraphe de niveau 0 sans puce.
pub(crate) fn para(runs: Vec<Run>) -> Paragraph {
    Paragraph {
        level: 0,
        style: ParaStyle {
            align: Align::Left,
            indent: 0.0,
            margin_left: 0.0,
            line_spacing: Spacing::Percent(1.0),
            space_before: Spacing::Points(0.0),
            space_after: Spacing::Points(0.0),
            bullet: Bullet::None,
            rtl: false,
        },
        runs,
    }
}

/// Paragraphe à puce `•` de niveau `level`.
pub(crate) fn bullet(level: u8, runs: Vec<Run>) -> Paragraph {
    let mut p = para(runs);
    p.level = level;
    p.style.bullet = Bullet::Char { char: "•".into(), color: None, size_pct: 1.0, font: None };
    p
}

/// Boîte de texte aux paragraphes `paragraphs`.
pub(crate) fn text_box(paragraphs: Vec<Paragraph>) -> TextBox {
    TextBox { paragraphs, ..TextBox::default() }
}

/// Le registre des widgets de base (`fixtures/widgets/base-registry.json`).
pub(crate) fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap()
}

/// Un élément `kind` d'id `id` dont le paramètre `param` vaut `value`.
pub(crate) fn element_with(id: &str, kind: &str, param: &str, value: Value) -> Element {
    let mut e = Element::new(id, kind);
    e.params.set(param, ParamValue::Value(value));
    e
}

/// Le deck obtenu en ajoutant une slide `content` dont les blocs sont `blocks`,
/// par `Op::AddSlide` sur le registre de base.
pub(crate) fn apply_blocks(blocks: Vec<Element>) -> Result<Deck, oku_slides_core::TransactionError> {
    let mut slide = Slide::new("s1", "content");
    slide.params.set("blocks", ParamValue::Slot(blocks));
    let mut deck = Deck::empty();
    apply_all(&mut deck, vec![Op::AddSlide { index: None, defaults: false, slide }], &registry())?;
    Ok(deck)
}

/// Une boîte de texte d'un paragraphe `text`.
pub(crate) fn text_item(id: &str, frame: (f64, f64, f64, f64), text: &str) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run(text)])])))
}

/// Les modèles du pack intégré Essentiel, dans l'ordre du pack.
pub(crate) fn essentiel() -> Vec<Template> {
    templates(include_str!("../../../templates/essentiel/pack.json"))
}

/// Les modèles du pack intégré Grilles, dans l'ordre du pack.
pub(crate) fn grilles() -> Vec<Template> {
    templates(include_str!("../../../templates/grilles/pack.json"))
}

fn templates(pack_json: &str) -> Vec<Template> {
    let pack: Value = serde_json::from_str(pack_json).unwrap();
    serde_json::from_value(pack["templates"].clone()).unwrap()
}

/// `item` avec le rôle de placeholder `role`.
pub(crate) fn with_role(mut item: Item, role: Role) -> Item {
    item.role = Some(role);
    item
}

/// Un layout `id` de type `kind`, sans placeholder.
pub(crate) fn layout(id: &str, kind: &str) -> Layout {
    Layout {
        id: id.to_owned(),
        name: kind.to_owned(),
        kind: LayoutKind::from_ooxml(kind),
        master: "m1".to_owned(),
        background: None,
        show_master_decor: true,
        decor: Vec::new(),
        placeholders: Vec::new(),
    }
}

/// Un item `kind` d'id `id` dans la boîte `(x, y, w, h)` (pt source), sans rôle.
pub(crate) fn item(id: &str, (x, y, w, h): (f64, f64, f64, f64), kind: ItemKind) -> Item {
    Item {
        id: id.to_owned(),
        name: id.to_owned(),
        alt: None,
        frame: Frame::new(x, y, w, h),
        z: 0,
        role: None,
        placeholder: None,
        kind,
    }
}

/// La slide source `number`, de layout `layout`, aux items `items`.
pub(crate) fn source_slide(number: u32, layout: Option<&str>, items: Vec<Item>) -> SourceSlide {
    SourceSlide {
        id: format!("s{number}"),
        number,
        layout: layout.map(str::to_owned),
        background: Fill::None,
        show_layout_decor: true,
        hidden: false,
        items,
        notes: String::new(),
    }
}

/// Un deck source 960 × 540 pt aux slides `slides`, sans thème, layout ni asset.
pub(crate) fn source_deck(slides: Vec<SourceSlide>) -> SourceDeck {
    SourceDeck {
        size: Size { w: 960.0, h: 540.0 },
        themes: Vec::new(),
        masters: Vec::new(),
        layouts: Vec::new(),
        slides,
        assets: BTreeMap::new(),
        fonts: Vec::new(),
        warnings: Vec::new(),
    }
}

/// Le deck obtenu par `ops` sur un deck vide et le registre de base ; panique
/// si une op est refusée ou si le deck lève une `Issue` (assets connus : `assets`).
pub(crate) fn applied(ops: Vec<Op>, assets: &[String]) -> Deck {
    let registry = registry();
    let mut deck = Deck::empty();
    apply_all(&mut deck, ops, &registry).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    let known: HashSet<String> = assets.iter().cloned().collect();
    let issues = validate(&deck, &registry, Some(&known));
    assert!(issues.is_empty(), "issues : {issues:?}");
    deck
}

/// Racine des fixtures d'import (`fixtures/import`).
fn import_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import")
}

/// Les fichiers sous `dir`, chemins relatifs à `root` avec des `/`, hors `.DS_Store`.
fn files_under(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if path.is_dir() {
            files_under(root, &path, out);
        } else {
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push((rel, fs::read(&path).unwrap()));
        }
    }
}

/// La fixture d'import `name`, lue : un `.pptx` (`alto-wealth.pptx`) ou un dossier OOXML zippé à la volée (`kitchen`).
pub(crate) fn fixture(name: &str) -> SourceDeck {
    let path = import_fixtures().join(name);
    let bytes = if path.is_dir() {
        let mut files = Vec::new();
        files_under(&path, &path, &mut files);
        files.sort_by(|a, b| (a.0 != "[Content_Types].xml", &a.0).cmp(&(b.0 != "[Content_Types].xml", &b.0)));
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (rel, data) in files {
            w.start_file(rel, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(&data).unwrap();
        }
        w.finish().unwrap().into_inner()
    } else {
        fs::read(&path).unwrap()
    };
    crate::read_pptx(&bytes, &crate::ReadLimits::default()).unwrap()
}

/// Les polices livrées avec l'app (`assets/fonts/*.ttf`), sans police système.
pub(crate) fn app_fonts() -> FontCatalog {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "ttf"))
        .collect();
    paths.sort();
    let mut catalog = FontCatalog::new();
    for p in paths {
        catalog.add(FontOrigin::App, fs::read(p).unwrap());
    }
    catalog
}
