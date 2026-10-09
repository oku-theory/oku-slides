//! t-355 : la CLI valide un deck avec les widgets de son pack, et n'embarque
//! pas un pack extrait à côté du deck dont le `pack.json` est celui d'un autre pack.
//!
//! Avant : `check`, `save` et `render` validaient avec le registre de base
//! (« unknown widget » sur chaque widget du pack) ; `packs/<hex>` à côté du
//! deck n'était vérifié que par son empreinte, pas par l'id de son manifeste.

use std::path::Path;
use std::process::Output;

use oku_slides_core::{PackFiles, PackRef, WidgetFile};
use oku_slides_session::{PackLibrary, PackManifest, Storage};
use speculoos::prelude::*;

use crate::common::{oku_slides, workdir};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Le message de `validate` pour un widget inconnu, en anglais.
const UNKNOWN: &str = "unknown widget";

fn quote_pack(id: uuid::Uuid) -> PackFiles {
    let manifest = PackManifest {
        id,
        name: "Pack".to_owned(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".to_owned(),
        templates: Vec::new(),
        examples: Default::default(),
    };
    let file = WidgetFile::parse("quote").unwrap_or_else(|| panic!("nom de widget"));
    manifest.to_files([(file, QUOTE.to_owned())].into()).unwrap_or_else(|e| panic!("pack.json : {e}"))
}

/// Un deck JSON dans `dir` qui utilise le widget `quote` du pack `r`.
fn quote_deck(dir: &Path, r: &PackRef) -> std::path::PathBuf {
    let deck = dir.join("deck.json");
    let json = serde_json::json!({ "version": 4, "templates": r,
        "slides": [{ "id": "s", "kind": "content", "params": { "title": "T", "blocks": [
            { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "H" } }
        ] } }] });
    std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&deck, serde_json::to_vec(&json).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    deck
}

/// Ajoute le pack `quote` à la bibliothèque de `dir` ; sa référence.
fn library_quote(dir: &Path) -> PackRef {
    let id = uuid::Uuid::new_v4();
    let files = quote_pack(id);
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = PackLibrary::new(Some(dir.join("data").join("templates")), storage);
    library.add_version(id, &files).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    PackRef { id, version: files.version() }
}

fn run(cmd: &mut std::process::Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"))
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn check_of_a_deck_using_its_pack_widgets_finds_no_unknown_widget() {
    let dir = workdir("t355-check");
    let deck = quote_deck(&dir, &library_quote(&dir));
    let checked = run(oku_slides(&dir, "en").arg("check").arg(&deck));
    assert_that!(checked.status.success()).is_true();
    assert_that!(stderr(&checked)).does_not_contain(UNKNOWN);
    assert_that!(String::from_utf8_lossy(&checked.stdout).into_owned()).contains("valid deck: 1 slides");
}

#[test]
fn save_then_render_of_a_deck_using_its_pack_widgets_find_no_unknown_widget() {
    let dir = workdir("t355-save");
    let deck = quote_deck(&dir, &library_quote(&dir));
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").arg("save").arg(&deck).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    assert_that!(stderr(&saved)).does_not_contain(UNKNOWN);
    let rendered = run(oku_slides(&dir, "en").arg("render").arg(&pdf).arg("--out").arg(dir.join("out")));
    assert_that!(rendered.status.success()).is_true();
    assert_that!(stderr(&rendered)).does_not_contain(UNKNOWN);
    let opened = run(oku_slides(&dir, "en").arg("open").arg(&pdf));
    assert_that!(opened.status.success()).is_true();
    assert_that!(stderr(&opened)).does_not_contain(UNKNOWN);
}

/// `packs/<hex>` à côté du deck a la bonne empreinte, mais c'est la version d'un autre pack.
#[test]
fn save_ignores_a_pack_beside_the_deck_whose_manifest_id_is_another_pack() {
    let dir = workdir("t355-id");
    let other = quote_pack(uuid::Uuid::new_v4());
    // Le deck annonce un autre id que celui du manifeste, avec la version de ce dernier.
    let r = PackRef { id: uuid::Uuid::new_v4(), version: other.version() };
    let extract = dir.join("extract");
    let deck = quote_deck(&extract, &r);
    let at = extract.join("packs").join(r.version.hex());
    std::fs::create_dir_all(at.join("widgets")).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(at.join("pack.json"), other.pack_json().unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(at.join("widgets").join("quote.typ"), QUOTE).unwrap_or_else(|e| panic!("{e}"));
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "fr").arg("save").arg(&deck).arg(&pdf));
    assert_that!(saved.status.success()).is_false();
    assert_that!(stderr(&saved)).contains("est introuvable");
    assert_that!(pdf.exists()).is_false();
}

/// Dans `dir` : le pack par défaut courant a le widget `quote` ; un deck sans pack l'utilise.
fn deck_without_pack_and_default_quote(dir: &Path) -> std::path::PathBuf {
    let id = oku_slides_session::DEFAULT_PACK;
    let files = quote_pack(id);
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = PackLibrary::new(Some(dir.join("data").join("templates")), storage);
    library.add_version(id, &files).unwrap_or_else(|e| panic!("bibliothèque : {e}"));
    library.set_current(id, &files.version()).unwrap_or_else(|e| panic!("courante : {e}"));
    let deck = dir.join("deck.json");
    let mut json: serde_json::Value = serde_json::from_slice(
        &std::fs::read(quote_deck(dir, &PackRef { id, version: files.version() })).unwrap_or_else(|e| panic!("{e}")),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    json.as_object_mut().unwrap_or_else(|| panic!("objet")).remove("templates");
    std::fs::write(&deck, serde_json::to_vec(&json).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    deck
}

/// Le pack par défaut est modifiable : sa version courante peut avoir des widgets.
/// Un deck sans pack les reçoit à `save`, validés et rendus avec eux.
#[test]
fn save_of_a_deck_without_pack_validates_with_the_widgets_of_the_current_default_pack() {
    let dir = workdir("t355-defaut");
    let deck = deck_without_pack_and_default_quote(&dir);
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").arg("save").arg(&deck).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    assert_that!(stderr(&saved)).does_not_contain(UNKNOWN);
    let rendered = run(oku_slides(&dir, "en").arg("render").arg(&pdf).arg("--out").arg(dir.join("out")));
    assert_that!(rendered.status.success()).is_true();
    assert_that!(stderr(&rendered)).does_not_contain(UNKNOWN);
}

/// `check` d'un deck sans pack : comme `save`, avec les widgets du pack par défaut courant.
#[test]
fn check_of_a_deck_without_pack_validates_with_the_widgets_of_the_current_default_pack() {
    let dir = workdir("t355-check-defaut");
    let deck = deck_without_pack_and_default_quote(&dir);
    let checked = run(oku_slides(&dir, "en").arg("check").arg(&deck));
    assert_that!(checked.status.success()).is_true();
    assert_that!(stderr(&checked)).does_not_contain(UNKNOWN);
    assert_that!(String::from_utf8_lossy(&checked.stdout).into_owned()).contains("valid deck: 1 slides");
}

/// Un pack introuvable n'arrête pas `check` : il est signalé, ses widgets aussi.
#[test]
fn check_of_a_deck_whose_pack_is_missing_reports_it_and_goes_on() {
    let dir = workdir("t355-check-absent");
    let r = PackRef { id: uuid::Uuid::new_v4(), version: quote_pack(uuid::Uuid::new_v4()).version() };
    let deck = quote_deck(&dir, &r);
    let checked = run(oku_slides(&dir, "en").arg("check").arg(&deck));
    assert_that!(checked.status.success()).is_true();
    assert_that!(stderr(&checked)).contains("not found");
    assert_that!(stderr(&checked)).contains(UNKNOWN);
}
