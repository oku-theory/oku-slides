//! `usage.jsonl` in the sources zip: written, read last under its own cap, merged.

use std::io::{Cursor, Read, Write};

use rstest::rstest;
use speculoos::prelude::*;
use zip::CompressionMethod;
use zip::unstable::write::FileOptionsExt;
use zip::write::SimpleFileOptions;

use super::*;
use crate::usage::usage_tests::record;
use crate::{USAGE_FILE, USAGE_MAX_BYTES, UsageLedger};

const MAX: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26, compact_after_secs: 0 };

fn roadmap() -> Deck {
    let mut deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
    deck.theme = None;
    deck
}

fn bundle() -> SourceBundle {
    SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap()
}

fn open(bytes: &[u8], limits: SourceLimits) -> (SourceBundle, Vec<Issue>) {
    SourceBundle::from_zip(bytes, limits, crate::test_registry::shared()).unwrap()
}

/// `zipped` rewritten with `extra` first or last, written with `options` (the others deflated);
/// also the uncompressed size of `zipped`'s own entries.
fn rezip(zipped: &[u8], extra: (&str, &[u8]), first: bool, options: SimpleFileOptions) -> (Vec<u8>, usize) {
    let mut za = zip::ZipArchive::new(Cursor::new(zipped)).unwrap();
    let mut entries = Vec::new();
    for i in 0..za.len() {
        let mut f = za.by_index(i).unwrap();
        let name = f.name().unwrap().into_owned();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        entries.push((name, data));
    }
    let own: usize = entries.iter().map(|(_, d)| d.len()).sum();
    let at = if first { 0 } else { entries.len() };
    entries.insert(at, (extra.0.to_owned(), extra.1.to_vec()));
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        for (i, (name, data)) in entries.iter().enumerate() {
            let opts = if i == at { options } else { SimpleFileOptions::default() };
            zw.start_file(name.as_str(), opts).unwrap();
            zw.write_all(data).unwrap();
        }
        zw.finish().unwrap();
    }
    (buf.into_inner(), own)
}

fn deflated() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

/// 200 valid turns, about 50 KiB of `usage.jsonl`.
fn many() -> UsageLedger {
    (0..200).map(|n| record(n, 1)).collect()
}

fn has_usage_entry(zipped: Vec<u8>) -> bool {
    let za = zip::ZipArchive::new(Cursor::new(zipped)).unwrap();
    za.file_names().any(|n| n.unwrap() == USAGE_FILE)
}

#[test]
fn to_zip_with_usage_then_from_zip_keeps_it() {
    let mut b = bundle();
    let _ = b.usage.insert(record(1, 10));
    let (back, issues) = open(&b.to_zip().unwrap(), MAX);
    assert_that!(issues).is_empty();
    assert_that!(back.usage).is_equal_to(&b.usage);
}

#[test]
fn to_zip_empty_usage_writes_no_entry() {
    assert_that!(has_usage_entry(bundle().to_zip().unwrap())).is_false();
}

#[test]
fn to_zip_with_false_leaves_the_ledger_out() {
    let mut b = bundle();
    let _ = b.usage.insert(record(1, 10));
    assert_that!(has_usage_entry(b.to_zip_with(false).unwrap())).is_false();
}

#[test]
fn to_zip_twice_gives_the_same_usage_bytes() {
    let mut b = bundle();
    let _ = b.usage.insert(record(2, 2));
    let _ = b.usage.insert(record(1, 1));
    let read = |zipped: Vec<u8>| {
        let mut za = zip::ZipArchive::new(Cursor::new(zipped)).unwrap();
        let mut data = Vec::new();
        za.by_name(USAGE_FILE).unwrap().read_to_end(&mut data).unwrap();
        data
    };
    assert_that!(read(b.to_zip().unwrap())).is_equal_to(read(b.to_zip().unwrap()));
}

#[test]
fn from_zip_without_usage_entry_returns_empty_ledger() {
    let (back, _) = open(&bundle().to_zip().unwrap(), MAX);
    assert_that!(back.usage.is_empty()).is_true();
}

#[test]
fn from_zip_bad_usage_line_opens_deck_with_issue() {
    let b = bundle();
    let (bytes, _) = rezip(&b.to_zip().unwrap(), (USAGE_FILE, b"garbage\n"), false, deflated());
    let (back, issues) = open(&bytes, MAX);
    assert_that!(back.deck().slides.len()).is_equal_to(b.deck().slides.len());
    assert_that!(issues).contains(Issue::usage_line_invalid(1));
}

#[rstest]
#[case::usage_first(true)]
#[case::usage_last(false)]
fn from_zip_budget_nearly_full_keeps_deck_and_ledger(#[case] first: bool) {
    let b = bundle();
    let ledger = many();
    let (bytes, own) = rezip(&b.to_zip().unwrap(), (USAGE_FILE, &ledger.to_jsonl()), first, deflated());
    // The deck's own entries leave 1 KiB of budget, far less than the ledger's 50 KiB.
    let tight = SourceLimits { total_bytes: own + 1024, ..MAX };
    let (back, issues) = open(&bytes, tight);
    assert_that!(back.deck().slides.len()).is_equal_to(b.deck().slides.len());
    assert_that!(back.usage).is_equal_to(&ledger);
    assert_that!(issues.iter().any(|i| i.message.id().starts_with("issue-usage"))).is_false();
}

#[test]
fn from_zip_usage_over_its_own_cap_is_dropped_whole() {
    let big = vec![b'\n'; usize::try_from(USAGE_MAX_BYTES).unwrap() + 1];
    let (bytes, _) = rezip(&bundle().to_zip().unwrap(), (USAGE_FILE, &big), false, deflated());
    let (back, issues) = open(&bytes, MAX);
    assert_that!(back.usage.is_empty()).is_true();
    assert_that!(issues).contains(Issue::usage_too_large());
}

#[test]
fn from_zip_usage_with_a_bad_crc_opens_deck_with_issue() {
    let ledger = UsageLedger::from_iter([record(1, 1)]);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let (mut bytes, _) = rezip(&bundle().to_zip().unwrap(), (USAGE_FILE, &ledger.to_jsonl()), false, stored);
    let needle = b"claude-opus-5-5";
    let at = bytes.windows(needle.len()).position(|w| w == needle).unwrap();
    bytes[at] = b'x';
    let (back, issues) = open(&bytes, MAX);
    assert_that!(back.usage.is_empty()).is_true();
    assert_that!(issues).contains(Issue::usage_unreadable());
}

#[test]
fn from_zip_encrypted_usage_first_opens_deck_with_issue() {
    let b = bundle();
    let secret = FileOptionsExt::with_deprecated_encryption(deflated(), b"secret").unwrap();
    let (bytes, _) = rezip(&b.to_zip().unwrap(), (USAGE_FILE, &many().to_jsonl()), true, secret);
    let (back, issues) = open(&bytes, MAX);
    assert_that!(back.deck().slides.len()).is_equal_to(b.deck().slides.len());
    assert_that!(back.usage.is_empty()).is_true();
    assert_that!(issues).contains(Issue::usage_unreadable());
}

#[test]
fn merge_unions_usage_of_both_bundles() {
    let mut a = bundle();
    let (mut b, _) = open(&a.to_zip().unwrap(), MAX);
    let _ = a.usage.insert(record(1, 1));
    let _ = b.usage.insert(record(2, 2));
    let (_, issues) = a.merge(b).unwrap();
    assert_that!(a.usage.len()).is_equal_to(2);
    assert_that!(issues.iter().any(|i| i.message.id().starts_with("issue-usage"))).is_false();
}

#[test]
fn merge_forged_turn_reports_it() {
    let mut a = bundle();
    let (mut b, _) = open(&a.to_zip().unwrap(), MAX);
    let _ = a.usage.insert(record(1, 1));
    let _ = b.usage.insert(record(1, 2));
    let (_, issues) = a.merge(b).unwrap();
    assert_that!(issues).contains(Issue::usage_line_forged(&uuid::Uuid::from_u128(1).to_string()));
}

#[test]
fn bundles_with_other_usage_are_not_equal() {
    let a = bundle();
    let (mut b, _) = open(&a.to_zip().unwrap(), MAX);
    let _ = b.usage.insert(record(1, 1));
    assert_that!(a == b).is_false();
}
