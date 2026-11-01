//! t-223: a deeply nested plain value in a snapshot overflowed the stack.
//!
//! A flat `LoroValue::Map` nested ~40,000 levels deep in a 1.4 KiB snapshot
//! aborted the process (on first access or on drop). Fixed upstream in Loro 1.17 (loro-dev/loro#1204): decoding and dropping
//! nested values no longer recurse. Our own read stops at `MAX_DEPTH`.

use std::collections::HashMap;

use loro::{ExportMode, LoroDoc, LoroValue};
use oku_slides_core::{Deck, DeckReplica};
use serde_json::Value;
use speculoos::prelude::*;

use crate::common;

/// The depth that overflowed an 8 MiB stack on Loro 1.16.2.
const HOSTILE_DEPTH: usize = 40_000;
/// The read bound of `replica::read::MAX_DEPTH`.
const MAX_DEPTH: usize = 64;
const SMALL: usize = 256 * 1024;
const BIG: usize = 1 << 30;

fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("thread: {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panic or overflow on a {size} byte stack"))
}

/// The snapshot of `roadmap` whose `deck.tokens` is a plain map nested `depth` levels.
///
/// Built on a big stack: forging is not what this test checks.
fn forged(depth: usize) -> Vec<u8> {
    on_stack(BIG, move || {
        let deck = Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("{e}"));
        let r = DeckReplica::new(&deck, common::shared()).unwrap_or_else(|e| panic!("{e}"));
        let doc = LoroDoc::new();
        doc.import(&r.snapshot().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
        let mut value = LoroValue::Null;
        for _ in 0..depth {
            value = LoroValue::from(HashMap::from([("n".to_owned(), value)]));
        }
        doc.get_map("deck").insert("tokens", value).unwrap_or_else(|e| panic!("{e}"));
        doc.commit();
        doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"))
    })
}

fn json_depth(v: &Value) -> usize {
    let mut stack = vec![(v, 1)];
    let mut max = 0;
    while let Some((v, d)) = stack.pop() {
        max = max.max(d);
        match v {
            Value::Object(m) => stack.extend(m.values().map(|c| (c, d + 1))),
            Value::Array(a) => stack.extend(a.iter().map(|c| (c, d + 1))),
            _ => {}
        }
    }
    max
}

#[test]
fn from_snapshot_deep_plain_value_reads_bounded_and_drops_on_a_small_stack() {
    let bytes = forged(HOSTILE_DEPTH);
    let depth = on_stack(SMALL, move || {
        let (r, _) = DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared()).unwrap_or_else(|e| panic!("{e}"));
        let (deck, _issues) = r.deck();
        json_depth(&deck.tokens)
    });
    // Read down to our bound, not dropped as unreadable.
    assert_that!(depth).is_greater_than(MAX_DEPTH / 2);
    assert_that!(depth).is_less_than_or_equal_to(MAX_DEPTH);
}
