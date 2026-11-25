use std::path::Path;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;

use crate::SessionErrorKind;
use crate::approvals_file::read_set_strict;

use crate::knowledge_approvals::KnowledgeApprovals;
use crate::test_support::{fresh_dir, logged};

const HASH: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn storage() -> Storage {
    Storage::local().unwrap()
}

fn hashes(list: &[&str]) -> Vec<String> {
    list.iter().map(|h| (*h).to_owned()).collect()
}

/// The hashes in the file at `path`, read with `std::fs`: a second `load` shares the process set.
fn on_disk(path: &Path) -> Vec<String> {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn knowledge_approvals_load_seeded_file_approves_its_hashes() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, format!("[\"{HASH}\"]")).unwrap();
    let approvals = KnowledgeApprovals::load(Some(path), storage());
    assert_that!(approvals.is_approved(HASH)).is_true();
    assert_that!(approvals.is_approved("sha256:00")).is_false();
}

#[test]
fn knowledge_approvals_load_missing_file_approves_nothing_silently() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let mut approved = true;
    let log = logged(|| approved = KnowledgeApprovals::load(Some(path), storage()).is_approved(HASH));
    assert_that!(approved).is_false();
    assert_that!(log.contains("unreadable")).is_false();
}

#[test]
fn knowledge_approvals_load_unreadable_file_approves_nothing_and_logs() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, b"{\"not\": \"an array\"}").unwrap();
    let mut approved = true;
    let log = logged(|| approved = KnowledgeApprovals::load(Some(path), storage()).is_approved(HASH));
    assert_that!(approved).is_false();
    assert_that!(log.contains("knowledge approvals unreadable")).is_true();
}

#[test]
fn knowledge_approvals_two_loads_of_one_file_share_the_set_read_once() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, format!("[\"{HASH}\"]")).unwrap();
    let first = KnowledgeApprovals::load(Some(path.clone()), storage());
    std::fs::write(&path, b"[]").unwrap();
    let second = KnowledgeApprovals::load(Some(path), storage());
    assert_that!((first.is_approved(HASH), second.is_approved(HASH))).is_equal_to((true, true));
}

#[test]
fn knowledge_approvals_approve_writes_sorted_array() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    approvals.approve(&hashes(&[HASH_C, HASH, HASH_B])).unwrap();
    assert_that!(on_disk(&path)).is_equal_to(hashes(&[HASH, HASH_B, HASH_C]));
    assert_that!(approvals.is_approved(HASH_B)).is_true();
}

#[test]
fn knowledge_approvals_approve_keeps_hashes_added_by_hand_after_load() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    std::fs::write(&path, format!("[\"{HASH_B}\"]")).unwrap();
    approvals.approve(&hashes(&[HASH])).unwrap();
    assert_that!(on_disk(&path)).is_equal_to(hashes(&[HASH, HASH_B]));
    assert_that!((approvals.is_approved(HASH), approvals.is_approved(HASH_B))).is_equal_to((true, true));
}

#[test]
fn knowledge_approvals_approve_corrupt_file_refuses_and_leaves_it() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, b"{\"not\": \"an array\"}").unwrap();
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    let err = approvals.approve(&hashes(&[HASH])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeApprovalsDamaged);
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(b"{\"not\": \"an array\"}".to_vec());
    assert_that!(approvals.is_approved(HASH)).is_false();
}

#[test]
fn knowledge_approvals_approve_unreadable_file_refuses_and_leaves_memory() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, format!("[\"{HASH}\"]")).unwrap();
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    // A directory where the file was: it exists but cannot be read, even by root.
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let err = approvals.approve(&hashes(&[HASH_B])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeApprovalsDamaged);
    assert_that!(approvals.is_approved(HASH_B)).is_false();
    assert_that!(approvals.count()).is_equal_to(1);
    assert_that!(path.is_dir()).is_true();
}

#[test]
fn knowledge_approvals_damaged_error_names_the_remedy_in_french() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::create_dir(&path).unwrap();
    let approvals = KnowledgeApprovals::load(Some(path), storage());
    let err = approvals.approve(&hashes(&[HASH])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeApprovalsDamaged);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.as_str()).is_equal_to(
        "Le fichier des approbations de connaissances est endommagé : réparez ou supprimez knowledge-approvals.json.",
    );
}

/// An approvals path whose folder is a dangling symbolic link: reading finds no file, writing
/// fails, even as root.
#[cfg(unix)]
fn unwritable_path(dir: &Path) -> std::path::PathBuf {
    std::os::unix::fs::symlink(dir.join("nowhere"), dir.join("folder")).unwrap();
    dir.join("folder").join("knowledge-approvals.json")
}

#[cfg(unix)]
#[test]
fn knowledge_approvals_approve_failed_write_refuses_and_leaves_memory() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = unwritable_path(dir.path());
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    let err = approvals.approve(&hashes(&[HASH])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeApprovalsWrite);
    assert_that!(approvals.is_approved(HASH)).is_false();
    assert_that!(approvals.count()).is_equal_to(0);
    assert_that!(path.exists()).is_false();
}

#[test]
fn knowledge_approvals_approve_input_already_in_file_writes_nothing_and_absorbs_it() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    let bytes = format!("[ \"{HASH}\" ]");
    std::fs::write(&path, &bytes).unwrap();
    approvals.approve(&hashes(&[HASH])).unwrap();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(bytes.into_bytes());
    assert_that!(approvals.is_approved(HASH)).is_true();
}

/// A write goes through a temporary file renamed over the old one: the inode would change.
#[cfg(unix)]
#[test]
fn knowledge_approvals_approve_known_hashes_writes_nothing() {
    use std::os::unix::fs::MetadataExt;
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let bytes = format!("[ \"{HASH}\" ]");
    std::fs::write(&path, &bytes).unwrap();
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    let before = std::fs::metadata(&path).unwrap().ino();
    approvals.approve(&hashes(&[HASH])).unwrap();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(bytes.into_bytes());
    assert_that!(std::fs::metadata(&path).unwrap().ino()).is_equal_to(before);
}

#[test]
fn knowledge_approvals_approve_empty_and_duplicate_input() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
    approvals.approve(&[]).unwrap();
    assert_that!(path.exists()).is_false();
    approvals.approve(&hashes(&[HASH, HASH])).unwrap();
    assert_that!(on_disk(&path)).is_equal_to(hashes(&[HASH]));
    assert_that!(approvals.count()).is_equal_to(1);
}

#[test]
fn knowledge_approvals_approve_sixteen_threads_keep_all() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(16));
    let all: Vec<String> = (0..16).map(|n| format!("sha256:{n:064x}")).collect();
    let threads: Vec<_> = all
        .iter()
        .map(|h| {
            let approvals = KnowledgeApprovals::load(Some(path.clone()), storage());
            let (barrier, h) = (barrier.clone(), h.clone());
            std::thread::spawn(move || {
                barrier.wait();
                approvals.approve(&[h])
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap().unwrap();
    }
    assert_that!(on_disk(&path)).is_equal_to(all);
}

#[test]
fn knowledge_approvals_approve_without_path_in_memory() {
    let approvals = KnowledgeApprovals::load(None, storage());
    approvals.approve(&hashes(&[HASH])).unwrap();
    assert_that!(approvals.is_approved(HASH)).is_true();
}

#[test]
fn knowledge_approvals_count_grows() {
    let approvals = KnowledgeApprovals::load(None, storage());
    assert_that!(approvals.count()).is_equal_to(0);
    approvals.approve(&hashes(&[HASH])).unwrap();
    approvals.approve(&hashes(&[HASH, HASH_B])).unwrap();
    assert_that!(approvals.count()).is_equal_to(2);
}

#[test]
fn read_set_strict_missing_is_empty() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let set = read_set_strict::<String>(&dir.path().join("absent.json"), &storage()).unwrap();
    assert_that!(set.is_empty()).is_true();
}

#[test]
fn read_set_strict_not_an_array_is_an_error() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let path = dir.path().join("knowledge-approvals.json");
    std::fs::write(&path, b"[1, 2]").unwrap();
    assert_that!(read_set_strict::<String>(&path, &storage()).is_err()).is_true();
}

#[cfg(unix)]
#[test]
fn knowledge_approvals_write_error_is_localized_in_french() {
    let dir = fresh_dir("oku-slides-knowledge-approvals-");
    let approvals = KnowledgeApprovals::load(Some(unwritable_path(dir.path())), storage());
    let err = approvals.approve(&hashes(&[HASH])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeApprovalsWrite);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.as_str()).is_equal_to("Vos approbations de connaissances n'ont pas pu être enregistrées.");
}
