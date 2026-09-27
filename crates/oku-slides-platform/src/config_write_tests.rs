use serde_json::json;
use speculoos::prelude::*;

use crate::config_write::write_value;
use crate::storage::Storage;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("oku-slides-config-write-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("config.toml")
}

#[test]
fn write_value_missing_file_creates_nested_table() {
    let path = temp("absent");
    std::fs::remove_file(&path).ok();
    write_value(&Storage::local().unwrap(), &path, "stt.vad.pause", &json!(0.9)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_that!(text.contains("[stt.vad]")).is_true();
    assert_that!(text.contains("pause = 0.9")).is_true();
}

#[test]
fn write_value_keeps_comments_and_other_keys() {
    let path = temp("commentaires");
    std::fs::write(&path, "# à moi\n[history]\n# garder\nlimit = 10\n\n[i18n]\nlocale = \"en\"\n").unwrap();
    write_value(&Storage::local().unwrap(), &path, "i18n.locale", &json!("fr")).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_that!(text.as_str()).is_equal_to("# à moi\n[history]\n# garder\nlimit = 10\n\n[i18n]\nlocale = \"fr\"\n");
}

#[test]
fn write_value_null_removes_key() {
    let path = temp("retrait");
    std::fs::write(&path, "[i18n]\nlocale = \"en\"\n").unwrap();
    write_value(&Storage::local().unwrap(), &path, "i18n.locale", &json!(null)).unwrap();
    assert_that!(std::fs::read_to_string(&path).unwrap().contains("locale")).is_false();
}

#[test]
fn write_value_invalid_toml_returns_error_and_keeps_file() {
    let path = temp("invalide");
    std::fs::write(&path, "[[[").unwrap();
    assert_that!(write_value(&Storage::local().unwrap(), &path, "history.limit", &json!(5))).is_err();
    assert_that!(std::fs::read_to_string(&path).unwrap().as_str()).is_equal_to("[[[");
}
