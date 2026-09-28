use std::path::{Path, PathBuf};

use oku_slides_core::{Deck, ParamValue};
use speculoos::prelude::*;

use super::*;

/// Un dossier de modèles neuf, supprimé à la fin du test.
struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn templates_dir() -> Dir {
    Dir(crate::test_support::fresh_dir("oku-slides-templates-").keep())
}

fn library(dir: &Path) -> Library {
    Library::new(Some(dir.to_owned()), Storage::local().unwrap())
}

/// Un ensemble de l'utilisateur, `id`, copie du premier intégré.
fn user_set(id: &str) -> TemplateSet {
    let mut set = builtin_sets()[0].clone();
    set.id = id.into();
    set.name = format!("Ensemble {id}");
    set
}

/// Écrit `set` à la main dans `<dir>/<dossier>/pack.json`.
fn write_pack(dir: &Path, folder: &str, set: &TemplateSet) {
    std::fs::create_dir_all(dir.join(folder)).unwrap();
    std::fs::write(dir.join(folder).join("pack.json"), serde_json::to_vec(set).unwrap()).unwrap();
}

/// Un deck d'une slide, `slide`, sans ensemble choisi.
fn deck_of(slide: Slide) -> Deck {
    Deck {
        version: oku_slides_core::DECK_VERSION,
        theme: None,
        params: Default::default(),
        vars: Default::default(),
        tokens: serde_json::Value::Null,
        templates: None,
        slides: vec![slide],
    }
}

/// Le texte d'un paramètre : une chaîne, les runs de ses paragraphes, et
/// le texte des paramètres des éléments d'un slot (pas leurs identifiants).
fn texts(value: &ParamValue, out: &mut Vec<String>) {
    match value {
        ParamValue::Text(s) => out.push(s.clone()),
        other => collect(&serde_json::to_value(other).unwrap(), out),
    }
}

fn collect(json: &serde_json::Value, out: &mut Vec<String>) {
    match json {
        serde_json::Value::Array(a) => a.iter().for_each(|v| collect(v, out)),
        serde_json::Value::Object(o) => {
            for (k, v) in o {
                match (k.as_str(), v) {
                    ("t", serde_json::Value::String(s)) => out.push(s.clone()),
                    ("params", serde_json::Value::Object(params)) => {
                        for p in params.values() {
                            match p {
                                serde_json::Value::String(s) => out.push(s.clone()),
                                p => collect(p, out),
                            }
                        }
                    }
                    (_, v) => collect(v, out),
                }
            }
        }
        _ => {}
    }
}

#[test]
fn builtin_sets_every_template_has_a_unique_id() {
    let sets = builtin_sets();
    assert_that!(sets.len()).is_greater_than_or_equal_to(2);
    for s in sets {
        let mut ids: Vec<&str> = s.templates.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        asserting(&format!("identifiants en double dans {}", s.id)).that(&ids.len()).is_equal_to(s.templates.len());
    }
}

#[test]
fn every_builtin_template_is_valid_v2() {
    let registry = crate::test_support::registry();
    for s in builtin_sets() {
        for t in &s.templates {
            asserting(&format!("{} / {}", s.id, t.id))
                .that(&oku_slides_core::validate(&deck_of(t.slide.clone()), &registry, None))
                .is_empty();
        }
    }
}

#[test]
fn builtin_sets_converted_to_v2_keep_their_text() {
    let sets = builtin_sets();
    let essentiel = sets.iter().find(|s| s.id == "essentiel").unwrap();
    let texte = &essentiel.templates[0].slide;
    // Le premier modèle d'« essentiel » : son titre, son surtitre et ses
    // points, passés de la slide v1 aux paramètres v2.
    assert_that!(texte.params.text("title")).is_equal_to(Some("Titre de la slide"));
    assert_that!(texte.params.text("kicker")).is_equal_to(Some("Surtitre"));
    let mut body = Vec::new();
    texts(texte.params.get("blocks").unwrap(), &mut body);
    assert_that!(body).contains("Premier point".to_owned());
    assert_that!(body).contains("Troisième point".to_owned());
    // La fin garde son texte.
    let fin = essentiel.templates.iter().find(|t| t.slide.kind == "end").unwrap();
    assert_that!(fin.slide.params.text("text")).is_equal_to(Some("Merci"));
    // Aucun modèle n'est vide : chaque ensemble garde du texte dans chacun
    // de ses modèles à contenu (hors couverture et sommaire, liés au deck).
    for s in sets {
        for t in s.templates.iter().filter(|t| !matches!(t.slide.kind.as_str(), "title" | "toc")) {
            let mut all = Vec::new();
            for (_, v) in t.slide.params.iter() {
                texts(v, &mut all);
            }
            asserting(&format!("{} / {} sans texte", s.id, t.id)).that(&all.is_empty()).is_false();
        }
    }
}

#[test]
fn slug_accented_or_blank_name_gives_file_friendly_id() {
    assert_that!(slug("Mes modèles à moi !")).is_equal_to("mes-modeles-a-moi".to_owned());
    assert_that!(slug("  ")).is_equal_to("modeles".to_owned());
}

#[test]
fn library_save_list_get_delete_round_trips_user_set() {
    let dir = templates_dir();
    let lib = library(&dir.0);
    let mut set = builtin_sets()[0].clone();
    set.id = lib.free_id("Mon ensemble");
    set.name = "Mon ensemble".into();
    lib.save(&set).unwrap();
    assert_that!(lib.list().iter().any(|s| s.id == "mon-ensemble" && !s.builtin && !s.default)).is_true();
    assert_that!(lib.list().iter().any(|s| s.id == DEFAULT_SET && s.default)).is_true();
    assert_that!(lib.get("mon-ensemble").unwrap().0.templates.len()).is_equal_to(set.templates.len());
    assert_that!(lib.free_id("Mon ensemble")).is_equal_to("mon-ensemble-2".to_owned());
    lib.delete("mon-ensemble").unwrap();
    assert_that!(lib.get("mon-ensemble")).is_none();
}

#[test]
fn library_lists_builtin_and_user_packs_from_folders() {
    let dir = templates_dir();
    write_pack(&dir.0, "zeta", &user_set("zeta"));
    write_pack(&dir.0, "alpha", &user_set("alpha"));
    let ids: Vec<(String, bool)> = library(&dir.0).list().into_iter().map(|s| (s.id, s.builtin)).collect();
    assert_that!(ids).is_equal_to(vec![
        ("essentiel".to_owned(), true),
        ("grilles".to_owned(), true),
        ("alpha".to_owned(), false),
        ("zeta".to_owned(), false),
    ]);
}

#[test]
fn library_save_writes_pack_json_in_the_pack_folder() {
    let dir = templates_dir();
    let set = user_set("mien");
    let path = library(&dir.0).save(&set).unwrap();
    assert_that!(path).is_equal_to(dir.0.join("mien").join("pack.json"));
    let read: TemplateSet = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_that!(read).is_equal_to(set);
    assert_that!(dir.0.join("mien.json").exists()).is_false();
}

#[test]
fn library_delete_removes_the_pack_folder() {
    let dir = templates_dir();
    let lib = library(&dir.0);
    lib.save(&user_set("mien")).unwrap();
    std::fs::write(dir.0.join("mien").join("widgets.typ"), "#let registry = ()\n").unwrap();
    lib.delete("mien").unwrap();
    assert_that!(dir.0.join("mien").exists()).is_false();
    assert_that!(lib.get("mien")).is_none();
}

#[test]
fn library_delete_with_a_path_like_id_touches_nothing() {
    let dir = templates_dir();
    let lib = library(&dir.0.join("packs"));
    lib.save(&user_set("mien")).unwrap();
    std::fs::write(dir.0.join("garde.txt"), "à garder").unwrap();
    assert_that!(lib.delete("..").map_err(|e| e.kind())).is_equal_to(Err(crate::SessionErrorKind::NoTemplateSet));
    assert_that!(dir.0.join("garde.txt").exists()).is_true();
    assert_that!(dir.0.join("packs").join("mien").join("pack.json").exists()).is_true();
}

#[test]
fn library_widgets_returns_the_pack_widgets_source() {
    let dir = templates_dir();
    let lib = library(&dir.0);
    lib.save(&user_set("avec")).unwrap();
    lib.save(&user_set("sans")).unwrap();
    std::fs::write(dir.0.join("avec").join("widgets.typ"), "#let registry = ()\n").unwrap();
    assert_that!(lib.widgets("avec")).is_equal_to(Some("#let registry = ()\n".to_owned()));
    assert_that!(lib.widgets("sans")).is_none();
    assert_that!(lib.widgets(DEFAULT_SET)).is_none();
    assert_that!(lib.widgets("../avec")).is_none();
}

#[test]
fn library_ignores_a_legacy_json_file() {
    let dir = templates_dir();
    std::fs::create_dir_all(&dir.0).unwrap();
    std::fs::write(dir.0.join("ancien.json"), serde_json::to_vec(&user_set("ancien")).unwrap()).unwrap();
    let lib = library(&dir.0);
    assert_that!(lib.get("ancien")).is_none();
    assert_that!(lib.list().iter().any(|s| s.id == "ancien")).is_false();
}

#[test]
fn library_user_pack_takes_its_id_from_its_folder() {
    let dir = templates_dir();
    write_pack(&dir.0, "dossier", &user_set("autre"));
    let lib = library(&dir.0);
    assert_that!(lib.get("dossier").map(|(s, _)| s.id.clone())).is_equal_to(Some("dossier".to_owned()));
    assert_that!(lib.get("autre")).is_none();
}

#[test]
fn library_save_widgets_writes_widgets_typ_in_the_pack_folder() {
    let dir = templates_dir();
    let lib = library(&dir.0);
    lib.save(&user_set("mien")).unwrap();
    let path = lib.save_widgets("mien", "#let registry = ()\n").unwrap();
    assert_that!(path).is_equal_to(dir.0.join("mien").join("widgets.typ"));
    assert_that!(lib.widgets("mien")).is_equal_to(Some("#let registry = ()\n".to_owned()));
    assert_that!(lib.save_widgets("..", "x").map_err(|e| e.kind()))
        .is_equal_to(Err(crate::SessionErrorKind::NoTemplateSet));
}
