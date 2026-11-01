use std::collections::BTreeMap;

use oku_slides_core::{SourceLimits, WidgetFile};
use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;

use super::*;
use crate::resume::ResumeError;

/// A budget of `n` bytes.
fn budget(n: u64) -> ReadBudget {
    let n = usize::try_from(n).unwrap();
    ReadBudget::new(SourceLimits { replica_bytes: n, total_bytes: n, compact_after_secs: 0 })
}

/// A resume over a fresh folder whose `sub/` holds `files`.
fn resume_with(sub: &str, files: &[(&str, &[u8])]) -> (tempfile::TempDir, Resume) {
    let dir = crate::test_support::fresh_dir("oku-slides-resume-read-");
    for (name, bytes) in files {
        std::fs::create_dir_all(dir.path().join(sub)).unwrap();
        std::fs::write(dir.path().join(sub).join(name), bytes).unwrap();
    }
    let resume = Resume::new(Storage::local().unwrap(), dir.path().to_path_buf());
    (dir, resume)
}

#[test]
fn read_files_charges_an_empty_entry_its_least_cost() {
    let (_guard, r) = resume_with("assets", &[("a", b""), ("b", b"")]);
    let mut b = budget(2 * ENTRY_BYTES);
    r.read_files(&r.dir.join("assets"), &mut b).unwrap();
    assert_that!(b.left).is_equal_to(0);
}

#[test]
fn read_files_past_the_entry_budget_is_too_many_entries() {
    let (_guard, r) = resume_with("packs", &[("a", b""), ("b", b""), ("c", b"")]);
    let err = r.read_files(&r.dir.join("packs"), &mut budget(2 * ENTRY_BYTES)).err();
    assert_that!(matches!(err, Some(ResumeError::BudgetExhausted { .. }))).is_true();
}

#[test]
fn read_files_charges_a_large_file_its_length() {
    let big = vec![7u8; 10_000];
    let (_guard, r) = resume_with("fonts", &[("f", &big)]);
    let mut b = budget(20_000);
    r.read_files(&r.dir.join("fonts"), &mut b).unwrap();
    assert_that!(b.left).is_equal_to(10_000);
}

#[test]
fn read_files_charges_a_hidden_entry_without_reading_it() {
    let (_guard, r) = resume_with("assets", &[(".DS_Store", b"x")]);
    let mut b = budget(ENTRY_BYTES + 1);
    let files = r.read_files(&r.dir.join("assets"), &mut b).unwrap();
    assert_that!(files.is_empty()).is_true();
    assert_that!(b.left).is_equal_to(1);
}

#[test]
fn read_files_shares_one_budget_across_folders() {
    let file = vec![1u8; 6_000];
    let (_guard, r) = resume_with("deck", &[("a", &file)]);
    std::fs::create_dir_all(r.dir.join("pack")).unwrap();
    std::fs::write(r.dir.join("pack").join("b"), &file).unwrap();
    let mut b = budget(10_000);
    r.read_files(&r.dir.join("deck"), &mut b).unwrap();
    assert_that!(r.read_files(&r.dir.join("pack"), &mut b).is_err()).is_true();
}

#[test]
fn group_packs_files_each_widget_under_its_hex() {
    let raw: BTreeMap<String, Vec<u8>> = [
        ("aa.json".to_owned(), b"m1".to_vec()),
        ("aa.card.typ".to_owned(), b"w1".to_vec()),
        ("bb.json".to_owned(), b"m2".to_vec()),
        ("cc.note.typ".to_owned(), b"w3".to_vec()),
        ("stray".to_owned(), b"x".to_vec()),
    ]
    .into_iter()
    .collect();
    let packs = group_packs(&raw);
    let card = WidgetFile::from_file_name("card.typ").unwrap();
    assert_that!(packs.keys().copied().collect::<Vec<_>>()).is_equal_to(vec!["aa", "bb", "cc"]);
    assert_that!(packs["aa"]).is_equal_to((Some(&b"m1"[..]), vec![(card, &b"w1"[..])]));
    assert_that!(packs["bb"].1.is_empty()).is_true();
    assert_that!(packs["cc"].0).is_none();
}
