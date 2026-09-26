use speculoos::prelude::*;

use super::*;

#[test]
fn builtin_sets_every_template_is_valid_with_unique_ids() {
    let sets = builtin_sets();
    assert_that!(sets.len()).is_greater_than_or_equal_to(2);
    for s in sets {
        let mut ids: Vec<&str> = s.templates.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        asserting(&format!("identifiants en double dans {}", s.id)).that(&ids.len()).is_equal_to(s.templates.len());
        for t in &s.templates {
            let deck = slider_core::Deck {
                version: 1,
                theme: None,
                params: Default::default(),
                vars: Default::default(),
                tokens: serde_json::Value::Null,
                templates: None,
                slides: vec![t.slide.clone()],
            };
            asserting(&format!("{} / {}", s.id, t.id)).that(&slider_core::validate(&deck, None)).is_empty();
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
    let dir = std::env::temp_dir().join(format!("oku-slides-templates-{}", std::process::id()));
    let lib = Library::new(Some(dir.clone()), Storage::local().unwrap());
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
    std::fs::remove_dir_all(&dir).ok();
}
