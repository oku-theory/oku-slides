use oku_slides_core::WidgetFile;
use speculoos::prelude::*;

use super::FileProbeCache;
use crate::widget_probe::FileProbe;

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap()
}

fn probe(message: &str) -> FileProbe {
    FileProbe::broken(&wf("kpi"), message)
}

#[test]
fn get_returns_the_probe_kept_under_its_key() {
    let mut c = FileProbeCache::default();
    c.insert(&wf("kpi"), [1; 32], probe("un"));
    c.insert(&wf("kpi"), [2; 32], probe("deux"));
    assert_that!(c.get(&wf("kpi"), &[1; 32])).is_equal_to(Some(probe("un")));
    assert_that!(c.get(&wf("kpi"), &[3; 32])).is_none();
    assert_that!(c.get(&wf("quote"), &[1; 32])).is_none();
}

#[test]
fn insert_beyond_per_file_evicts_the_least_recently_served() {
    let mut c = FileProbeCache::default();
    let n = u8::try_from(FileProbeCache::PER_FILE).unwrap();
    for k in 0..n {
        c.insert(&wf("kpi"), [k; 32], probe("p"));
    }
    // La première, servie de nouveau, devient la plus récente.
    assert_that!(c.get(&wf("kpi"), &[0; 32])).is_some();
    c.insert(&wf("kpi"), [n; 32], probe("p"));
    assert_that!(c.len()).is_equal_to(FileProbeCache::PER_FILE);
    assert_that!(c.get(&wf("kpi"), &[0; 32])).is_some();
    assert_that!(c.get(&wf("kpi"), &[1; 32])).is_none();
}

#[test]
fn names_are_bounded_separately_and_a_same_key_is_replaced() {
    let mut c = FileProbeCache::default();
    c.insert(&wf("kpi"), [1; 32], probe("ancienne"));
    c.insert(&wf("kpi"), [1; 32], probe("nouvelle"));
    c.insert(&wf("quote"), [1; 32], probe("q"));
    assert_that!(c.len()).is_equal_to(2);
    assert_that!(c.get(&wf("kpi"), &[1; 32])).is_equal_to(Some(probe("nouvelle")));
}
