use std::io::Cursor;
use std::sync::Arc;

use oku_slides_core::{Deck, SourceBundle, USAGE_FILE, UsageRecord, WidgetRegistry};
use rstest::rstest;
use speculoos::prelude::*;

use super::{Platform, sources_zip};

/// A platform on a fresh data dir (kept alive) whose `[usage] store_in_deck` is `store`.
fn platform(store: bool) -> (tempfile::TempDir, Platform) {
    let data = tempfile::tempdir().unwrap();
    let mut settings = oku_slides_session::Settings::defaults().unwrap();
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    settings.usage.store_in_deck = store;
    (data, Platform::with_settings(settings).unwrap())
}

/// The roadmap fixture holding one agent turn.
fn bundle_with_usage() -> SourceBundle {
    let deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
    let mut bundle = SourceBundle::new(deck, Arc::new(WidgetRegistry::default())).unwrap();
    let record: UsageRecord = serde_json::from_value(serde_json::json!({
        "turn": "00000000-0000-0000-0000-000000000001", "at": "2026-10-30T14:02:11Z",
        "tab": "deck", "agent": "claude", "model": "m", "tokens": null, "cost": null,
    }))
    .unwrap();
    let _ = bundle.usage.insert(record);
    bundle
}

fn has_usage_entry(zipped: &[u8]) -> bool {
    let za = zip::ZipArchive::new(Cursor::new(zipped)).unwrap();
    za.file_names().any(|n| n.unwrap() == USAGE_FILE)
}

#[rstest]
#[case::on(true, true)]
#[case::off(false, false)]
fn sources_zip_store_in_deck_decides_the_ledger_entry(#[case] store: bool, #[case] entry: bool) {
    let (_data, platform) = platform(store);
    let zipped = sources_zip(&platform, &bundle_with_usage()).unwrap();
    assert_that!(has_usage_entry(&zipped)).is_equal_to(entry);
}
