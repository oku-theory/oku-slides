use speculoos::prelude::*;

use super::*;
use crate::test_support::fresh_dir;
use crate::{ApiHeader, HeaderValue, Model};

fn storage() -> Storage {
    Storage::local().unwrap_or_else(|e| panic!("storage: {e}"))
}

fn names(dir: &Path) -> Vec<String> {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("dir: {e}"));
    let mut names: Vec<String> = entries.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect();
    names.sort();
    names
}

#[test]
fn load_missing_file_gives_savable_defaults() {
    let dir = fresh_dir("saved-config-missing-");
    let loaded = load(&storage(), dir.path());
    assert_that!(loaded.config).is_equal_to(Config::default());
    assert_that!(loaded.savable).is_true();
}

#[test]
fn load_missing_file_keeps_nothing_aside() {
    let dir = fresh_dir("saved-config-missing-aside-");
    let _loaded = load(&storage(), dir.path());
    assert_that!(names(dir.path())).is_empty();
}

#[test]
fn load_valid_file_reads_env_headers() {
    let dir = fresh_dir("saved-config-valid-");
    let json = r#"{"model": "api", "version": 1,
      "api": {"headers": [{"name": "api-key", "value": {"env": "AZURE_KEY"}}]}}"#;
    storage().write(&path(dir.path()), json).unwrap_or_else(|e| panic!("write: {e}"));
    let loaded = load(&storage(), dir.path());
    assert_that!(loaded.config.model).is_equal_to(Model::Api);
    assert_that!(loaded.config.api.headers)
        .is_equal_to(vec![ApiHeader { name: "api-key".into(), value: HeaderValue::Env("AZURE_KEY".into()) }]);
}

#[test]
fn load_invalid_file_with_taken_name_uses_a_suffix() {
    let dir = fresh_dir("saved-config-suffix-");
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    // Every name the next few seconds could give is already taken.
    for s in secs..secs + 5 {
        storage().write(&dir.path().join(format!("config.json.invalid-{s}")), "old").unwrap_or_else(|e| panic!("{e}"));
    }
    storage().write(&path(dir.path()), "{").unwrap_or_else(|e| panic!("write: {e}"));
    let loaded = load(&storage(), dir.path());
    assert_that!(loaded.savable).is_true();
    assert_that!(names(dir.path()).iter().any(|n| n.ends_with("-1"))).is_true();
}

#[test]
fn load_unreadable_file_is_not_savable() {
    let dir = fresh_dir("saved-config-unreadable-");
    // A folder named config.json: present, yet never readable as a file.
    std::fs::create_dir(path(dir.path())).unwrap_or_else(|e| panic!("mkdir: {e}"));
    let loaded = load(&storage(), dir.path());
    assert_that!(loaded.savable).is_false();
}
