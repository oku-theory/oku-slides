use serde_json::json;
use speculoos::prelude::*;

use crate::config_write::write_value;
use crate::storage::Storage;

/// Un dossier neuf et le chemin de son `config.toml`, encore absent.
fn temp(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = crate::test_support::fresh_dir(&format!("oku-slides-config-write-{name}-"));
    let path = dir.path().join("config.toml");
    (dir, path)
}

#[test]
fn write_value_missing_file_creates_nested_table() {
    let (_dir, path) = temp("absent");
    write_value(&Storage::local().unwrap(), &path, "stt.vad.pause", &json!(0.9)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_that!(text.contains("[stt.vad]")).is_true();
    assert_that!(text.contains("pause = 0.9")).is_true();
}

#[test]
fn write_value_keeps_comments_and_other_keys() {
    let (_dir, path) = temp("commentaires");
    std::fs::write(&path, "# à moi\n[history]\n# garder\nlimit = 10\n\n[i18n]\nlocale = \"en\"\n").unwrap();
    write_value(&Storage::local().unwrap(), &path, "i18n.locale", &json!("fr")).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_that!(text.as_str()).is_equal_to("# à moi\n[history]\n# garder\nlimit = 10\n\n[i18n]\nlocale = \"fr\"\n");
}

#[test]
fn write_value_null_removes_key() {
    let (_dir, path) = temp("retrait");
    std::fs::write(&path, "[i18n]\nlocale = \"en\"\n").unwrap();
    write_value(&Storage::local().unwrap(), &path, "i18n.locale", &json!(null)).unwrap();
    assert_that!(std::fs::read_to_string(&path).unwrap().contains("locale")).is_false();
}

#[test]
fn write_value_invalid_toml_returns_error_and_keeps_file() {
    let (_dir, path) = temp("invalide");
    std::fs::write(&path, "[[[").unwrap();
    assert_that!(write_value(&Storage::local().unwrap(), &path, "history.limit", &json!(5))).is_err();
    assert_that!(std::fs::read_to_string(&path).unwrap().as_str()).is_equal_to("[[[");
}
