//! Aides de test du projecteur : jetons effectifs des thèmes intégrés, texte, items,
//! slides, layouts et decks source, registre de base, application d'une slide ou d'un
//! plan par les ops du cœur, profils du pack intégré `essentiel`, thème Office, fixtures
//! d'import lues et polices de l'app.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{
    Deck, Element, LayoutProfile, Namespace, Op, PackKind, PackPin, PackVersion, ParamValue, Slide, WidgetRegistry,
    WidgetSchema, apply_all, kinds, validate,
};
use serde_json::{Map, Value};
use uuid::Uuid;

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
        font_from_defaults: false,
        style_from_reader: false,
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

/// La sonde des packs intégrés `base` et `essentiel` (`fixtures/widgets/builtin-registry.json`).
fn builtin_probe() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    serde_json::from_str(&text).unwrap()
}

/// Le registre des packs intégrés `base` et `essentiel` (`fixtures/widgets/builtin-registry.json`).
pub(crate) fn registry() -> WidgetRegistry {
    WidgetRegistry::from_probe(builtin_probe()).unwrap()
}

/// Le registre des packs intégrés plus les layouts `schemas` sous `deck-layouts` (un import hi-fi).
pub(crate) fn registry_with_deck_layouts(schemas: &[WidgetSchema]) -> WidgetRegistry {
    let mut probe = builtin_probe();
    let pack =
        serde_json::json!({ "namespace": Namespace::deck_layouts(), "kind": PackKind::Layouts, "widgets": schemas });
    probe["packs"].as_array_mut().unwrap().push(pack);
    WidgetRegistry::from_probe(probe).unwrap()
}

/// L'épinglage de `base` dans les tests : l'id du pack de la fixture, une version quelconque.
pub(crate) fn base_pin() -> PackPin {
    PackPin {
        id: Uuid::parse_str("3b1f6c2e-5a4d-4e8b-9c71-0d2e4f6a8b13").unwrap(),
        version: PackVersion::from_hex(&"0".repeat(64)).unwrap(),
        requires: vec![],
    }
}

/// L'épinglage d'`essentiel` dans les tests : un id et une version quelconques, requérant `base`.
pub(crate) fn essentiel_pin() -> PackPin {
    PackPin {
        id: Uuid::parse_str("d5eb8a57-07b4-47f7-a818-dde5347160cf").unwrap(),
        version: PackVersion::from_hex(&"0".repeat(64)).unwrap(),
        requires: vec![kinds::base::namespace()],
    }
}

/// Les ops qui épinglent `base` puis `essentiel`, comme la session avant un import.
pub(crate) fn builtin_pins() -> Vec<Op> {
    vec![
        Op::AddPack { kind: PackKind::Widgets, namespace: kinds::base::namespace(), pin: base_pin() },
        Op::AddPack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace(), pin: essentiel_pin() },
    ]
}

/// Un élément `kind` d'id `id` dont le paramètre `param` vaut `value`.
pub(crate) fn element_with(id: &str, kind: &str, param: &str, value: Value) -> Element {
    let mut e = Element::new(id, kind);
    e.params.set(param, ParamValue::Value(value));
    e
}

/// Le deck obtenu en ajoutant une slide `essentiel/content` dont les blocs sont `blocks`,
/// par `Op::AddSlide` sur le registre des packs intégrés.
pub(crate) fn apply_blocks(blocks: Vec<Element>) -> Result<Deck, oku_slides_core::TransactionError> {
    let mut slide = Slide::new("s1", kinds::essentiel::CONTENT);
    slide.params.set("blocks", ParamValue::Slot(blocks));
    let mut deck = Deck::empty();
    let mut ops = builtin_pins();
    ops.push(Op::AddSlide { index: None, defaults: false, slide });
    apply_all(&mut deck, ops, &registry())?;
    Ok(deck)
}

/// Une boîte de texte d'un paragraphe `text`.
pub(crate) fn text_item(id: &str, frame: (f64, f64, f64, f64), text: &str) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run(text)])])))
}

/// Les profils du pack intégré `essentiel`, sous son espace de noms, dans l'ordre du manifeste.
pub(crate) fn essentiel_profiles() -> Vec<(Namespace, LayoutProfile)> {
    let manifest: Value =
        serde_json::from_str(include_str!("../../../typst/oku-slides/packs/essentiel/layouts.json")).unwrap();
    let profiles: Vec<LayoutProfile> = serde_json::from_value(manifest["profiles"].clone()).unwrap();
    profiles.into_iter().map(|p| (kinds::essentiel::namespace(), p)).collect()
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
