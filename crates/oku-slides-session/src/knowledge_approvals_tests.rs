use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;

use crate::knowledge_approvals::KnowledgeApprovals;
use crate::test_support::{fresh_dir, logged};

const HASH: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn storage() -> Storage {
    Storage::local().unwrap()
}

#[test]
fn knowledge_approvals_load_seeded_file_approves_its_hashes() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, format!("[\"{HASH}\"]")).unwrap();
    let approvals = KnowledgeApprovals::load(Some(path), &storage());
    assert_that!(approvals.is_approved(HASH)).is_true();
    assert_that!(approvals.is_approved("sha256:00")).is_false();
}

#[test]
fn knowledge_approvals_load_missing_file_approves_nothing_silently() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let mut approved = true;
    let log = logged(|| approved = KnowledgeApprovals::load(Some(path), &storage()).is_approved(HASH));
    assert_that!(approved).is_false();
    assert_that!(log.contains("unreadable")).is_false();
}

#[test]
fn knowledge_approvals_load_unreadable_file_approves_nothing_and_logs() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, b"{\"not\": \"an array\"}").unwrap();
    let mut approved = true;
    let log = logged(|| approved = KnowledgeApprovals::load(Some(path), &storage()).is_approved(HASH));
    assert_that!(approved).is_false();
    assert_that!(log.contains("knowledge approvals unreadable")).is_true();
}

#[test]
fn knowledge_approvals_two_loads_of_one_file_share_the_set_read_once() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, format!("[\"{HASH}\"]")).unwrap();
    let first = KnowledgeApprovals::load(Some(path.clone()), &storage());
    std::fs::write(&path, b"[]").unwrap();
    let second = KnowledgeApprovals::load(Some(path), &storage());
    assert_that!((first.is_approved(HASH), second.is_approved(HASH))).is_equal_to((true, true));
}
