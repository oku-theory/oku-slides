//! Le pack que génère l'import hi-fi se sonde et se rend. Ses fichiers et
//! les schémas que l'import déclare pour ses widgets sont ceux des
//! instantanés de `oku-slides-import` (`src/hifi/snapshots`, `*_pack.snap`
//! et `*_schemas.snap`) : lus ici comme des fichiers, sans dépendre de la
//! crate d'import. La sonde du rendu (celle que la CLI lance par le
//! travailleur de rendu) doit lire exactement les schémas déclarés : le
//! projecteur applique ses ops avec eux, sans rendu.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use oku_slides_core::{Deck, Namespace, PackKind, ParamType, WidgetFile, WidgetSchema, qualify};
use oku_slides_render::{BuiltinPack, Fonts, PackSet, PackWidgets, Renderer, load_registry, probe_registry};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Ligne qui précède chaque fichier dans un instantané de pack.
const FILE_MARK: &str = "// ==== ";

/// Le contenu de l'instantané `<case>_<kind>` du pack généré (après son en-tête).
fn snapshot(case: &str, kind: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../oku-slides-import/src/hifi/snapshots")
        .join(format!("oku_slides_import__hifi__pack__pack_tests__{case}_{kind}.snap"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
    text.splitn(3, "---\n").nth(2).unwrap_or_else(|| panic!("instantané sans en-tête")).to_owned()
}

/// L'espace de noms du pack généré dans ces tests.
fn import_ns() -> Namespace {
    Namespace::parse("import").unwrap_or_else(|| panic!("espace de noms"))
}

/// Le pack de layouts généré de `case`, seul dans son ensemble.
fn pack(case: &str) -> PackSet {
    let mut files = BTreeMap::new();
    let mut current: Option<(WidgetFile, String)> = None;
    for line in snapshot(case, "pack").lines() {
        if let Some(name) = line.strip_prefix(FILE_MARK).and_then(|l| l.strip_suffix(" ====")) {
            files.extend(current.take());
            let file = WidgetFile::from_file_name(name).unwrap_or_else(|| panic!("nom de fichier {name}"));
            current = Some((file, String::new()));
        } else if let Some((_, source)) = current.as_mut() {
            source.push_str(line);
            source.push('\n');
        }
    }
    files.extend(current);
    PackSet::new(vec![
        PackWidgets::new("import", import_ns(), PackKind::Layouts, files).unwrap_or_else(|e| panic!("{e}")),
    ])
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Les schémas que l'import déclare pour le pack de `case`, triés par nom.
fn declared(case: &str) -> Vec<WidgetSchema> {
    let mut schemas: Vec<WidgetSchema> =
        serde_json::from_str(&snapshot(case, "schemas")).unwrap_or_else(|e| panic!("{e}"));
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    schemas
}

/// Les jetons qu'ajoute un thème généré par l'import : couleurs du thème
/// Office, marges de page nulles.
fn theme() -> serde_json::Map<String, Value> {
    let tokens = json!({
        "color": {
            "accent-2": "#ed7d31", "accent-3": "#a5a5a5", "accent-4": "#ffc000", "accent-5": "#5b9bd5",
            "accent-6": "#70ad47", "link": "#0563c1", "link-visited": "#954f72",
        },
        "page": { "margin-x": 0, "margin-top": 0, "margin-bottom": 0 },
    });
    let Value::Object(tokens) = tokens else { unreachable!() };
    tokens
}

/// Une slide `id` du widget `w` : `filled`, chaque texte rempli et un élément
/// dans `extras` ; sinon, sans paramètre.
fn slide(id: &str, w: &WidgetSchema, filled: bool) -> Value {
    let mut params = serde_json::Map::new();
    if filled {
        for p in &w.params {
            let v = match p.ty {
                ParamType::Rich => json!([{ "runs": [{ "t": format!("Texte de {}", p.name) }] }, {
                    "list": "bullet", "runs": [{ "t": "Puce", "bold": true }]
                }]),
                ParamType::Plain => json!("Page {n}"),
                ParamType::Slot { .. } => json!([{
                    "id": format!("{id}-e"), "kind": oku_slides_core::kinds::base::TEXT, "frame": { "x": 100, "y": 100, "w": 400, "h": 100 },
                    "params": { "body": [{ "runs": [{ "t": "En plus" }] }] }
                }]),
                _ => continue,
            };
            params.insert(p.name.clone(), v);
        }
    }
    json!({ "id": id, "kind": qualify(&import_ns(), &w.name), "params": params })
}

#[rstest]
#[case("kitchen")]
#[case("minimal")]
#[case("synthetic")]
fn import_pack_probe_reads_the_declared_schemas(#[case] case: &str) {
    let fonts = Fonts::default();
    let pack = pack(case);
    let probe = probe_registry(&fonts, &pack).unwrap_or_else(|e| panic!("sonde : {e}"));
    let mut probed: Vec<WidgetSchema> = serde_json::from_value(probe["packs"][0]["widgets"].clone()).unwrap();
    probed.sort_by(|a, b| a.name.cmp(&b.name));
    assert_that!(probed).is_equal_to(declared(case));
    // Contrôlés comme tout pack (schémas, slots).
    assert_that!(load_registry(&fonts, &pack)).is_ok();
}

#[rstest]
#[case("kitchen")]
#[case("minimal")]
#[case("synthetic")]
fn import_pack_widgets_render_their_fields_and_extras(#[case] case: &str) {
    let fonts = Fonts::default();
    // Le pack généré requiert `base` : ses éléments en extras en sont.
    let mut packs = pack(case).packs().to_vec();
    packs.push(BuiltinPack::Base.widgets().unwrap_or_else(|e| panic!("{e}")));
    let pack = PackSet::new(packs).unwrap_or_else(|e| panic!("{e}"));
    let registry = load_registry(&fonts, &pack).unwrap_or_else(|e| panic!("sonde : {e}"));
    let mut r = Renderer::new(fonts).unwrap_or_else(|e| panic!("{e}"));
    r.set_widgets(&pack, Arc::new(registry)).unwrap_or_else(|e| panic!("{e}"));
    r.set_theme(Some(theme())).unwrap_or_else(|e| panic!("{e}"));
    let widgets = declared(case);
    let slides: Vec<Value> = widgets
        .iter()
        .enumerate()
        .flat_map(|(i, w)| [slide(&format!("p{i}"), w, true), slide(&format!("v{i}"), w, false)])
        .collect();
    let deck =
        Deck::from_json(&json!({ "version": 8, "slides": slides }).to_string()).unwrap_or_else(|e| panic!("{e}"));
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("rendu : {e}"));
    assert_that!(compiled.page_count()).is_equal_to(slides.len());
    let layout = compiled.layout();
    for (i, w) in widgets.iter().enumerate() {
        for p in w.params.iter().filter(|p| matches!(p.ty, ParamType::Rich | ParamType::Plain)) {
            // Vide sans valeur ni défaut : la boîte reste, pour l'éditeur.
            for (owner, empty) in [(format!("p{i}"), false), (format!("v{i}"), p.default.is_none())] {
                let field = layout.fields.iter().find(|f| f.owner == owner && f.field == p.name);
                asserting(&format!("{} : champ {} de {owner}", w.name, p.name))
                    .that(&field.map(|f| f.empty))
                    .is_equal_to(Some(empty));
            }
        }
        asserting(&format!("{} : élément de extras", w.name))
            .that(&layout.boxes.iter().any(|b| b.id == format!("p{i}-e")))
            .is_true();
    }
    // La zone libre est la page, avec sa grille : l'interface y déplace les éléments.
    let zone = &layout.zones[0];
    assert_that!((zone.x, zone.y, zone.w, zone.h, zone.grid.is_some())).is_equal_to((0.0, 0.0, 1920.0, 1080.0, true));
}
