# Token usage ledger in each deck — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** record each agent turn's tokens, model, preset and cost in its deck's sources zip, and read
them back aggregated through `oku-slides usage`, the `usage` MCP tool and the `show_usage` box.

**Architecture:** `oku-slides-core` owns the record, the add-only `UsageLedger` (one `usage.jsonl`
zip entry, unioned on merge), date ranges and the only aggregation (`aggregate`).
`oku-slides-agent` builds a record when a prompt reply arrives (`usage_board::record_turn`) and hands
it to `TabSession::record_usage`; `oku-slides-session` routes it to the owning deck tab
(`Workspace::usage_owner`), stores it in the deck's real document whatever the mode, persists it in
the resume, and saves it silently when a clean deck closes (the sources zip re-embedded into the
PDF bytes already on disk, no render). `oku-slides-mcp` and `oku-slides-cli` call `aggregate`; both
tables take their headers and cells from session `usage_table.rs` (`usage_headers`, `usage_cells`, `usage_total_cells`);
the box is built in session (`usage_panel`) and filed into the turn's thread with `show_static`, like
`render`. `[usage] store_in_deck` (platform) turns the whole feature off.

**Tech Stack:** Rust (snafu, serde, `time`, `uuid`, `zip`), agent-client-protocol 2.2.0 with
`unstable_end_turn_token_usage` (enabled in `oku-slides-agent`'s `Cargo.toml` only), Fluent (20 locales), A2UI `Badge` + `Markdown`, Playwright e2e.

**Spec:** [`docs/superpowers/specs/2026-10-30-deck-usage-ledger-design.md`](../specs/2026-10-30-deck-usage-ledger-design.md) (t-1296)

## Global Constraints

- `SOURCES_VERSION` stays `13`; the entry is `usage.jsonl` at the zip root, absent when empty,
  skipped by name in the main entry loop (never opened there) and read after it, capped by
  `USAGE_MAX_BYTES` (16 MiB, a `u64`) alone, outside the zip's shared budget, so a deck full of
  assets keeps its ledger.
- A usage record never sets `Document.dirty`; the ledger is never part of `Deck` nor the Loro replica.
- Reading untrusted ledger bytes never fails the deck: bad lines become `Issue`s (at most 20 plus one
  summary), never `?`.
- Record checks (`UsageRecord::is_valid`, `UsageCost::is_valid`): `turn` UUID, `at` UTC, `agent` 1 to 64
  chars and `model` ≤ 200 chars, every char passing `shown_char` (no control, no Unicode format char
  such as bidi marks and overrides, U+061C, U+200B–U+200F, U+2060–U+2069, U+FEFF, no U+2028/U+2029),
  `currency` three ASCII upper-case letters, `amount` finite, ≥ 0 and ≤ 1e9. Lines ≤ 4 KiB, ledger
  ≤ 10 000 turns (past it `Inserted::Full`). No `deny_unknown_fields`: unknown fields are kept in
  `UsageRecord.extra` (`#[serde(flatten)]`) and written back, so an older app keeps a newer app's fields.
- Costs are never summed across currencies; days and months are UTC.
- No `std::fs` in production code (Storage / OpenDAL); no `unwrap`/`expect` outside tests; Snafu
  errors with a backtrace in every variant; `lock().unwrap_or_else(PoisonError::into_inner)`; no
  std mutex guard across an await (session work in `spawn_blocking`).
- Every new pub item and module documented, first sentence under 15 words; a new file ≤ 500 lines, a
  modified one ≤ 1000: new code goes in new files where the host file is at or past 500.
- Every new Fluent key in all 20 locales (`ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi
  zh-Hans zh-Hant`), same key order, translated (never English copied), English first; follow
  `crates/oku-slides-i18n/AGENTS.md` (CJK counters, th classifiers, pt-BR, no literal braces).
- Tests: sibling `x_tests.rs` via `#[cfg(test)] #[path]`, speculoos, `unit_scenario_result` names,
  rstest for tables; localized errors tested on key/variant plus a non-English locale.
- Commits: `type(scope): description (t-N)` in English, citing the task's sub-task and t-1296, with
  `!` when a crate's public API breaks (Tasks 6 and 7); no
  `Co-Authored-By` line; message written to a file in the scratchpad and committed with `-F`;
  `git add` as its own command, index checked right before committing.
- `ARCHITECTURE.md` (≤ 50 lines) of each crate touched updated in the same commit; core, session,
  mcp and agent are at 50 already, so every edit merges into existing lines (zero net lines).
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh
  `tempfile` dir. The first nextest run in this worktree builds cold (> 10 min): run it in the background.

## Spec deviations found while planning and reviewing (folded into the spec in the plan commit)

1. The running agent does not know its preset id: Task 6 keeps it in `Link`'s `Current`, next to the
   running launch, not in `Launch`.
2. A pack or theme tab's link to a deck: a deck-only pack's `TabDocument::deck_tab` first, then
   `Tab.opener` (set only when the user opened the tab, in memory only, so lost on restart); a tab
   the agent opened has none, so its own turns are not counted. The deck agent's turns are always
   counted (they run in the deck tab). A help tab records nothing and lists neither tool.
3. Fluent keys go to 20 locales. CLI numbers are printed plain.
4. The record keeps ACP's `total_tokens` (`tokens.total`).
5. A pack tab's `Mode::Pack` preview is a copy of its opener's deck, so only deck mode (`doc`) and
   history (`stash`) store records; pack tabs route to their opener.
6. The "use `show_usage` for cost questions" guidance lives in the tool's description, not in
   `deck.md` (shared with help tabs, which do not list the tool).
7. Collisions keep the record whose JSON line sorts first (not "the first seen"), so merge is
   commutative. New issues `usage-unreadable` (CRC or I/O) and `usage-lines-dropped`.
8. The box and the CLI table have nine columns (key, turns, input, output, reasoning, cache read,
   cache write, total, cost), from one place (session `usage_table.rs`); an empty cost cell shows `–`.
9. The first cost of a resumed session (`session/load` or `session/resume`) only sets the base and
   records `null`; replayed, unknown-session or invalid costs are ignored. Cancelled turns are
   recorded, failed ones are not. The local agent reports no usage.
10. A quit writes nothing to the deck file (the resume keeps the records); closing a clean deck
    whose file is unchanged saves it silently, re-embedding the sources into the PDF on disk without
    rendering; otherwise the unsaved records are lost with the tab, with a warning in the log.
11. With `--by deck`, a turn found in several decks counts once, under the first deck given; labels
    are the path without `.oku-slides.pdf` (or without the extension of a JSON deck), with every char
    `shown_char` refuses dropped. The CLI logs only the ledger's issues (`issue-usage-*`).
12. The tools' `by` is optional and defaults to `day`, like the CLI's `--by`.
13. The ledger's own read cap is 16 MiB whatever the zip budget has left; an encrypted or otherwise
    unreadable `usage.jsonl` is dropped with `usage-unreadable`, never failing the open.

## Files

| File | Change |
|---|---|
| `Cargo.toml` | `time` features `parsing`, `serde` |
| `crates/oku-slides-core/src/usage.rs` (+ tests) | record (`extra` kept), ledger, caps, `shown_char`, `sanitized`, `Inserted` (with `Full`) (Task 1) |
| `crates/oku-slides-core/src/validate.rs` | five `Issue::usage_*` constructors (Task 1) |
| `crates/oku-slides-core/src/sources.rs`, `sources_zip.rs`, `sources_usage_tests.rs`, `AGENTS.md` | `usage` field, `read_usage`, `to_zip_with`, merge union; add-only rule (Task 2) |
| `crates/oku-slides-core/src/usage_report.rs` (+ tests) | `By`, `Range`, `parse_day`, `UsageRangeError`, `aggregate` (Task 3) |
| `crates/oku-slides-session/src/deck_usage.rs`, `resume_usage.rs`, `workspace_usage.rs` (+ tests), `lib.rs`, `resume.rs`, `deck_file.rs`, `agent/tabs.rs`, `workspace_agent.rs`, `AGENTS.md` | record, route, resume (ledger in `DocSnapshot`) (Task 4) |
| `src-tauri/src/state.rs` | `AppSessions::usage_owner` delegate (Task 4) |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml` | `[usage] store_in_deck` (Task 5) |
| `crates/oku-slides-session/src/merge.rs`, `workspace_deck_packs_close.rs`, `crates/oku-slides-core/AGENTS.md` | silent save on close without render; the `store_in_deck` exception to add-only (Task 5) |
| `docs/aide/en.json`, `docs/aide/fr.json` | the setting in the help (Task 5) |
| `crates/oku-slides-agent/Cargo.toml`, `src/usage_board.rs` (+ tests), `link.rs`, `stack.rs`, `choice.rs`, `turn.rs`, `tab_agent.rs`, `bin/fake_agent.rs`, `tests/usage.rs` | capture (Task 6) |
| `crates/oku-slides-session/src/agent/usage_table.rs`, `usage_panel.rs` (+ tests), `crates/oku-slides-mcp/src/usage_tools.rs`, `tool_kinds.rs`, `tools.rs`, `handler.rs`, `tool_error.rs`, `tests/usage.rs`, `crates/oku-slides-local-agent/src/families.rs`, `ui/src/…/Markdown.tsx`, `ui/src/styles.css` | tools, shared table, box; tables scroll in a `.md-table` wrapper (Task 7) |
| `crates/oku-slides-cli/src/usage_cmd.rs` (+ tests), `main.rs`, `tests/usage.rs` | CLI (Task 8) |
| `crates/oku-slides-i18n/src/locales/*/{core,session,agent,cli}.ftl` | new keys × 20 (Tasks 1, 3, 7, 8) |
| `fixtures/agent/e2e.json`, `ui/e2e/290-usage.cjs` | e2e (Task 9) |
| `README.md` | `usage` command, `[usage]` setting, `OKU_SLIDES_USAGE__STORE_IN_DECK` (Task 10) |

---


### Task 1: Usage record and ledger (core)

**Files:**
- Modify: `Cargo.toml` (workspace `time`, line 63)
- Create: `crates/oku-slides-core/src/usage.rs`, `crates/oku-slides-core/src/usage_tests.rs`
- Modify: `crates/oku-slides-core/src/validate.rs` (`impl Issue`, after `theme_version_missing`), `crates/oku-slides-core/src/lib.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (after `issue-theme-closure`, en :159)
- Docs: `crates/oku-slides-core/ARCHITECTURE.md` (last `Modules` line, `Dependencies` line)

**Interfaces:**
- Produces (all re-exported at the core root): `USAGE_FILE: &str = "usage.jsonl"`, `USAGE_MAX_BYTES: u64 = 16 MiB`
  (`u64`, the type of `Storage::read_limited`'s cap and of zip entry sizes),
  `UsageTab { Deck, Pack, Theme }`, `UsageTokens { total, input, output: u64, thought, cache_read, cache_write: Option<u64> }`,
  `UsageCost { amount: f64, currency: String }` with `is_valid(&self) -> bool` (finite, 0 to 1e9, three upper-case letters),
  `UsageRecord { turn: Uuid, at: OffsetDateTime, tab: UsageTab, agent: String, model: Option<String>, tokens: Option<UsageTokens>, cost: Option<UsageCost>, extra: serde_json::Map<String, serde_json::Value> }`
  (`extra`: `#[serde(flatten)]`, the fields a newer app wrote), built with
  `UsageRecord::new(turn: Uuid, at: OffsetDateTime, tab: UsageTab, agent: impl Into<String>) -> UsageRecord`
  (model, tokens and cost `None`, `extra` empty; callers set the rest with struct update:
  `UsageRecord { model, tokens, cost, ..UsageRecord::new(turn, at, tab, agent) }`, never a full literal),
  with `is_valid(&self) -> bool` and `sanitized(self) -> Option<UsageRecord>`,
  `shown_char(c: char) -> bool` (Task 8 filters printed paths with it),
  `Inserted { Added, Same, Forged, Invalid, Full }`,
  `UsageLedger::{insert(&mut self, UsageRecord) -> Inserted, records(&self) -> impl Iterator<Item = &UsageRecord>, len, is_empty,
  union(&mut self, UsageLedger) -> Vec<Issue>, from_jsonl(&[u8]) -> (UsageLedger, Vec<Issue>), to_jsonl(&self) -> Vec<u8>}`,
  `impl FromIterator<UsageRecord> for UsageLedger`;
  `Issue::{usage_line_invalid(line: usize), usage_line_forged(turn: &str), usage_too_large(), usage_unreadable(), usage_lines_dropped(count: usize)}`.
- Test helper for Tasks 2-3: `crate::usage::usage_tests::record(n: u128, input: u64) -> UsageRecord` (`pub(crate)`).

Design notes the code below implements (from the reviews):
- No `deny_unknown_fields`, and unknown fields are kept: `extra` (`#[serde(flatten)]`) holds them and
  `to_jsonl` writes them back, so a file saved by this app keeps a newer app's fields, and a re-saved
  copy of a line equals the original (no `Forged` tie-break that would favour a stripped line).
- Reading is capped: a line over 4 KiB is invalid unparsed, at most 20 issues plus one
  `usage_lines_dropped(n)` summary; forged turns are reported once each through a `HashSet<Uuid>`.
- `insert` refuses (`Inserted::Full`) a new turn past 10 000 records or past `USAGE_MAX_BYTES` of
  lines in all (the ledger counts its line bytes), so a full ledger always fits the read cap and is
  read back whole. It refuses (`Invalid`) a record whose own JSON line is over 4 KiB: re-serializing
  `extra` can lengthen a line (`1e2` comes back `100.0`), and a stored record must always re-read.
- `insert` stores only valid records (`Inserted::Invalid` otherwise). Two records for one turn
  (`Forged`): the one whose JSON line is smaller (byte order) is kept, whatever the order they come
  in, so `union` is commutative even on forged content.
- `is_valid` is an allow-list on `agent` (1 to 64 chars) and `model` (≤ 200) through the public
  `shown_char`: no control character (Cc), no format character (Cf: bidi marks and overrides, U+061C,
  zero-width U+200B–U+200D, U+2060–U+2064, U+FEFF…), no line or paragraph separator (U+2028, U+2029),
  so a CLI table or a Markdown cell cannot be spoofed. `sanitized` brings an agent-made record into
  that shape at capture (Task 6).
- A cost amount is at most 1e9: 10 000 turns of it sum to 1e13, so no total ever reaches `inf`.

- [ ] **Step 1: Enable `time` parsing and serde**

In `Cargo.toml` line 63:

```toml
time = { version = "0.3", features = ["formatting", "local-offset", "macros", "parsing", "serde"] }
```

- [ ] **Step 2: Write the failing tests** — `crates/oku-slides-core/src/usage_tests.rs`

```rust
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;
use time::macros::datetime;
use uuid::Uuid;

use super::*;

/// A valid record for turn `n`, `input` input tokens, everything else fixed.
pub(crate) fn record(n: u128, input: u64) -> UsageRecord {
    UsageRecord {
        model: Some("claude-opus-5-5".into()),
        tokens: Some(UsageTokens {
            total: input + 10,
            input,
            output: 10,
            thought: None,
            cache_read: Some(5),
            cache_write: None,
        }),
        cost: Some(UsageCost { amount: 0.04, currency: "USD".into() }),
        ..UsageRecord::new(Uuid::from_u128(n), datetime!(2026-10-30 14:02:11 UTC), UsageTab::Deck, "claude")
    }
}

/// [`record`] with an unknown field `pad` of `len` bytes, as a newer app might write.
fn padded(n: u128, len: usize) -> UsageRecord {
    let mut r = record(n, 1);
    r.extra.insert("pad".into(), serde_json::json!("p".repeat(len)));
    r
}

fn line(r: &UsageRecord) -> Vec<u8> {
    serde_json::to_vec(r).unwrap()
}

fn lines(rs: &[UsageRecord]) -> Vec<u8> {
    rs.iter().flat_map(|r| [line(r), b"\n".to_vec()]).flatten().collect()
}

#[test]
fn to_jsonl_then_from_jsonl_returns_same_ledger() {
    let l = UsageLedger::from_iter([record(1, 100), record(2, 200)]);
    let (back, issues) = UsageLedger::from_jsonl(&l.to_jsonl());
    assert_that!(issues).is_empty();
    assert_that!(back).is_equal_to(&l);
}

#[test]
fn to_jsonl_same_ledger_returns_same_bytes() {
    let a = UsageLedger::from_iter([record(1, 1), record(2, 2)]);
    let b = UsageLedger::from_iter([record(2, 2), record(1, 1)]);
    assert_that!(a.to_jsonl()).is_equal_to(b.to_jsonl());
}

#[test]
fn records_are_sorted_by_end_of_turn_then_turn() {
    let mut early = record(9, 1);
    early.at = datetime!(2026-10-29 08:00 UTC);
    let l = UsageLedger::from_iter([record(2, 1), early, record(1, 1)]);
    let turns: Vec<u128> = l.records().map(|r| r.turn.as_u128()).collect();
    assert_that!(turns).is_equal_to(vec![9, 1, 2]);
}

/// One `record(1, 1)` line with a field this app does not know.
fn newer_line() -> Vec<u8> {
    let mut value = serde_json::to_value(record(1, 1)).unwrap();
    value["from_a_newer_app"] = serde_json::json!({"k": [1, 2]});
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn from_jsonl_unknown_field_keeps_the_record() {
    let (back, issues) = UsageLedger::from_jsonl(&newer_line());
    assert_that!(issues).is_empty();
    assert_that!(back.len()).is_equal_to(1);
}

#[test]
fn to_jsonl_writes_unknown_fields_back() {
    let (back, _) = UsageLedger::from_jsonl(&newer_line());
    let text = String::from_utf8(back.to_jsonl()).unwrap();
    assert_that!(text.as_str()).contains(r#""from_a_newer_app":{"k":[1,2]}"#);
}

#[test]
fn union_with_a_resaved_copy_reports_nothing() {
    let (mut original, _) = UsageLedger::from_jsonl(&newer_line());
    let (resaved, _) = UsageLedger::from_jsonl(&original.to_jsonl());
    let issues = original.union(resaved);
    assert_that!(issues).is_empty();
    assert_that!(original.len()).is_equal_to(1);
}

#[test]
fn from_jsonl_invalid_line_drops_it_and_keeps_others() {
    let mut bytes = b"{not json}\n".to_vec();
    bytes.extend(lines(&[record(1, 1)]));
    let (back, issues) = UsageLedger::from_jsonl(&bytes);
    assert_that!(back.len()).is_equal_to(1);
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_invalid(1)]);
}

#[test]
fn from_jsonl_blank_lines_are_skipped_silently() {
    let mut bytes = b"\n  \n".to_vec();
    bytes.extend(lines(&[record(1, 1)]));
    let (back, issues) = UsageLedger::from_jsonl(&bytes);
    assert_that!(back.len()).is_equal_to(1);
    assert_that!(issues).is_empty();
}

#[test]
fn from_jsonl_line_over_4_kib_is_invalid() {
    let mut r = record(1, 1);
    r.model = Some("m".repeat(150));
    let mut value = serde_json::to_value(&r).unwrap();
    value["padding"] = serde_json::json!("p".repeat(5000));
    let (back, issues) = UsageLedger::from_jsonl(&serde_json::to_vec(&value).unwrap());
    assert_that!(back.is_empty()).is_true();
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_invalid(1)]);
}

#[rstest]
#[case::empty_agent(|r: &mut UsageRecord| r.agent = String::new())]
#[case::long_agent(|r: &mut UsageRecord| r.agent = "a".repeat(65))]
#[case::control_in_agent(|r: &mut UsageRecord| r.agent = "cl\u{1b}[31maude".into())]
#[case::bidi_in_model(|r: &mut UsageRecord| r.model = Some("opus\u{202E}5".into()))]
#[case::zero_width_in_agent(|r: &mut UsageRecord| r.agent = "cla\u{200B}ude".into())]
#[case::line_separator_in_model(|r: &mut UsageRecord| r.model = Some("opus\u{2028}5".into()))]
#[case::long_model(|r: &mut UsageRecord| r.model = Some("m".repeat(201)))]
#[case::lower_currency(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: 1.0, currency: "usd".into() }))]
#[case::long_currency(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: 1.0, currency: "USDT".into() }))]
#[case::negative_amount(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: -1.0, currency: "USD".into() }))]
#[case::huge_amount(|r: &mut UsageRecord| r.cost = Some(UsageCost { amount: 1e10, currency: "USD".into() }))]
#[case::not_utc(|r: &mut UsageRecord| r.at = datetime!(2026-10-30 14:02:11 +02:00))]
fn from_jsonl_record_failing_checks_is_dropped(#[case] spoil: fn(&mut UsageRecord)) {
    let mut r = record(1, 1);
    spoil(&mut r);
    let (back, issues) = UsageLedger::from_jsonl(&line(&r));
    assert_that!(back.is_empty()).is_true();
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_invalid(1)]);
}

#[test]
fn from_jsonl_nan_amount_is_dropped() {
    // JSON has no NaN: an out-of-range number is the closest a hostile line gets; it is dropped.
    let text = String::from_utf8(line(&record(1, 1))).unwrap().replace("0.04", "1e400");
    let (back, issues) = UsageLedger::from_jsonl(text.as_bytes());
    assert_that!(back.is_empty()).is_true();
    assert_that!(issues).has_length(1);
}

#[test]
fn insert_invalid_record_is_not_stored() {
    let mut r = record(1, 1);
    r.agent = String::new();
    let mut l = UsageLedger::default();
    assert_that!(l.insert(r)).is_equal_to(Inserted::Invalid);
    assert_that!(l.is_empty()).is_true();
}

#[test]
fn insert_same_record_twice_returns_same() {
    let mut l = UsageLedger::default();
    assert_that!(l.insert(record(1, 1))).is_equal_to(Inserted::Added);
    assert_that!(l.insert(record(1, 1))).is_equal_to(Inserted::Same);
}

#[test]
fn insert_past_the_record_cap_returns_full() {
    let mut l: UsageLedger = (0..MAX_RECORDS as u128).map(|n| record(n, 1)).collect();
    assert_that!(l.insert(record(MAX_RECORDS as u128, 1))).is_equal_to(Inserted::Full);
    assert_that!(l.len()).is_equal_to(MAX_RECORDS);
}

#[test]
fn insert_past_the_byte_cap_returns_full_and_the_ledger_still_fits() {
    let mut l = UsageLedger::default();
    let mut n = 0;
    while l.insert(padded(n, 3600)) == Inserted::Added {
        n += 1;
    }
    assert_that!(n).is_greater_than(3000);
    assert_that!(l.insert(padded(n + 1, 3600))).is_equal_to(Inserted::Full);
    let written = u64::try_from(l.to_jsonl().len()).unwrap();
    assert_that!(written).is_less_than_or_equal_to(USAGE_MAX_BYTES);
}

#[test]
fn insert_record_whose_line_is_over_4_kib_is_invalid() {
    let mut l = UsageLedger::default();
    assert_that!(l.insert(padded(1, 5000))).is_equal_to(Inserted::Invalid);
    assert_that!(l.is_empty()).is_true();
}

#[rstest]
#[case::letter('a', true)]
#[case::accent('é', true)]
#[case::cjk('中', true)]
#[case::space(' ', true)]
#[case::escape('\u{1b}', false)]
#[case::arabic_letter_mark('\u{61C}', false)]
#[case::zero_width_space('\u{200B}', false)]
#[case::zero_width_joiner('\u{200D}', false)]
#[case::rtl_override('\u{202E}', false)]
#[case::word_joiner('\u{2060}', false)]
#[case::isolate('\u{2066}', false)]
#[case::line_separator('\u{2028}', false)]
#[case::paragraph_separator('\u{2029}', false)]
#[case::byte_order_mark('\u{FEFF}', false)]
#[case::tag_letter('\u{E0041}', false)]
fn shown_char_refuses_controls_format_and_separators(#[case] c: char, #[case] shown: bool) {
    assert_that!(shown_char(c)).is_equal_to(shown);
}

#[rstest]
#[case::small_first(1, 3)]
#[case::large_first(3, 1)]
fn insert_forged_turn_keeps_the_smaller_line(#[case] first: u64, #[case] second: u64) {
    let mut l = UsageLedger::from_iter([record(1, first)]);
    assert_that!(l.insert(record(1, second))).is_equal_to(Inserted::Forged);
    let kept = l.records().next().and_then(|r| r.tokens).map(|t| t.input);
    assert_that!(kept).is_equal_to(Some(1));
}

#[test]
fn from_jsonl_forged_turn_is_reported_once() {
    let bytes = lines(&[record(1, 1), record(1, 2), record(1, 3)]);
    let (back, issues) = UsageLedger::from_jsonl(&bytes);
    assert_that!(back.len()).is_equal_to(1);
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_forged(&Uuid::from_u128(1).to_string())]);
}

#[test]
fn from_jsonl_many_bad_lines_reports_twenty_then_a_summary() {
    let bytes = "garbage\n".repeat(25);
    let (_, issues) = UsageLedger::from_jsonl(bytes.as_bytes());
    assert_that!(issues).has_length(21);
    assert_that!(issues[19]).is_equal_to(Issue::usage_line_invalid(20));
    assert_that!(issues[20]).is_equal_to(Issue::usage_lines_dropped(5));
}

#[test]
fn read_lines_over_record_cap_drops_and_counts_the_rest() {
    let bytes = lines(&[record(1, 1), record(2, 1), record(3, 1)]);
    let (back, issues) = UsageLedger::read_lines(&bytes, 2);
    assert_that!(back.len()).is_equal_to(2);
    assert_that!(issues).is_equal_to(vec![Issue::usage_lines_dropped(1)]);
}

#[test]
fn union_same_turn_twice_is_idempotent() {
    let mut a = UsageLedger::from_iter([record(1, 1)]);
    let issues = a.union(a.clone());
    assert_that!(issues).is_empty();
    assert_that!(a.len()).is_equal_to(1);
}

#[rstest]
#[case::distinct_turns(vec![record(1, 1)], vec![record(2, 2)])]
#[case::forged_turn(vec![record(1, 1), record(2, 2)], vec![record(1, 7)])]
#[case::forged_both_ways(vec![record(1, 7)], vec![record(1, 1)])]
fn union_is_commutative(#[case] a: Vec<UsageRecord>, #[case] b: Vec<UsageRecord>) {
    let (a, b) = (UsageLedger::from_iter(a), UsageLedger::from_iter(b));
    let (mut ab, mut ba) = (a.clone(), b.clone());
    let _ = ab.union(b);
    let _ = ba.union(a);
    assert_that!(ab).is_equal_to(ba);
}

#[test]
fn union_forged_turn_is_reported() {
    let mut a = UsageLedger::from_iter([record(1, 1)]);
    let issues = a.union(UsageLedger::from_iter([record(1, 2)]));
    assert_that!(issues).is_equal_to(vec![Issue::usage_line_forged(&Uuid::from_u128(1).to_string())]);
}

#[test]
fn to_jsonl_null_tokens_cost_and_model_are_written_null() {
    let mut r = record(1, 1);
    (r.tokens, r.cost, r.model) = (None, None, None);
    let text = String::from_utf8(UsageLedger::from_iter([r]).to_jsonl()).unwrap();
    assert_that!(text.as_str()).contains(r#""model":null"#);
    assert_that!(text.as_str()).contains(r#""tokens":null"#);
    assert_that!(text.as_str()).contains(r#""cost":null"#);
}

#[test]
fn to_jsonl_unreported_token_kinds_stay_null() {
    let text = String::from_utf8(UsageLedger::from_iter([record(1, 1)]).to_jsonl()).unwrap();
    assert_that!(text.as_str()).contains(r#""thought":null"#);
}

#[test]
fn sanitized_strips_control_and_bidi_characters() {
    let mut r = record(1, 1);
    r.agent = "cl\u{1b}aude\u{202E}".into();
    r.model = Some("opus\u{2066}-5".into());
    let s = r.sanitized().unwrap();
    assert_that!(s.agent.as_str()).is_equal_to("claude");
    assert_that!(s.model.as_deref()).is_equal_to(Some("opus-5"));
    assert_that!(s.is_valid()).is_true();
}

#[test]
fn sanitized_truncates_agent_and_model_to_their_caps() {
    let mut r = record(1, 1);
    r.agent = "é".repeat(70);
    r.model = Some("m".repeat(300));
    let s = r.sanitized().unwrap();
    assert_that!(s.agent.chars().count()).is_equal_to(64);
    assert_that!(s.model.map(|m| m.chars().count())).is_equal_to(Some(200));
}

#[rstest]
#[case::nan(f64::NAN, "USD")]
#[case::negative(-0.5, "USD")]
#[case::lower_currency(0.5, "usd")]
#[case::huge(2e9, "USD")]
fn sanitized_invalid_cost_becomes_none(#[case] amount: f64, #[case] currency: &str) {
    let mut r = record(1, 1);
    r.cost = Some(UsageCost { amount, currency: currency.into() });
    assert_that!(r.sanitized().and_then(|s| s.cost)).is_none();
}

#[test]
fn sanitized_converts_the_end_of_turn_to_utc() {
    let mut r = record(1, 1);
    r.at = datetime!(2026-10-30 16:02:11 +02:00);
    assert_that!(r.sanitized().map(|s| s.at)).is_equal_to(Some(datetime!(2026-10-30 14:02:11 UTC)));
}

#[test]
fn sanitized_empty_model_becomes_none() {
    let mut r = record(1, 1);
    r.model = Some("\u{7}".into());
    assert_that!(r.sanitized().and_then(|s| s.model)).is_none();
}

#[test]
fn sanitized_agent_empty_after_cleaning_returns_none() {
    let mut r = record(1, 1);
    r.agent = "\u{1b}\u{202E}".into();
    assert_that!(r.sanitized()).is_none();
}

#[test]
fn usage_line_invalid_localized_in_french_names_the_line() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(Issue::usage_line_invalid(3).message.id()).is_equal_to("issue-usage-line-invalid");
    assert_that!(Issue::usage_line_invalid(3).localize(&fr)).contains("ligne 3");
}

#[test]
fn usage_lines_dropped_localized_in_french_counts_them() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(Issue::usage_lines_dropped(5).localize(&fr)).contains("5");
    assert_that!(Issue::usage_lines_dropped(5).message.id()).is_equal_to("issue-usage-lines-dropped");
}
```

Declare it at the end of `usage.rs`: `#[cfg(test)] #[path = "usage_tests.rs"] pub(crate) mod usage_tests;`
(`pub(crate)` because Tasks 2 and 3 reuse `record`). `Message::id()` is
`crates/oku-slides-i18n/src/message.rs:71`.

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo nextest run -p oku-slides-core usage` (in the background the first time: cold build).
Expected: compile errors, `UsageLedger` not found.

- [ ] **Step 4: Implement `usage.rs`**

```rust
//! Agent token usage: one record per turn, kept in an add-only ledger.
//!
//! The ledger is the `usage.jsonl` entry of the sources zip; its bytes are untrusted, so reading
//! drops what fails the checks and reports it as an [`Issue`], never failing the deck. Reading is
//! capped (line size, record count, issue count) so a hostile entry costs little.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

use crate::validate::Issue;

/// The ledger's zip entry and resume file name.
pub const USAGE_FILE: &str = "usage.jsonl";
/// The most `usage.jsonl` bytes ever read or written, outside the zip budget.
pub const USAGE_MAX_BYTES: u64 = 16 << 20;
const MAX_LINE: usize = 4 << 10;
/// The most turns a ledger holds; [`UsageLedger::insert`] also keeps its lines under
/// [`USAGE_MAX_BYTES`] in all, so a full ledger is always read back whole.
const MAX_RECORDS: usize = 10_000;
const MAX_ISSUES: usize = 20;
const MAX_AGENT: usize = 64;
const MAX_MODEL: usize = 200;
/// The largest cost amount kept: 10 000 turns of it still sum to a finite total.
const MAX_COST: f64 = 1e9;

/// The kind of tab a turn ran in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageTab {
    /// A deck tab.
    Deck,
    /// A pack tab linked to the deck.
    Pack,
    /// A theme tab linked to the deck.
    Theme,
}

/// A turn's token counts, as the agent reported them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTokens {
    /// All token kinds together, as the agent counted them.
    pub total: u64,
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Reasoning tokens; `None`: not reported.
    pub thought: Option<u64>,
    /// Cache read tokens; `None`: not reported.
    pub cache_read: Option<u64>,
    /// Cache write tokens; `None`: not reported.
    pub cache_write: Option<u64>,
}

/// A turn's cost, in the agent's currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageCost {
    /// Amount, finite, from 0 to 1e9.
    pub amount: f64,
    /// ISO 4217 code, three upper-case letters.
    pub currency: String,
}

impl UsageCost {
    /// Whether the amount is finite, 0 to 1e9, the currency three upper-case letters.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.amount.is_finite()
            && (0.0..=MAX_COST).contains(&self.amount)
            && self.currency.len() == 3
            && self.currency.bytes().all(|b| b.is_ascii_uppercase())
    }
}

/// One agent turn: when, where, which agent and model, what it spent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// The turn's identity, made by the app.
    pub turn: Uuid,
    /// End of the turn, UTC.
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    /// The tab the turn ran in.
    pub tab: UsageTab,
    /// The agent preset id.
    pub agent: String,
    /// The session's model; `None`: the agent offers none.
    pub model: Option<String>,
    /// Token counts; `None`: the reply had no usage.
    pub tokens: Option<UsageTokens>,
    /// The turn's cost; `None`: the agent sent none.
    pub cost: Option<UsageCost>,
    /// Fields a newer app wrote, kept as read so a rewrite does not drop them.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Whether `c` may appear in an agent or model name shown in a terminal or Markdown.
///
/// Refuses controls (Cc), format characters (Cf: bidi marks, overrides and isolates, U+061C,
/// zero-width characters, U+FEFF, tags…) and the line and paragraph separators.
#[must_use]
pub fn shown_char(c: char) -> bool {
    !c.is_control() && !is_format(c) && !matches!(c, '\u{2028}' | '\u{2029}')
}

/// Whether `c` is in Unicode's general category Cf (format), as of Unicode 15.1.
fn is_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

fn shown_text(s: &str, max: usize) -> bool {
    s.chars().count() <= max && s.chars().all(shown_char)
}

/// `s` without the characters [`shown_char`] refuses, cut to `max` characters.
fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| shown_char(*c)).take(max).collect()
}

impl UsageRecord {
    /// A turn with no model, token counts or cost yet, and no extra fields.
    #[must_use]
    pub fn new(turn: Uuid, at: OffsetDateTime, tab: UsageTab, agent: impl Into<String>) -> Self {
        Self { turn, at, tab, agent: agent.into(), model: None, tokens: None, cost: None, extra: Map::new() }
    }

    /// Whether the record passes the ledger's checks.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.at.offset() == UtcOffset::UTC
            && !self.agent.is_empty()
            && shown_text(&self.agent, MAX_AGENT)
            && self.model.as_deref().is_none_or(|m| shown_text(m, MAX_MODEL))
            && self.cost.as_ref().is_none_or(UsageCost::is_valid)
    }

    /// The record brought into the checks' shape; `None` when no agent name is left.
    ///
    /// For records the app makes from agent values: names cleaned and cut, the end of turn put
    /// in UTC, an unusable cost dropped.
    #[must_use]
    pub fn sanitized(self) -> Option<Self> {
        let agent = clean(&self.agent, MAX_AGENT);
        if agent.is_empty() {
            return None;
        }
        let model = self.model.map(|m| clean(&m, MAX_MODEL)).filter(|m| !m.is_empty());
        let cost = self.cost.filter(UsageCost::is_valid);
        Some(Self { at: self.at.to_offset(UtcOffset::UTC), agent, model, cost, ..self })
    }
}

/// What [`UsageLedger::insert`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inserted {
    /// A new turn.
    Added,
    /// The same record was already there.
    Same,
    /// Another record holds this turn: the smaller JSON line of the two is kept.
    Forged,
    /// The record fails [`UsageRecord::is_valid`] or its line is over 4 KiB: not stored.
    Invalid,
    /// The ledger holds its most turns or bytes: not stored.
    Full,
}

/// The deck's usage records by turn, add-only.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageLedger {
    records: BTreeMap<Uuid, UsageRecord>,
    /// Bytes of `to_jsonl`: each record's line plus its newline.
    bytes: u64,
}

/// Issues of a read or a union: at most [`MAX_ISSUES`], then a count; each forged turn once.
#[derive(Default)]
struct Report {
    issues: Vec<Issue>,
    unreported: usize,
    forged: HashSet<Uuid>,
}

impl Report {
    fn push(&mut self, issue: Issue) {
        if self.issues.len() < MAX_ISSUES {
            self.issues.push(issue);
        } else {
            self.unreported += 1;
        }
    }

    fn forged(&mut self, turn: Uuid) {
        if self.forged.insert(turn) {
            self.push(Issue::usage_line_forged(&turn.to_string()));
        }
    }

    fn finish(mut self) -> Vec<Issue> {
        if self.unreported > 0 {
            self.issues.push(Issue::usage_lines_dropped(self.unreported));
        }
        self.issues
    }
}

/// `r` as one JSON line; empty (and logged) if serde ever fails.
fn json_line(r: &UsageRecord) -> Vec<u8> {
    serde_json::to_vec(r).unwrap_or_else(|e| {
        // Plain fields, string keys and JSON values: serde_json does not fail on them.
        tracing::error!(error = %e, turn = %r.turn, "usage record not serializable");
        Vec::new()
    })
}

/// What `line` adds to `usage.jsonl`: its bytes and the newline.
fn line_size(line: &[u8]) -> u64 {
    u64::try_from(line.len()).unwrap_or(u64::MAX).saturating_add(1)
}

impl UsageLedger {
    /// Adds a valid record; a turn already held keeps the smaller of the two lines.
    pub fn insert(&mut self, record: UsageRecord) -> Inserted {
        let line = json_line(&record);
        if !record.is_valid() || line.is_empty() || line.len() > MAX_LINE {
            return Inserted::Invalid;
        }
        let size = line_size(&line);
        let full = self.records.len() >= MAX_RECORDS || self.bytes.saturating_add(size) > USAGE_MAX_BYTES;
        match self.records.entry(record.turn) {
            Entry::Vacant(_) if full => Inserted::Full,
            Entry::Vacant(e) => {
                e.insert(record);
                self.bytes += size;
                Inserted::Added
            }
            Entry::Occupied(e) if *e.get() == record => Inserted::Same,
            Entry::Occupied(mut e) => {
                let held = json_line(e.get());
                if line < held {
                    // The kept line is the smaller one: the byte count only goes down.
                    self.bytes = self.bytes.saturating_sub(line_size(&held)) + size;
                    e.insert(record);
                }
                Inserted::Forged
            }
        }
    }

    /// The records, by end of turn then turn id.
    pub fn records(&self) -> impl Iterator<Item = &UsageRecord> {
        let mut v: Vec<_> = self.records.values().collect();
        v.sort_by_key(|r| (r.at, r.turn));
        v.into_iter()
    }

    /// Number of turns.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether no turn is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Adds `other`'s turns; each turn held with other content is reported once.
    pub fn union(&mut self, other: UsageLedger) -> Vec<Issue> {
        let mut report = Report::default();
        for record in other.records.into_values() {
            let turn = record.turn;
            match self.insert(record) {
                Inserted::Forged => report.forged(turn),
                Inserted::Full => report.unreported += 1,
                Inserted::Added | Inserted::Same | Inserted::Invalid => {}
            }
        }
        report.finish()
    }

    /// Reads `usage.jsonl`; bad, forged or surplus lines are dropped and reported.
    #[must_use]
    pub fn from_jsonl(bytes: &[u8]) -> (Self, Vec<Issue>) {
        Self::read_lines(bytes, MAX_RECORDS)
    }

    /// [`Self::from_jsonl`] keeping at most `max_records` turns.
    fn read_lines(bytes: &[u8], max_records: usize) -> (Self, Vec<Issue>) {
        let (mut ledger, mut report) = (Self::default(), Report::default());
        for (i, line) in bytes.split(|b| *b == b'\n').enumerate() {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let parsed = (line.len() <= MAX_LINE).then(|| serde_json::from_slice::<UsageRecord>(line).ok()).flatten();
            let Some(record) = parsed else {
                report.push(Issue::usage_line_invalid(i + 1));
                continue;
            };
            if ledger.len() >= max_records && !ledger.records.contains_key(&record.turn) {
                report.unreported += 1;
                continue;
            }
            let turn = record.turn;
            match ledger.insert(record) {
                Inserted::Forged => report.forged(turn),
                Inserted::Invalid => report.push(Issue::usage_line_invalid(i + 1)),
                Inserted::Full => report.unreported += 1,
                Inserted::Added | Inserted::Same => {}
            }
        }
        (ledger, report.finish())
    }

    /// Writes `usage.jsonl`, one record per line, in [`Self::records`] order.
    #[must_use]
    pub fn to_jsonl(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for line in self.records().map(json_line).filter(|l| !l.is_empty()) {
            out.extend(line);
            out.push(b'\n');
        }
        out
    }
}

impl FromIterator<UsageRecord> for UsageLedger {
    fn from_iter<I: IntoIterator<Item = UsageRecord>>(iter: I) -> Self {
        let mut l = Self::default();
        for r in iter {
            l.insert(r);
        }
        l
    }
}

#[cfg(test)]
#[path = "usage_tests.rs"]
pub(crate) mod usage_tests;
```

- [ ] **Step 5: Add the `Issue` constructors** in `validate.rs`, after `theme_version_missing` (`Message::arg`
takes `usize` through `number_arg_rounded!`)

```rust
    /// A `usage.jsonl` line dropped: unreadable or failing the record checks.
    #[must_use]
    pub fn usage_line_invalid(line: usize) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-usage-line-invalid").arg("line", line) }
    }

    /// A usage turn held twice with different content: one of them is kept.
    #[must_use]
    pub fn usage_line_forged(turn: &str) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-usage-line-forged").arg("turn", turn) }
    }

    /// The usage ledger exceeds what may still be read: dropped whole.
    #[must_use]
    pub fn usage_too_large() -> Self {
        Self { slide: None, element: None, message: Message::new("issue-usage-too-large") }
    }

    /// The usage ledger entry cannot be read (corrupt zip data): dropped whole.
    #[must_use]
    pub fn usage_unreadable() -> Self {
        Self { slide: None, element: None, message: Message::new("issue-usage-unreadable") }
    }

    /// More usage lines were dropped than reported one by one.
    #[must_use]
    pub fn usage_lines_dropped(count: usize) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-usage-lines-dropped").arg("count", count) }
    }
```

- [ ] **Step 6: Add the Fluent keys** to `core.ftl` of all 20 locales, after `issue-theme-closure`, same
order everywhere (Task 3 adds two more keys right after these). English:

```
issue-usage-line-invalid = usage ledger line { $line } unreadable, discarded
issue-usage-line-forged = usage turn { $turn } recorded twice with different content, one discarded
issue-usage-too-large = usage ledger too large, discarded
issue-usage-unreadable = usage ledger unreadable, discarded
issue-usage-lines-dropped = { $count ->
    [one] { $count } more usage ledger line discarded
   *[other] { $count } more usage ledger lines discarded
}
```

French:

```
issue-usage-line-invalid = ligne { $line } du journal d'usage illisible, écartée
issue-usage-line-forged = tour d'usage { $turn } inscrit deux fois avec un contenu différent, l'un écarté
issue-usage-too-large = journal d'usage trop volumineux, écarté
issue-usage-unreadable = journal d'usage illisible, écarté
issue-usage-lines-dropped = { $count ->
    [one] { $count } autre ligne du journal d'usage écartée
   *[other] { $count } autres lignes du journal d'usage écartées
}
```

Translate the 18 others following `crates/oku-slides-i18n/AGENTS.md` (plural categories per locale,
CJK counters, th classifiers, pt-BR, no literal braces).

- [ ] **Step 7: Export from `lib.rs`**: `pub mod usage;` (after `pub mod tokens;`, alphabetical) and
`pub use usage::{Inserted, USAGE_FILE, USAGE_MAX_BYTES, UsageCost, UsageLedger, UsageRecord, UsageTab, UsageTokens, shown_char};`
(after `pub use token_spec::…`). Add a crate-doc bullet after the `tokens` one:

```rust
//! - [`UsageLedger`]: the deck's agent usage, one [`UsageRecord`] per turn, add-only, checked on read.
```

- [ ] **Step 8: Update `ARCHITECTURE.md`** (zero net lines). Replace

```
  `num.rs`: named float/int casts (import, session). `error/` (Snafu, Fluent): `OpError`, `ReplicaError`…; `reason.rs`: refusal reasons.
```

with

```
  `num.rs`: float/int casts. `error/` (Snafu, Fluent), `reason.rs`: refusals. `usage.rs`: `UsageLedger` (add-only, capped reads).
```

and the `Dependencies` line

```
`oku-slides-i18n`, `oku-slides-chart`, `loro`, `zip`, `sha2` / `hex`, `uuid`, `schemars`.
```

with

```
`oku-slides-i18n`, `oku-slides-chart`, `loro`, `zip`, `sha2` / `hex`, `uuid`, `schemars`, `time` (usage dates).
```

Check: `wc -l crates/oku-slides-core/ARCHITECTURE.md` still prints 50.

- [ ] **Step 9: Run the tests** — `cargo nextest run -p oku-slides-core usage`, `cargo nextest run -p oku-slides-i18n`
and `cargo clippy -p oku-slides-core --all-targets -- -D warnings`. Expected: PASS, no warning.

- [ ] **Step 10: Commit** — message file in the scratchpad, `git add Cargo.toml Cargo.lock crates/oku-slides-core crates/oku-slides-i18n`
as its own command, index checked, then `git commit -F <file>`:
`feat(core): usage record and add-only ledger (t-1297, t-1296)`.

### Task 2: Ledger in the sources zip and merge (core)

**Files:**
- Modify: `crates/oku-slides-core/src/sources.rs` (struct :78-95, `PartialEq` :99-107, `new` :124-132, `from_replica` :140-147, `merge` :388-413, `to_zip` / `from_zip` :444-467)
- Modify: `crates/oku-slides-core/src/sources_zip.rs` (`write_zip` :83, `read_zip` :116-201, new `read_usage`)
- Create: `crates/oku-slides-core/src/sources_usage_tests.rs` (`sources_tests.rs` is at 571 lines)
- Modify: `crates/oku-slides-core/src/lib.rs` (crate doc, `SourceBundle` bullet)
- Docs: `crates/oku-slides-core/ARCHITECTURE.md` (`sources.rs` bullet), `crates/oku-slides-core/AGENTS.md`

**Interfaces:**
- Consumes: Task 1 `UsageLedger`, `USAGE_FILE`, `USAGE_MAX_BYTES`, `Issue::{usage_too_large, usage_unreadable}`, `usage_tests::record`.
- Produces: `pub usage: UsageLedger` on `SourceBundle` (set after `from_replica`, whose signature does not
  change); `SourceBundle::to_zip_with(&self, usage: bool) -> Result<Vec<u8>, SourcesError>` (`false`: no
  `usage.jsonl`); `to_zip()` = `to_zip_with(true)`; `merge` returns the union's issues after the others.

Design notes (from the reviews):
- The main loop skips `usage.jsonl` by its name (`name_for_index`) before `by_index` opens it:
  `by_index` fails on an encrypted entry (`PASSWORD_REQUIRED`) or an unsupported compression
  method, and the loop's `?` would then refuse the whole deck. Skipped, it neither fails the loop
  nor spends the shared budget, wherever it sits in the zip.
- It is read after the loop with `by_name`, under its own cap `USAGE_MAX_BYTES` only, not from
  what `SourceLimits.total_bytes` leaves: a deck whose assets nearly fill the budget keeps its
  ledger, instead of reading it empty and writing it out empty at the next save. A read therefore
  decompresses at most `total_bytes` plus 16 MiB.
- Every failure (opening, encryption, compression method, CRC, I/O, size) becomes an `Issue`
  (`usage-unreadable` or `usage-too-large`): no `?`, the deck always opens.

- [ ] **Step 1: Write the failing tests** — `crates/oku-slides-core/src/sources_usage_tests.rs`

```rust
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
```

Declare it in `sources.rs` next to `sources_tests`:
`#[cfg(test)] #[path = "sources_usage_tests.rs"] mod sources_usage_tests;`.
The needle occurs once: the roadmap fixture holds no `claude-opus` (`grep -c claude-opus fixtures/roadmap.json` → 0)
and every other entry is deflated. The encrypted entry is written with ZipCrypto through
`zip::unstable::write::FileOptionsExt` (zip 9.0.4, not feature-gated); it is the only use of zip's
unstable API, in this test only. Placed first, it is the entry the loop meets before any other: the
test fails on the current code, whose `by_index(i)?` refuses the deck with `PASSWORD_REQUIRED`.

- [ ] **Step 2: Run to fail** — `cargo nextest run -p oku-slides-core sources_usage`. Expected: no field `usage`, no `to_zip_with`.

- [ ] **Step 3: Implement in `sources.rs`**
  - `use crate::usage::UsageLedger;`
  - Field, after `themes`:

```rust
    /// Agent usage by turn, add-only (`usage.jsonl`); never part of the deck or the replica.
    pub usage: UsageLedger,
```

  - `PartialEq`: add `&& self.usage == other.usage`; its doc says "the same assets, fonts, packs, themes and usage".
  - `new`: `let (assets, fonts, packs, themes, usage) = Default::default();` and `Ok(Self { replica, deck, assets, fonts, packs, themes, usage })`.
  - `from_replica` (signature unchanged): `let bundle = Self { replica, deck, assets, fonts, packs, themes, usage: UsageLedger::default() };` and add to its doc: "The usage ledger starts empty; `read_zip` sets it."
  - `merge`, after the themes loop and before `self.replica.deck()`:

```rust
        let usage_issues = self.usage.union(other.usage);
        let (deck, issues) = self.replica.deck();
        self.deck = deck;
        let mut issues = self.with_validation(issues);
        issues.extend(usage_issues);
        Ok((report, issues))
```

    and its doc: "Merges `other`: its history, then its assets, fonts, packs, themes and usage (union by turn)."
  - `to_zip` / `to_zip_with`:

```rust
    /// The sources zip, the usage ledger included.
    ///
    /// # Errors
    ///
    /// Tree too deep to be reopened, or zip write refused.
    pub fn to_zip(&self) -> Result<Vec<u8>, SourcesError> {
        self.to_zip_with(true)
    }

    /// The sources zip; `usage: false` leaves `usage.jsonl` out (usage not stored in decks).
    ///
    /// # Errors
    ///
    /// Tree too deep to be reopened, or zip write refused.
    pub fn to_zip_with(&self, usage: bool) -> Result<Vec<u8>, SourcesError> {
        Ok(self.write_zip(usage)?)
    }
```

  - `from_zip` doc, add: "`usage.jsonl` is read last, under its own cap [`crate::USAGE_MAX_BYTES`]
    outside `limits.total_bytes`; a ledger that cannot be read is reported, never an error."

- [ ] **Step 4: Implement in `sources_zip.rs`**
  - Imports: `use std::io::{Cursor, Read, Seek, Write};` and `use crate::usage::{USAGE_FILE, USAGE_MAX_BYTES, UsageLedger};`.
  - `write_zip(&self, usage: bool)`, after `write_themes(...)?;`:

```rust
            if usage && !self.usage.is_empty() {
                zw.start_file(USAGE_FILE, deflate).context(ZipSnafu)?;
                zw.write_all(&self.usage.to_jsonl()).context(IoSnafu)?;
            }
```

  - At the top of `read_zip`'s loop, before `let f = za.by_index(i).context(ZipSnafu)?;`:

```rust
            // `usage.jsonl` is read after the loop (`read_usage`): skipped by name before
            // `by_index` opens it, since an entry it cannot open (encrypted) must not fail the deck.
            if za.name_for_index(i).and_then(Result::ok).as_deref() == Some(USAGE_FILE) {
                continue;
            }
```

    `name_for_index` returns `Option<ZipResult<Cow<'_, str>>>`; the borrow ends with the condition,
    before `by_index` takes `za` mutably. A name that cannot be decoded is not `usage.jsonl`, and
    `by_index` then reports it as it does today.
  - At the end of `read_zip`, replacing the last four lines:

```rust
        let (mut bundle, mut issues) = Self::from_replica(replica, read, assets, fonts, packs, themes);
        report_discarded(&mut issues, &discarded_packs);
        report_discarded_themes(&mut issues, &discarded_themes);
        let (usage, usage_issues) = read_usage(&mut za);
        bundle.usage = usage;
        issues.extend(usage_issues);
        Ok((bundle, issues))
```

  - New function, after `read_entry`:

```rust
/// Reads `usage.jsonl`, last, under [`USAGE_MAX_BYTES`] only; never fails the deck.
///
/// Its cap is its own, outside `SourceLimits.total_bytes`: a deck whose assets fill the budget
/// keeps its ledger, so its next save does not write it out empty.
fn read_usage<R: Read + Seek>(za: &mut zip::ZipArchive<R>) -> (UsageLedger, Vec<Issue>) {
    let dropped = |issue: Issue| (UsageLedger::default(), vec![issue]);
    if za.index_for_name(USAGE_FILE).is_none() {
        return (UsageLedger::default(), Vec::new());
    }
    // An encrypted entry or an unsupported compression method is refused here.
    let f = match za.by_name(USAGE_FILE) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(error = %e, "usage.jsonl unreadable");
            return dropped(Issue::usage_unreadable());
        }
    };
    if f.size() > USAGE_MAX_BYTES {
        return dropped(Issue::usage_too_large());
    }
    let cap = usize::try_from(USAGE_MAX_BYTES).unwrap_or(usize::MAX);
    let announced = usize::try_from(f.size()).unwrap_or(cap);
    let mut data = Vec::with_capacity(announced.min(MAX_INITIAL_CAPACITY));
    match f.take(USAGE_MAX_BYTES + 1).read_to_end(&mut data) {
        // A size header that lied: more bytes than announced.
        Ok(_) if data.len() > cap => dropped(Issue::usage_too_large()),
        Ok(_) => UsageLedger::from_jsonl(&data),
        // A CRC mismatch surfaces here, at the end of the entry.
        Err(e) => {
            tracing::warn!(error = %e, "usage.jsonl unreadable");
            dropped(Issue::usage_unreadable())
        }
    }
}
```

  `read_zip` keeps `let mut budget` (the loop spends it); `read_usage` does not take it.

- [ ] **Step 5: Run** — `cargo nextest run -p oku-slides-core` and `cargo check -p oku-slides-session -p oku-slides-cli`
(both call `to_zip()`, unchanged). Expected: PASS.

- [ ] **Step 6: Docs**
  - `lib.rs` crate doc: `//!   [`SourceBundle::merge`] adds the union of the assets and fonts to it.` becomes
    `//!   [`SourceBundle::merge`] adds the union of the assets, fonts and usage ledger to it.`
  - `ARCHITECTURE.md` (zero net lines): replace the four `sources.rs` lines

```
- `sources.rs`: `SourceBundle` (`SOURCES_VERSION`); its zip in `sources_zip.rs`: Loro snapshot, assets, fonts,
  `<kind>/<hex>/{<kind>.json, <kind>/<name>.typ}` per `PackKind` (`sources_packs.rs`), `themes/<hex>/theme.json`
  (`sources_themes.rs`, pins checked against manifests and closure), all add-only, verified on read (a forged version is
  reported once, as is a namespace pinned under both kinds) and unioned on merge; reads bounded by `SourceLimits`.
```

    with

```
- `sources.rs`: `SourceBundle` (`SOURCES_VERSION`); its zip in `sources_zip.rs`: Loro snapshot, assets, fonts, `<kind>/<hex>/
  {<kind>.json, <kind>/<name>.typ}` per `PackKind` (`sources_packs.rs`), `themes/<hex>/theme.json` (`sources_themes.rs`, pins
  checked against manifests and closure), `usage.jsonl` (read last, own cap, never fails), all add-only, verified on read
  (a forged version reported once, as a namespace under both kinds), unioned on merge; reads bounded by `SourceLimits`.
```

  - `crates/oku-slides-core/AGENTS.md`, Invariants: change
    `- Keep bundle assets, fonts and packs add-only: merge by union, never remove one. (spec:2026-09-27-sources-loro-design, plan:2026-10-01-packs-portables)`
    to
    `- Keep bundle assets, fonts, packs and the usage ledger add-only: merge by union, never remove one. (spec:2026-09-27-sources-loro-design, plan:2026-10-01-packs-portables, spec:2026-10-30-deck-usage-ledger-design)`
    and add after it
    `- Keep usage out of Deck and the replica; read usage.jsonl last under its own cap, never failing the deck, and never bump SOURCES_VERSION for it. (spec:2026-10-30-deck-usage-ledger-design)`
  - Run `just agents-lint` and `wc -l crates/oku-slides-core/ARCHITECTURE.md` (50).

- [ ] **Step 7: Commit** — `git add crates/oku-slides-core`, index checked, `git commit -F <file>`:
`feat(core): usage.jsonl in the sources zip, unioned on merge (t-1298, t-1296)`.

### Task 3: Aggregation (core)

**Files:**
- Create: `crates/oku-slides-core/src/usage_report.rs`, `crates/oku-slides-core/src/usage_report_tests.rs`
- Modify: `crates/oku-slides-core/src/lib.rs`, `ARCHITECTURE.md`
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (after `issue-usage-lines-dropped`)

**Interfaces:**
- Consumes: `UsageRecord`, `UsageTab`, `UsageCost` (Task 1).
- Produces (re-exported at the core root): `By { Day, Month, Model, Agent, Tab, Deck }` (`Debug, Clone, Copy, PartialEq,
  Eq, Serialize`, lowercase: `UsageReport` serializes it; no `Deserialize`, since mcp and cli parse their own
  enums and map them to it),
  `parse_day(&str) -> Result<time::Date, UsageRangeError>` (`YYYY-MM-DD`),
  `Range` (`Range::ALL`, `Range::new(Option<Date>, Option<Date>) -> Result<Range, UsageRangeError>`,
  `Range::parse(since: Option<&str>, until: Option<&str>) -> Result<Range, UsageRangeError>`, `holds(self, OffsetDateTime) -> bool`),
  `UsageRangeError` (opaque, `kind() -> UsageRangeErrorKind { BadDate, Reversed }`, `Localize`),
  `CostTotal { currency: String, amount: f64, partial: bool }`,
  `UsageRow { key: Option<String> (None: unknown model, and the totals row), turns, turns_without_tokens, input, output, thought, cache_read, cache_write, total: u64, costs: Vec<CostTotal> }`,
  `UsageReport { by: By, rows: Vec<UsageRow>, totals: UsageRow }` (all `Serialize`),
  `aggregate<'a>(records: impl IntoIterator<Item = (&'a str, &'a UsageRecord)>, by: By, range: Range) -> UsageReport`
  (the `&str` is the deck label, read only by `By::Deck`; one-deck callers pass `""`).
  With `parse_day` here, mcp and cli need no `time` dependency.

- [ ] **Step 1: Write the failing tests** — `crates/oku-slides-core/src/usage_report_tests.rs`

```rust
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;
use time::OffsetDateTime;
use time::macros::{date, datetime};

use super::*;
use crate::usage::usage_tests::record;
use crate::usage::{UsageCost, UsageRecord, UsageTab};

fn at(mut r: UsageRecord, when: OffsetDateTime) -> UsageRecord {
    r.at = when;
    r
}

fn report(rs: &[UsageRecord], by: By, range: Range) -> UsageReport {
    aggregate(rs.iter().map(|r| ("d", r)), by, range)
}

#[test]
fn aggregate_by_day_sums_tokens_per_utc_day() {
    let rs = [at(record(1, 100), datetime!(2026-10-30 23:59 UTC)), at(record(2, 50), datetime!(2026-10-31 00:01 UTC))];
    let r = report(&rs, By::Day, Range::ALL);
    let keys: Vec<_> = r.rows.iter().map(|row| row.key.clone()).collect();
    assert_that!(keys).is_equal_to(vec![Some("2026-10-30".to_owned()), Some("2026-10-31".to_owned())]);
    assert_that!((r.totals.input, r.totals.total)).is_equal_to((150, 170));
}

#[rstest]
#[case::month(By::Month, "2026-10")]
#[case::model(By::Model, "claude-opus-5-5")]
#[case::agent(By::Agent, "claude")]
#[case::tab(By::Tab, "deck")]
#[case::deck(By::Deck, "d")]
fn aggregate_groups_by_key(#[case] by: By, #[case] key: &str) {
    let r = report(&[record(1, 1)], by, Range::ALL);
    assert_that!(r.rows[0].key.as_deref()).is_equal_to(Some(key));
}

#[test]
fn aggregate_by_tab_names_pack_and_theme() {
    let mut pack = record(2, 1);
    pack.tab = UsageTab::Pack;
    let mut theme = record(3, 1);
    theme.tab = UsageTab::Theme;
    let r = report(&[record(1, 1), pack, theme], By::Tab, Range::ALL);
    let keys: Vec<_> = r.rows.iter().filter_map(|row| row.key.clone()).collect();
    assert_that!(keys).is_equal_to(vec!["deck".to_owned(), "pack".to_owned(), "theme".to_owned()]);
}

#[test]
fn aggregate_by_deck_keeps_decks_apart() {
    let (a, b) = (record(1, 1), record(2, 1));
    let r = aggregate([("a", &a), ("b", &b)], By::Deck, Range::ALL);
    assert_that!(r.rows).has_length(2);
}

#[test]
fn aggregate_without_model_has_unknown_key_first() {
    let mut r1 = record(1, 1);
    r1.model = None;
    let r = report(&[record(2, 1), r1], By::Model, Range::ALL);
    assert_that!(r.rows[0].key).is_none();
}

#[test]
fn aggregate_keeps_currencies_apart() {
    let mut eur = record(2, 1);
    eur.cost = Some(UsageCost { amount: 1.0, currency: "EUR".into() });
    let r = report(&[record(1, 1), eur], By::Day, Range::ALL);
    let currencies: Vec<_> = r.totals.costs.iter().map(|c| c.currency.as_str()).collect();
    assert_that!(currencies).is_equal_to(vec!["EUR", "USD"]);
}

#[test]
fn aggregate_sums_one_currency() {
    let r = report(&[record(1, 1), record(2, 1)], By::Day, Range::ALL);
    assert_that!(r.totals.costs).has_length(1);
    assert_that!((r.totals.costs[0].amount - 0.08).abs()).is_less_than(1e-9);
    assert_that!(r.totals.costs[0].partial).is_false();
}

#[test]
fn aggregate_missing_cost_marks_partial() {
    let mut free = record(2, 1);
    free.cost = None;
    let r = report(&[record(1, 1), free], By::Day, Range::ALL);
    assert_that!(r.totals.costs[0].partial).is_true();
}

#[test]
fn aggregate_counts_turns_without_tokens() {
    let mut blind = record(2, 1);
    blind.tokens = None;
    let r = report(&[record(1, 1), blind], By::Day, Range::ALL);
    assert_that!((r.totals.turns, r.totals.turns_without_tokens)).is_equal_to((2, 1));
}

#[test]
fn aggregate_unreported_token_kinds_count_as_zero() {
    let r = report(&[record(1, 1)], By::Day, Range::ALL);
    assert_that!((r.totals.thought, r.totals.cache_read, r.totals.cache_write)).is_equal_to((0, 5, 0));
}

#[test]
fn aggregate_range_is_inclusive() {
    let rs = [
        at(record(1, 1), datetime!(2026-10-29 12:00 UTC)),
        at(record(2, 1), datetime!(2026-10-30 00:00 UTC)),
        at(record(3, 1), datetime!(2026-10-31 23:59 UTC)),
        at(record(4, 1), datetime!(2026-11-01 00:00 UTC)),
    ];
    let range = Range::new(Some(date!(2026-10-30)), Some(date!(2026-10-31))).unwrap();
    assert_that!(report(&rs, By::Day, range).totals.turns).is_equal_to(2);
}

#[test]
fn aggregate_empty_returns_no_rows_and_zero_totals() {
    let r = report(&[], By::Day, Range::ALL);
    assert_that!(r.rows).is_empty();
    assert_that!(r.totals.turns).is_equal_to(0);
}

#[test]
fn range_since_after_until_is_refused() {
    let err = Range::new(Some(date!(2026-10-31)), Some(date!(2026-10-30))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(UsageRangeErrorKind::Reversed);
}

#[rstest]
#[case::open(None, None)]
#[case::since_only(Some("2026-10-30"), None)]
#[case::until_only(None, Some("2026-10-30"))]
#[case::same_day(Some("2026-10-30"), Some("2026-10-30"))]
fn range_parse_valid_bounds_is_ok(#[case] since: Option<&str>, #[case] until: Option<&str>) {
    assert_that!(Range::parse(since, until)).is_ok();
}

#[rstest]
#[case::words("yesterday")]
#[case::slashes("2026/10/30")]
#[case::no_day("2026-10")]
#[case::bad_month("2026-13-01")]
fn parse_day_bad_text_returns_bad_date(#[case] text: &str) {
    assert_that!(parse_day(text).map_err(|e| e.kind())).is_equal_to(Err(UsageRangeErrorKind::BadDate));
}

#[test]
fn parse_day_iso_date_returns_it() {
    assert_that!(parse_day("2026-10-30").ok()).is_equal_to(Some(date!(2026-10-30)));
}

#[test]
fn range_errors_localize_in_french() {
    let fr = Localizer::new(Some("fr"));
    let bad = parse_day("hier").unwrap_err().localize(&fr);
    assert_that!(bad.as_str()).contains("hier");
    let reversed = Range::parse(Some("2026-10-31"), Some("2026-10-30")).unwrap_err().localize(&fr);
    assert_that!(reversed.as_str()).contains("2026-10-31");
    assert_that!(reversed.as_str()).contains("après");
}

#[test]
fn usage_report_serializes_by_lowercase_and_skips_internals() {
    let json = serde_json::to_value(report(&[record(1, 1)], By::Day, Range::ALL)).unwrap();
    assert_that!(json["by"].as_str()).is_equal_to(Some("day"));
    assert_that!(json["totals"].get("with_cost")).is_none();
}
```

- [ ] **Step 2: Run to fail** — `cargo nextest run -p oku-slides-core usage_report`. Expected: module not found.

- [ ] **Step 3: Implement `usage_report.rs`**

```rust
//! Usage reports: a ledger's turns summed by day, month, model, agent, tab or deck.
//!
//! [`aggregate`] is the only usage sum: the CLI and the agent tools both call it. Days and months
//! are UTC; costs are never summed across currencies, and a record's amount is at most 1e9, so
//! a sum stays finite.

use std::collections::BTreeMap;

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;
use snafu::{Backtrace, Snafu};
use time::macros::format_description;
use time::{Date, OffsetDateTime};

use crate::usage::{UsageRecord, UsageTab};

/// What a report's rows group by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum By {
    /// UTC day, `YYYY-MM-DD`.
    Day,
    /// UTC month, `YYYY-MM`.
    Month,
    /// The session's model.
    Model,
    /// The agent preset.
    Agent,
    /// The tab kind.
    Tab,
    /// The deck, by its label.
    Deck,
}

/// A usage date range that cannot be used.
#[derive(Debug, Snafu)]
pub struct UsageRangeError(RangeInner);

/// What is wrong with a [`UsageRangeError`]'s range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UsageRangeErrorKind {
    /// A bound is not a `YYYY-MM-DD` date.
    BadDate,
    /// `since` is after `until`.
    Reversed,
}

#[derive(Debug, Snafu)]
enum RangeInner {
    #[snafu(display("invalid date {text}, expected YYYY-MM-DD"))]
    BadDate { text: String, backtrace: Backtrace },
    #[snafu(display("since {since} is after until {until}"))]
    Reversed { since: Date, until: Date, backtrace: Backtrace },
}

impl UsageRangeError {
    /// What is wrong with the range.
    #[must_use]
    pub fn kind(&self) -> UsageRangeErrorKind {
        match self.0 {
            RangeInner::BadDate { .. } => UsageRangeErrorKind::BadDate,
            RangeInner::Reversed { .. } => UsageRangeErrorKind::Reversed,
        }
    }
}

impl Localize for UsageRangeError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            RangeInner::BadDate { text, .. } => l.tr_with("usage-range-bad-date", &[("date", text.as_str().into())]),
            RangeInner::Reversed { since, until, .. } => l.tr_with(
                "usage-range-reversed",
                &[("since", since.to_string().into()), ("until", until.to_string().into())],
            ),
        }
    }
}

/// Reads a `YYYY-MM-DD` UTC day.
///
/// # Errors
///
/// [`UsageRangeErrorKind::BadDate`]: not such a date (the text is kept, cut to 32 characters).
pub fn parse_day(text: &str) -> Result<Date, UsageRangeError> {
    Date::parse(text, format_description!("[year]-[month]-[day]"))
        .map_err(|_| BadDateSnafu { text: text.chars().take(32).collect::<String>() }.build().into())
}

/// Inclusive UTC dates a report keeps; open ends allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    since: Option<Date>,
    until: Option<Date>,
}

impl Range {
    /// Every turn.
    pub const ALL: Self = Self { since: None, until: None };

    /// A range from optional bounds.
    ///
    /// # Errors
    ///
    /// [`UsageRangeErrorKind::Reversed`]: `since` after `until`.
    pub fn new(since: Option<Date>, until: Option<Date>) -> Result<Self, UsageRangeError> {
        match (since, until) {
            (Some(since), Some(until)) if since > until => Err(ReversedSnafu { since, until }.build().into()),
            _ => Ok(Self { since, until }),
        }
    }

    /// A range from optional `YYYY-MM-DD` bounds, as the CLI and the tools take them.
    ///
    /// # Errors
    ///
    /// A bound that is not a date, or `since` after `until`.
    pub fn parse(since: Option<&str>, until: Option<&str>) -> Result<Self, UsageRangeError> {
        Self::new(since.map(parse_day).transpose()?, until.map(parse_day).transpose()?)
    }

    /// Whether the UTC day of `at` is in the range.
    #[must_use]
    pub fn holds(self, at: OffsetDateTime) -> bool {
        let day = at.date();
        self.since.is_none_or(|s| day >= s) && self.until.is_none_or(|u| day <= u)
    }
}

/// A currency's summed cost; `partial` when some turns had none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostTotal {
    /// ISO 4217 code.
    pub currency: String,
    /// Sum of the turns' amounts.
    pub amount: f64,
    /// Some turns of the row reported no cost.
    pub partial: bool,
}

/// One report row: the sums of the turns sharing its key.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UsageRow {
    /// The group's key; `None`: unknown (no model reported).
    pub key: Option<String>,
    /// Turns in the row.
    pub turns: u64,
    /// Turns whose reply had no token counts.
    pub turns_without_tokens: u64,
    /// Input tokens.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Reasoning tokens.
    pub thought: u64,
    /// Cache read tokens.
    pub cache_read: u64,
    /// Cache write tokens.
    pub cache_write: u64,
    /// All tokens, as the agents counted them.
    pub total: u64,
    /// Costs by currency, in currency order.
    pub costs: Vec<CostTotal>,
    /// Turns that reported a cost, to mark costs partial.
    #[serde(skip)]
    with_cost: u64,
}

impl UsageRow {
    fn add(&mut self, r: &UsageRecord) {
        self.turns += 1;
        match r.tokens {
            Some(t) => {
                self.input = self.input.saturating_add(t.input);
                self.output = self.output.saturating_add(t.output);
                self.thought = self.thought.saturating_add(t.thought.unwrap_or(0));
                self.cache_read = self.cache_read.saturating_add(t.cache_read.unwrap_or(0));
                self.cache_write = self.cache_write.saturating_add(t.cache_write.unwrap_or(0));
                self.total = self.total.saturating_add(t.total);
            }
            None => self.turns_without_tokens += 1,
        }
        if let Some(c) = &r.cost {
            self.with_cost += 1;
            match self.costs.binary_search_by(|t| t.currency.as_str().cmp(&c.currency)) {
                Ok(i) => self.costs[i].amount += c.amount,
                Err(i) => {
                    self.costs.insert(i, CostTotal { currency: c.currency.clone(), amount: c.amount, partial: false });
                }
            }
        }
    }

    fn close(mut self) -> Self {
        let partial = self.with_cost < self.turns;
        for c in &mut self.costs {
            c.partial = partial;
        }
        self
    }
}

/// A report: rows in key order, then their totals.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageReport {
    /// What the rows group by.
    pub by: By,
    /// The rows, unknown key first.
    pub rows: Vec<UsageRow>,
    /// All rows together.
    pub totals: UsageRow,
}

/// Sums `records` (deck label, record) by `by`, keeping those in `range`.
#[must_use]
pub fn aggregate<'a>(records: impl IntoIterator<Item = (&'a str, &'a UsageRecord)>, by: By, range: Range) -> UsageReport {
    let mut rows: BTreeMap<Option<String>, UsageRow> = BTreeMap::new();
    let mut totals = UsageRow::default();
    for (deck, r) in records {
        if !range.holds(r.at) {
            continue;
        }
        let day = r.at.date();
        let key = match by {
            By::Day => Some(format!("{:04}-{:02}-{:02}", day.year(), u8::from(day.month()), day.day())),
            By::Month => Some(format!("{:04}-{:02}", day.year(), u8::from(day.month()))),
            By::Model => r.model.clone(),
            By::Agent => Some(r.agent.clone()),
            By::Tab => Some(tab_key(r.tab).to_owned()),
            By::Deck => Some(deck.to_owned()),
        };
        rows.entry(key.clone()).or_insert_with(|| UsageRow { key, ..UsageRow::default() }).add(r);
        totals.add(r);
    }
    UsageReport { by, rows: rows.into_values().map(UsageRow::close).collect(), totals: totals.close() }
}

fn tab_key(tab: UsageTab) -> &'static str {
    match tab {
        UsageTab::Deck => "deck",
        UsageTab::Pack => "pack",
        UsageTab::Theme => "theme",
    }
}

#[cfg(test)]
#[path = "usage_report_tests.rs"]
mod usage_report_tests;
```

- [ ] **Step 4: Fluent keys** — `core.ftl`, all 20 locales, right after `issue-usage-lines-dropped`. English:

```
usage-range-bad-date = “{ $date }” is not a date: use YYYY-MM-DD.
usage-range-reversed = The start date { $since } is after the end date { $until }.
```

French:

```
usage-range-bad-date = « { $date } » n'est pas une date : utilisez AAAA-MM-JJ.
usage-range-reversed = La date de début { $since } est après la date de fin { $until }.
```

The 18 others per `crates/oku-slides-i18n/AGENTS.md` (the `YYYY-MM-DD` hint in each locale's own letters
where it has them, as the French does).

- [ ] **Step 5: Run** — `cargo nextest run -p oku-slides-core usage_report`, `cargo nextest run -p oku-slides-i18n`,
`cargo clippy -p oku-slides-core --all-targets -- -D warnings`. Expected: PASS.

- [ ] **Step 6: Export and document**
  - `lib.rs`: `pub mod usage_report;` (after `pub mod usage;`) and
    `pub use usage_report::{By, CostTotal, Range, UsageRangeError, UsageRangeErrorKind, UsageReport, UsageRow, aggregate, parse_day};`;
    the crate-doc bullet from Task 1 becomes
    `//! - [`UsageLedger`]: the deck's agent usage, one [`UsageRecord`] per turn, add-only; [`aggregate`] sums it into a [`UsageReport`].`
  - `ARCHITECTURE.md` (zero net lines): the Task 1 line

```
  `num.rs`: float/int casts. `error/` (Snafu, Fluent), `reason.rs`: refusals. `usage.rs`: `UsageLedger` (add-only, capped reads).
```

    becomes

```
  `num.rs`: float/int casts. `error/`, `reason.rs`: refusals. `usage.rs`: `UsageLedger`; `usage_report.rs`: `aggregate`, `Range`.
```

    Check `wc -l` still prints 50.

- [ ] **Step 7: Commit** — `git add crates/oku-slides-core crates/oku-slides-i18n`, index checked, `git commit -F <file>`:
`feat(core): usage reports by day, month, model, agent, tab, deck (t-1299, t-1296)`.

### Task 4: Record, route and resume the ledger (session)

The deck's real document depends on the mode: `self.doc` in deck mode, `stash` while history is open.
A pack tab (`Mode::Pack`, always a pack tab: `is_pack_tab`) holds only a preview copy of its opener's
deck, and a theme or help tab has no deck: they record nothing themselves, their turns are routed to the
owning deck tab by the workspace. A record never sets `dirty`; it sets `Document.usage_unsaved` and
snapshots the resume at once (`persist(true)`). The resume takes the ledger from the snapshotted
document itself (never from the borrowed `Blobs` source, which in history is the preview's), as an
`Arc<UsageLedger>` of its own in `DocSnapshot`: `Blobs` and its cache are untouched, so a new record
never copies the deck's assets. The session reuses the `Arc` while the ledger is unchanged, and the
resume writer rewrites `usage.jsonl` only when it receives a different `Arc` from the one it last wrote.

**Files:**
- Create: `crates/oku-slides-session/src/deck_usage.rs`, `crates/oku-slides-session/src/deck_usage_tests.rs`
- Create: `crates/oku-slides-session/src/resume_usage.rs` (child module of `resume`, so it reaches `put`)
- Create: `crates/oku-slides-session/src/workspace_usage.rs`, `crates/oku-slides-session/src/workspace_usage_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (`Document` :187-212, `DocDefaults` :216-223 and :454, `Session.sent_blobs` :330-332 and :510, `stop_resume` :655-658, `mod` list :38-120)
- Modify: `crates/oku-slides-session/src/resume.rs` (imports :23-26, `DocSnapshot` :174-230, `DocOut`/`DocIn` :240-258, `read_doc` :506-590, `fresh_from_disk` :738-754, `persist_with` :799)
- Modify: `crates/oku-slides-session/src/resume_writer.rs` (`Shared` :58-62, `send` :101-114, `run` :143-164, `write` :166-183)
- Modify: `crates/oku-slides-session/src/deck_file.rs` (`write_to` :174-181)
- Modify: `crates/oku-slides-session/src/agent/tabs.rs` (trait `Sessions` :50-141, `impl Sessions for Mutex<Session>` :144-185, `TabSession` :186-283)
- Modify: `crates/oku-slides-session/src/workspace.rs` (module list :343-404), `crates/oku-slides-session/src/workspace_agent.rs` (`impl Sessions for Mutex<Workspace>` :74-108)
- Modify: `src-tauri/src/state.rs` (`impl Sessions for AppSessions` :115-188)
- Modify (test doubles, `usage_owner` is required): `crates/oku-slides-session/src/agent/tabs_tests.rs:13`,
  `crates/oku-slides-mcp/tests/common/mod.rs` (:305, :345, :389, :476), `crates/oku-slides-mcp/tests/common/other_tabs.rs:53`,
  `crates/oku-slides-mcp/src/hub_tests.rs:16`, `crates/oku-slides-mcp/src/hub_thread_tests.rs:13`,
  `crates/oku-slides-agent/tests/common/mod.rs:305`
- Test: `crates/oku-slides-session/src/resume_tests.rs`, `crates/oku-slides-session/src/resume_writer_tests.rs` (their `defaults()` literals, new cases)
- Docs: `crates/oku-slides-session/ARCHITECTURE.md` (lines 8, 40, 45; zero net lines), `crates/oku-slides-session/AGENTS.md` (line 14; the file is at the lint's 60-line cap)

**Interfaces:**
- Consumes (Tasks 1-2): `UsageRecord` (with `extra`), `UsageLedger::{insert, union, is_empty, len, from_jsonl, to_jsonl}`,
  `Inserted::{Added, Same, Forged, Invalid, Full}`, `USAGE_FILE`, `USAGE_MAX_BYTES: u64`, `SourceBundle.usage`.
- Produces:
  - `Session::record_usage(&mut self, record: UsageRecord) -> bool` (true: added; false in a help, pack or
    theme tab, with `[usage] store_in_deck` off (Task 5), for a turn already there, or a full ledger).
  - `Session::usage_ledger(&self) -> Option<&UsageLedger>` (deck mode: the deck's; history: the stashed deck's; otherwise `None`).
  - `Workspace::usage_owner(ws: &Mutex<Workspace>, tab: &str) -> Option<String>` (pub; wakes nothing).
  - `Sessions::usage_owner(&self, tab: &str) -> Option<String>`, a **required** trait method (no default: a
    host that forgot it would lose pack and theme turns silently). `Mutex<Session>` returns `Some(tab)`;
    `Mutex<Workspace>` and `AppSessions` call `Workspace::usage_owner`; every test double implements it.
  - `TabSession::record_usage(&self, record: UsageRecord)` (blocking; never fails; a missing owner tab is a `tracing::warn!`).
  - `TabSession::usage_ledger(&self) -> crate::Result<Option<UsageLedger>>` (`Ok(None)`: no deck counts this tab's turns).
  - `Document.usage_unsaved: bool` (crate-private; in `document.json`, `#[serde(default)]`), cleared by `write_to`.
  - `Resume::write_usage(&self, sub: &str, usage: &UsageLedger) -> Result<()>` (`pub(crate)`, for the writer).

- [ ] **Step 1: Write the failing session tests** — `crates/oku-slides-session/src/deck_usage_tests.rs`

```rust
use speculoos::prelude::*;
use time::macros::datetime;
use uuid::Uuid;

use super::*;
use crate::test_support::{fresh_dir, session};

/// A valid usage record of turn `n` (core's own helper is `cfg(test)` of core).
pub(crate) fn record(n: u128) -> UsageRecord {
    UsageRecord {
        turn: Uuid::from_u128(n),
        at: datetime!(2026-10-30 14:02:11 UTC),
        tab: oku_slides_core::UsageTab::Deck,
        agent: "claude".into(),
        model: Some("claude-opus-5-5".into()),
        tokens: Some(oku_slides_core::UsageTokens {
            total: 110,
            input: 100,
            output: 10,
            thought: None,
            cache_read: Some(5),
            cache_write: None,
        }),
        cost: Some(oku_slides_core::UsageCost { amount: 0.04, currency: "USD".into() }),
        extra: serde_json::Map::new(),
    }
}

/// The ledger of the file at `path`, as a reader sees it.
pub(crate) fn ledger_in(path: &std::path::Path) -> UsageLedger {
    let s = session();
    crate::read_document(&s.storage, path, s.source_limits, &s.registry)
        .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.one_line()))
        .sources
        .usage
}

/// `s` saved as `name` in a fresh folder: the folder (kept alive) and the file.
fn saved(s: &mut Session, name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = fresh_dir("oku-slides-usage-");
    let path = dir.path().join(format!("{name}{}", crate::EXTENSION));
    s.save(Some(&path)).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    (dir, path)
}

#[test]
fn record_usage_in_deck_mode_adds_without_dirtying() {
    let mut s = session();
    assert_that!(s.record_usage(record(1))).is_true();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    assert_that!(s.unsaved()).is_false();
    assert_that!(s.doc.usage_unsaved).is_true();
}

#[test]
fn record_usage_same_turn_twice_adds_once() {
    let mut s = session();
    s.record_usage(record(1));
    assert_that!(s.record_usage(record(1))).is_false();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
}

#[test]
fn record_usage_on_a_full_ledger_refuses_and_warns_once() {
    let mut s = session();
    // Core's `MAX_RECORDS`.
    for n in 0..10_000 {
        s.record_usage(record(n));
    }
    assert_that!(s.record_usage(record(10_000))).is_false();
    assert_that!(s.doc.usage_full_warned).is_true();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(10_000));
}

#[test]
fn record_usage_in_history_mode_lands_in_the_stashed_deck() {
    let mut s = session();
    s.history_open().unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    assert_that!(s.record_usage(record(1))).is_true();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    s.history_close().unwrap_or_else(|e| panic!("close history: {}", e.one_line()));
    assert_that!(s.doc.sources.usage.len()).is_equal_to(1);
}

#[test]
fn record_usage_in_help_mode_records_nothing() {
    let mut s = session();
    s.open_help("fr", None).unwrap_or_else(|e| panic!("help: {}", e.one_line()));
    assert_that!(s.record_usage(record(1))).is_false();
    assert_that!(s.usage_ledger()).is_none();
}

#[test]
fn record_usage_writes_the_resume_ledger_at_once() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    let bytes = std::fs::read(dir.path().join("deck").join(oku_slides_core::USAGE_FILE)).unwrap();
    assert_that!(UsageLedger::from_jsonl(&bytes).0.len()).is_equal_to(1);
}

#[test]
fn record_usage_survives_a_resume() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    assert_that!(back.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    assert_that!(back.doc.usage_unsaved).is_true();
    assert_that!(back.unsaved()).is_false();
}

#[test]
fn record_usage_in_history_mode_survives_a_resume() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.history_open().unwrap_or_else(|e| panic!("history: {}", e.one_line()));
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    assert_that!(back.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
}

#[test]
fn resume_of_a_file_changed_on_disk_keeps_the_unsaved_records() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    let (_folder, path) = saved(&mut s, "union");
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    // Another copy of the app writes the file: a new hash, its own turn, not ours.
    let mut other = session();
    other.open(&path).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    other.record_usage(record(2));
    other.apply("Thème", vec![crate::test_support::set_theme_op("pro")], None).unwrap();
    other.save(None).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    let turns: Vec<Uuid> = back.usage_ledger().into_iter().flat_map(UsageLedger::records).map(|r| r.turn).collect();
    assert_that!(turns).is_equal_to(vec![Uuid::from_u128(1), Uuid::from_u128(2)]);
    assert_that!(back.doc.usage_unsaved).is_true();
}

#[test]
fn resume_of_a_file_holding_every_record_is_not_flagged_unsaved() {
    let dir = fresh_dir("oku-slides-usage-resume-");
    let mut s = session();
    let (_folder, path) = saved(&mut s, "held");
    assert_that!(s.resume(dir.path().to_owned())).is_true();
    s.record_usage(record(1));
    s.flush_resume();
    drop(s);
    // Another copy saved the same turn, and a change of its own.
    let mut other = session();
    other.open(&path).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    other.record_usage(record(1));
    other.apply("Thème", vec![crate::test_support::set_theme_op("pro")], None).unwrap();
    other.save(None).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    let mut back = session();
    assert_that!(back.resume(dir.path().to_owned())).is_true();
    assert_that!(back.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(1));
    assert_that!(back.doc.usage_unsaved).is_false();
}

#[test]
fn save_writes_the_ledger_and_clears_usage_unsaved() {
    let mut s = session();
    s.record_usage(record(1));
    let (_folder, path) = saved(&mut s, "saved");
    assert_that!(s.doc.usage_unsaved).is_false();
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
}
```

`crate::test_support::set_theme_op` exists (`test_support.rs:357`, used by `history_tests.rs:211` with `"pro"`).
Declare the file at the end of `deck_usage.rs`: `#[cfg(test)] #[path = "deck_usage_tests.rs"] pub(crate) mod deck_usage_tests;`
(`pub(crate)`: the workspace tests reuse `record` and `ledger_in`).

Add to `resume_tests.rs` (after `write_snapshot_removes_orphan_assets`; add `use std::sync::Arc;` if absent):

```rust
#[test]
fn with_blobs_new_usage_record_keeps_the_blobs_and_makes_a_new_ledger() {
    let mut doc = document();
    let (mut blobs, mut usage) = (Vec::new(), Vec::new());
    let first = DocSnapshot::with_blobs(&doc, &doc.sources, &mut blobs, &mut usage).unwrap();
    doc.sources.usage.insert(crate::deck_usage::deck_usage_tests::record(1));
    let second = DocSnapshot::with_blobs(&doc, &doc.sources, &mut blobs, &mut usage).unwrap();
    assert_that!(Arc::ptr_eq(first.blobs(), second.blobs())).is_true();
    assert_that!(Arc::ptr_eq(first.usage_arc(), second.usage_arc())).is_false();
    assert_that!(second.usage_arc().len()).is_equal_to(1);
}

#[test]
fn with_blobs_same_ledger_reuses_its_arc() {
    let mut doc = document();
    doc.sources.usage.insert(crate::deck_usage::deck_usage_tests::record(1));
    let (mut blobs, mut usage) = (Vec::new(), Vec::new());
    let first = DocSnapshot::with_blobs(&doc, &doc.sources, &mut blobs, &mut usage).unwrap();
    let second = DocSnapshot::with_blobs(&doc, &doc.sources, &mut blobs, &mut usage).unwrap();
    assert_that!(Arc::ptr_eq(first.usage_arc(), second.usage_arc())).is_true();
}

#[test]
fn write_usage_then_read_doc_keeps_usage_and_its_flag() {
    let r = resume();
    let mut doc = document();
    doc.sources.usage.insert(crate::deck_usage::deck_usage_tests::record(1));
    doc.usage_unsaved = true;
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    r.write_usage(DECK_DIR, &doc.sources.usage).unwrap();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources.usage).is_equal_to(&doc.sources.usage);
    assert_that!(back.usage_unsaved).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn write_usage_empty_ledger_removes_usage_file() {
    let r = resume();
    let mut doc = document();
    doc.sources.usage.insert(crate::deck_usage::deck_usage_tests::record(1));
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    r.write_usage(DECK_DIR, &doc.sources.usage).unwrap();
    let file = r.dir.join(DECK_DIR).join(oku_slides_core::USAGE_FILE);
    assert_that!(file.exists()).is_true();
    r.write_usage(DECK_DIR, &oku_slides_core::UsageLedger::default()).unwrap();
    assert_that!(file.exists()).is_false();
    std::fs::remove_dir_all(&r.dir).ok();
}

#[test]
fn read_doc_bad_usage_line_is_set_aside_and_the_doc_opens() {
    let r = resume();
    let doc = document();
    r.write_snapshot(DECK_DIR, &DocSnapshot::of(&doc, &mut Vec::new()).unwrap()).unwrap();
    std::fs::write(r.dir.join(DECK_DIR).join(oku_slides_core::USAGE_FILE), b"{not json}\n").unwrap();
    let back = r.read_doc(DECK_DIR, &defaults(), 1 << 24, &registry()).unwrap();
    assert_that!(back.sources.usage.is_empty()).is_true();
    std::fs::remove_dir_all(&r.dir).ok();
}
```

Add to `resume_writer_tests.rs` (after `writer_merged_jobs_keep_the_latest_documents_and_state`; add
`use std::sync::Arc;` and `use oku_slides_core::{USAGE_FILE, UsageLedger};`):

```rust
/// A deck job whose snapshot carries that very `usage` (its ledger equals it, the cache holds it).
fn job_with_usage(usage: &Arc<UsageLedger>) -> Job {
    let mut doc = document("pro");
    doc.sources.usage = (**usage).clone();
    let snapshot = DocSnapshot::with_blobs(&doc, &doc.sources, &mut Vec::new(), &mut vec![Arc::clone(usage)]).unwrap();
    Job { docs: vec![(DECK_DIR, snapshot)], state: state("s-1"), agent: None }
}

/// The writer rewrites `usage.jsonl` only for a ledger it has not written yet.
#[test]
fn writer_same_usage_arc_is_written_once() {
    let dir = temp_dir();
    let w = Writer::new(resume(&dir));
    let mut ledger = UsageLedger::default();
    ledger.insert(crate::deck_usage::deck_usage_tests::record(1));
    let usage = Arc::new(ledger);
    w.send(job_with_usage(&usage));
    w.flush();
    let file = dir.join(DECK_DIR).join(USAGE_FILE);
    std::fs::remove_file(&file).unwrap();
    w.send(job_with_usage(&usage));
    w.flush();
    assert_that!(file.exists()).is_false();
    let mut grown = (*usage).clone();
    grown.insert(crate::deck_usage::deck_usage_tests::record(2));
    w.send(job_with_usage(&Arc::new(grown)));
    w.flush();
    assert_that!(UsageLedger::from_jsonl(&std::fs::read(&file).unwrap()).0.len()).is_equal_to(2);
    std::fs::remove_dir_all(&dir).ok();
}
```

In `resume_tests.rs:10-12` and `resume_writer_tests.rs:17-19`, the
`defaults()` literal becomes `DocDefaults { undo_limit: 10, author: None, store_usage: true }`.

- [ ] **Step 2: Write the failing routing tests** — `crates/oku-slides-session/src/workspace_usage_tests.rs`

```rust
use std::sync::Arc;

use serde_json::json;
use speculoos::prelude::*;

use super::super::packs::workspace_packs_fixtures::{Packs, deck_packs, packs};
use crate::agent::{Sessions, TabOpen, TabSession};
use crate::deck_usage::deck_usage_tests::record;
use crate::theme::PAPIER_THEME;
use crate::{DocumentKind, TabDocument, Workspace};

/// `tab`'s agent view of the fixture's workspace.
fn tab_session(f: &Packs, tab: &str) -> TabSession {
    let sessions: Arc<dyn Sessions> = f.ws.clone();
    TabSession::new(tab, sessions)
}

/// The number of turns `t-1`'s deck holds.
fn deck_turns(f: &Packs) -> usize {
    f.with("t-1", |s| s.usage_ledger().map_or(0, oku_slides_core::UsageLedger::len))
}

#[test]
fn usage_owner_of_a_deck_tab_is_itself() {
    let f = packs();
    assert_that!(Workspace::usage_owner(&f.ws, "t-1")).is_equal_to(Some("t-1".to_owned()));
}

#[test]
fn record_usage_from_user_opened_pack_tab_lands_in_opener_deck() {
    let f = packs();
    let pack_tab = f.pack_tab();
    tab_session(&f, &pack_tab).record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(1);
    assert_that!(f.with(&pack_tab, |s| s.usage_ledger().is_none())).is_true();
}

#[test]
fn record_usage_from_agent_opened_pack_tab_records_nothing() {
    let f = packs();
    let tab = match f.open_by_agent("t-1", f.acme, &mut |_| true) {
        Ok(TabOpen::Opened { tab, .. }) => tab,
        other => panic!("a new pack tab: {other:?}"),
    };
    let agent_tab = tab_session(&f, &tab);
    agent_tab.record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(0);
    assert_that!(agent_tab.usage_ledger().unwrap_or_else(|e| panic!("{}", e.one_line()))).is_none();
}

#[test]
fn record_usage_from_user_opened_theme_tab_lands_in_opener_deck() {
    let f = packs();
    f.send(json!({"cmd": "open_theme", "tab": "t-1", "theme": PAPIER_THEME}));
    let theme_tab = f.active(0);
    assert_that!(f.document(&theme_tab)).is_equal_to(Some(TabDocument::Theme { theme: PAPIER_THEME }));
    tab_session(&f, &theme_tab).record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(1);
}

#[test]
fn record_usage_from_help_tab_records_nothing() {
    let f = packs();
    f.send(json!({"cmd": "open_help", "window": "w-1"}));
    let help = f.active(0);
    assert_that!(f.document(&help).map(|d| d.kind())).is_equal_to(Some(DocumentKind::Help));
    assert_that!(Workspace::usage_owner(&f.ws, &help)).is_none();
    tab_session(&f, &help).record_usage(record(1));
    assert_that!(deck_turns(&f)).is_equal_to(0);
}

#[test]
fn usage_ledger_of_deck_pack_tab_reads_owner_deck() {
    let (f, pin) = deck_packs();
    let deck_pack = f.deck_pack_tab(pin.id);
    tab_session(&f, "t-1").record_usage(record(1));
    let ledger = tab_session(&f, &deck_pack).usage_ledger().unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(ledger.map(|l| l.len())).is_equal_to(Some(1));
}
```

Declare it at the end of `workspace_usage.rs`: `#[cfg(test)] #[path = "workspace_usage_tests.rs"] mod workspace_usage_tests;`.

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo nextest run -p oku-slides-session usage` (first run in this worktree builds cold: run it in the background).
Expected: compile errors (`record_usage`, `usage_unsaved`, `usage_owner`, `store_usage`, `write_usage` not found).

- [ ] **Step 4: Add the document flags and the setting slot** in `lib.rs`

In `struct Document` (after `disk_replaced`):

```rust
    /// Agent usage recorded since the last save: the file does not hold it yet.
    ///
    /// Never makes the deck unsaved: a clean tab that closes writes it alone.
    usage_unsaved: bool,
    /// The full ledger's refusal is already logged: logged once per document.
    usage_full_warned: bool,
```

`Document::new` (:211) builds them with `usage_unsaved: false, usage_full_warned: false`. In `struct DocDefaults` (after `author`):

```rust
    /// Agent usage is recorded and saved (`[usage] store_in_deck`).
    store_usage: bool,
```

and :454 becomes (Task 5 adds `settings.usage`; until then `true`):

```rust
        let doc_defaults =
            DocDefaults { undo_limit: settings.history.limit, author: settings.author.resolved(), store_usage: true };
```

In `struct Session`, after `sent_blobs` (:332):

```rust
    /// Usage ledgers already sent to the resume thread: shared while unchanged,
    /// so the thread rewrites `usage.jsonl` only for a new one.
    sent_usage: Vec<Arc<oku_slides_core::UsageLedger>>,
```

built `sent_usage: Vec::new(),` after `sent_blobs: Vec::new(),` (:510) and cleared in `stop_resume`
(`self.sent_usage.clear();` after `self.sent_blobs.clear();`).

Add `mod deck_usage;` to the module list, between `mod deck_theme_edit;` and `mod deck_widgets;`.

- [ ] **Step 5: Implement `deck_usage.rs`**

```rust
//! The deck's agent usage: turns recorded without making the deck unsaved.
//!
//! A record goes to the deck's own document (the stash while history is
//! open), snapshots the resume at once and waits for the next save; a
//! clean tab that closes writes it alone.

use oku_slides_core::{Inserted, UsageLedger, UsageRecord};

use crate::{Document, Mode, Session};

impl Session {
    /// Records a turn's agent usage in the deck; false when nothing was added.
    ///
    /// Only in deck mode or, while history is open, in the deck set aside: a
    /// help, pack or theme tab records nothing (the workspace routes their
    /// turns to their deck tab), nor does `[usage] store_in_deck` off.
    pub fn record_usage(&mut self, record: UsageRecord) -> bool {
        if !self.doc_defaults.store_usage {
            return false;
        }
        let turn = record.turn;
        let Some(doc) = self.usage_document_mut() else { return false };
        let added = match doc.sources.usage.insert(record) {
            Inserted::Added => {
                doc.usage_unsaved = true;
                true
            }
            Inserted::Same => false,
            Inserted::Full => {
                if !doc.usage_full_warned {
                    doc.usage_full_warned = true;
                    tracing::warn!(%turn, "agent usage ledger full: later turns are not recorded");
                }
                false
            }
            Inserted::Forged | Inserted::Invalid => {
                tracing::warn!(%turn, "agent usage record refused by the ledger");
                false
            }
        };
        if added {
            self.persist(true);
        }
        added
    }

    /// The deck's usage ledger; `None` in a help, pack or theme tab.
    #[must_use]
    pub fn usage_ledger(&self) -> Option<&UsageLedger> {
        match &self.mode {
            Mode::Deck => Some(&self.doc.sources.usage),
            Mode::History { stash, .. } => Some(&stash.sources.usage),
            Mode::Help { .. } | Mode::Pack { .. } | Mode::Theme { .. } => None,
        }
    }

    /// The document whose ledger takes a turn: the deck, or the deck set aside.
    fn usage_document_mut(&mut self) -> Option<&mut Document> {
        match &mut self.mode {
            Mode::Deck => Some(&mut self.doc),
            Mode::History { stash, .. } => Some(&mut **stash),
            Mode::Help { .. } | Mode::Pack { .. } | Mode::Theme { .. } => None,
        }
    }
}
```

`Session::save_pending_usage` joins this file in Task 5. `UsageRecord.turn` is a `Uuid` (`Display`).

- [ ] **Step 6: Persist the ledger in the resume** (`resume.rs`, `resume_writer.rs` and the new `resume_usage.rs`)

`Blobs` and `Blobs::matches` do not change. Add `UsageLedger` to the `oku_slides_core` import list
(:23-26). `DocSnapshot` gains, after `blobs`:

```rust
    /// The snapshotted document's own ledger (never the lending preview's),
    /// shared with the previous snapshot while it is unchanged.
    usage: Arc<UsageLedger>,
    usage_unsaved: bool,
```

and an accessor next to `blobs()`, for the writer (below) and the tests:

```rust
    /// The ledger to write, shared with the session's previous snapshots.
    pub(crate) fn usage_arc(&self) -> &Arc<UsageLedger> {
        &self.usage
    }
```

`DocSnapshot::of` (cfg(test)) becomes `Self::with_blobs(doc, &doc.sources, cache, &mut Vec::new())`.
`with_blobs` takes the session's usage cache as a fourth argument and fills both fields:

```rust
    /// Comme `of`, les images et polices prises à `sources` (celles du deck
    /// prêtées à l'aperçu de l'historique) ; le journal d'usage, celui de `doc`,
    /// partagé par `usage_cache` tant qu'il ne change pas.
    pub(crate) fn with_blobs(
        doc: &Document,
        sources: &SourceBundle,
        cache: &mut Vec<Arc<Blobs>>,
        usage_cache: &mut Vec<Arc<UsageLedger>>,
    ) -> Result<Self, ReplicaError> {
```

and, before `Ok(Self { … })`:

```rust
        // Compared, not cloned, while unchanged: a turn copies the ledger once.
        let usage = if let Some(u) = usage_cache.iter().find(|u| ***u == doc.sources.usage) {
            Arc::clone(u)
        } else {
            let u = Arc::new(doc.sources.usage.clone());
            // Deck and edited pack: two documents at most.
            if usage_cache.len() >= 2 {
                usage_cache.remove(0);
            }
            usage_cache.push(Arc::clone(&u));
            u
        };
```

with `usage, usage_unsaved: doc.usage_unsaved,` added to the returned `Self { … }`. In `persist_with`
(:799) the call becomes `DocSnapshot::with_blobs(doc, &blobs.sources, &mut self.sent_blobs, &mut self.sent_usage)`.

`DocOut` gains `usage_unsaved: bool`; `DocIn` gains `#[serde(default)] usage_unsaved: bool` (a
`document.json` written before this change reads `false`). `write_snapshot` fills
`usage_unsaved: doc.usage_unsaved` in `out` and writes no usage itself: the writer thread does (below).

In `read_doc`, `let (sources, issues)` becomes `let (mut sources, issues)`, then after its issue loop:

```rust
        sources.usage = self.read_usage(&dir);
```

and after `doc.blank = raw.blank;`: `doc.usage_unsaved = raw.usage_unsaved;`.

Declare the child module after the imports of `resume.rs`: `#[path = "resume_usage.rs"] mod usage;`. Then
`resume_usage.rs`:

```rust
//! The resume's copy of a deck's agent usage: `deck/usage.jsonl`.
//!
//! Untrusted on read like the rest of the resume: a bounded read, bad lines
//! set aside (logged), never a resume that fails because of it.

use std::path::Path;

use oku_slides_core::{USAGE_FILE, USAGE_MAX_BYTES, UsageLedger};
use oku_slides_platform::storage::StorageErrorKind;
use snafu::ResultExt;

use super::{AccessSnafu, Result, Resume};

impl Resume {
    /// Writes `usage` as `<sub>/usage.jsonl`; an empty ledger removes the file.
    pub(crate) fn write_usage(&self, sub: &str, usage: &UsageLedger) -> Result<()> {
        let path = self.dir.join(sub).join(USAGE_FILE);
        if usage.is_empty() {
            return self.storage.delete(&path).context(AccessSnafu { path: &path });
        }
        self.put(&path, usage.to_jsonl())
    }

    /// Reads `dir/usage.jsonl`: missing, empty; unreadable or too large, empty and logged.
    pub(super) fn read_usage(&self, dir: &Path) -> UsageLedger {
        let path = dir.join(USAGE_FILE);
        let bytes = match self.storage.read_limited(&path, USAGE_MAX_BYTES) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == StorageErrorKind::NotFound => return UsageLedger::default(),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %oku_slides_platform::errors::one_line(&e),
                    "resume usage ledger unreadable: set aside"
                );
                return UsageLedger::default();
            }
        };
        let (ledger, issues) = UsageLedger::from_jsonl(&bytes);
        for issue in &issues {
            tracing::warn!(issue = %issue, "resume usage line set aside");
        }
        ledger
    }
}
```

(`USAGE_MAX_BYTES` is a `u64`, the type `Storage::read_limited` takes.)

`resume_writer.rs`: `Shared` gains the ledger each sub-folder holds on disk; `write` rewrites a ledger
only when its `Arc` differs from the one it last wrote. A merged job keeps only the newest snapshot,
which always carries the whole ledger, so a skipped write never loses a record. Add
`use oku_slides_core::UsageLedger;` and:

```rust
/// The usage ledger of each sub-folder as this writer last wrote it.
type UsageOnDisk = Vec<(&'static str, Arc<UsageLedger>)>;
```

```rust
#[derive(Default)]
struct Shared {
    slot: Mutex<Slot>,
    changed: Condvar,
    /// Locked only by `write`, never with `slot`.
    usage_on_disk: Mutex<UsageOnDisk>,
}
```

In `send`'s inline fallback, `write(resume, &job)` becomes `write(resume, &job, &self.shared.usage_on_disk)`;
in `run`, `write(resume, &job)` becomes `write(resume, &job, &shared.usage_on_disk)`. `write` becomes:

```rust
fn write(resume: &Resume, job: &Job, usage_on_disk: &Mutex<UsageOnDisk>) {
    let written = job
        .docs
        .iter()
        .try_for_each(|(sub, doc)| {
            resume.write_snapshot(sub, doc)?;
            write_usage_once(resume, sub, doc.usage_arc(), usage_on_disk)
        })
        .and_then(|()| resume.write_state(&job.state));
    // … the rest of `write` unchanged (its log, then the conversation).
}

/// Writes `sub`'s usage ledger unless this writer already wrote that very one.
fn write_usage_once(
    resume: &Resume,
    sub: &'static str,
    usage: &Arc<UsageLedger>,
    on_disk: &Mutex<UsageOnDisk>,
) -> crate::resume::Result<()> {
    let mut on_disk = on_disk.lock().unwrap_or_else(PoisonError::into_inner);
    if on_disk.iter().any(|(s, u)| *s == sub && Arc::ptr_eq(u, usage)) {
        return Ok(());
    }
    resume.write_usage(sub, usage)?;
    on_disk.retain(|(s, _)| *s != sub);
    on_disk.push((sub, Arc::clone(usage)));
    Ok(())
}
```

`fresh_from_disk`, in the arm where the file changed (`Ok(opened) if …`), keeps the records the file
does not have yet (the ledger is add-only, so the union never loses one), and flags them unsaved only if
the file lacked some:

```rust
            Ok(opened) if saved.saved_hash.as_deref() != Some(opened.hash.as_str()) => {
                let notice = opened.notice(&self.localizer);
                let mut doc = Document::new(opened.sources, Some(path.clone()), &self.doc_defaults);
                doc.saved_hash = Some(opened.hash);
                doc.notice = notice;
                // The records not saved yet stay; the file's own come with it.
                let held = doc.sources.usage.len();
                for issue in doc.sources.usage.union(saved.sources.usage) {
                    tracing::warn!(issue = %issue, "resume usage record not merged with the file's");
                }
                doc.usage_unsaved = doc.sources.usage.len() > held;
                doc
            }
```

(`path` borrows `saved.path`; moving `saved.sources.usage` is a disjoint field move.)

In `deck_file.rs`, `write_to` also clears the flag: add `self.doc.usage_unsaved = false;` after
`self.doc.dirty = false;`.

- [ ] **Step 7: Route in `tabs.rs`, `workspace_usage.rs`, `workspace_agent.rs`, `state.rs` and the test doubles**

In trait `Sessions` (after `document_kind`), a required method:

```rust
    /// The deck tab whose ledger takes `tab`'s agent usage; `None`: not counted.
    ///
    /// Required: a multi-tab host routes a pack or theme tab's turns to its
    /// deck; a single-document host answers `tab` itself.
    ///
    /// # Blocking
    ///
    /// Takes the host's lock; wakes no dormant tab.
    fn usage_owner(&self, tab: &str) -> Option<String>;
```

In `impl Sessions for Mutex<Session>` (after `document_kind`):

```rust
    /// The session itself: `Session::record_usage` refuses outside a deck.
    fn usage_owner(&self, tab: &str) -> Option<String> {
        Some(tab.to_owned())
    }
```

On `TabSession` (after `document_kind`; add `use oku_slides_core::{UsageLedger, UsageRecord};`):

```rust
    /// Records a turn's usage in the deck that counts this tab's turns; blocking.
    ///
    /// Never fails the turn: a tab no deck counts records nothing, and an
    /// owner tab that is gone is logged.
    pub fn record_usage(&self, record: UsageRecord) {
        let Some(owner) = self.sessions.usage_owner(&self.tab) else { return };
        let mut record = Some(record);
        let done = self.sessions.with(&owner, &mut |s| {
            if let Some(r) = record.take() {
                s.record_usage(r);
            }
        });
        if let Err(e) = done {
            tracing::warn!(tab = %self.tab, owner = %owner, error = %e.one_line(), "agent usage not recorded");
        }
    }

    /// A copy of the usage ledger of the deck that counts this tab's turns; blocking.
    ///
    /// `None`: no deck counts them (a help tab, a tab the agent opened).
    ///
    /// # Errors
    ///
    /// Fails if the owner tab is unknown to the host.
    pub fn usage_ledger(&self) -> crate::Result<Option<UsageLedger>> {
        let Some(owner) = self.sessions.usage_owner(&self.tab) else { return Ok(None) };
        let mut out = None;
        self.sessions.with(&owner, &mut |s| out = s.usage_ledger().cloned())?;
        Ok(out)
    }
```

`crates/oku-slides-session/src/workspace_usage.rs`:

```rust
//! The workspace and agent usage: which deck tab counts a tab's turns.
//!
//! A deck tab counts its own; a deck-pack tab its owner deck, a pack or
//! theme tab the deck that opened it (`Tab.opener`, set only when the user
//! opened it); a help tab, or a tab an agent opened, none.

use std::sync::Mutex;

use crate::DocumentKind;

use super::Workspace;

impl Workspace {
    /// The deck tab whose ledger takes `tab`'s agent usage; wakes nothing.
    ///
    /// # Blocking
    ///
    /// Takes the workspace lock alone.
    pub fn usage_owner(ws: &Mutex<Self>, tab: &str) -> Option<String> {
        let w = Self::lock(ws);
        let t = w.tabs.get(tab)?;
        match t.info.document.kind() {
            DocumentKind::Deck => Some(tab.to_owned()),
            DocumentKind::Pack | DocumentKind::Theme => {
                t.info.document.deck_tab().map(str::to_owned).or_else(|| t.opener.clone())
            }
            DocumentKind::Help => None,
        }
    }
}
```

Declare it in `workspace.rs` after `mod deck_packs_close;`: `#[path = "workspace_usage.rs"] mod usage;`.
`deck_tab` comes first: a deck-pack tab's owner is fixed, its opener only memory.

In `impl Sessions for Mutex<Workspace>` (`workspace_agent.rs`):

```rust
    fn usage_owner(&self, tab: &str) -> Option<String> {
        Workspace::usage_owner(self, tab)
    }
```

In `impl Sessions for AppSessions` (`src-tauri/src/state.rs`, after `document_kind`):

```rust
    /// Read from the workspace's cached tabs: wakes nothing.
    fn usage_owner(&self, tab: &str) -> Option<String> {
        Workspace::usage_owner(&self.workspace, tab)
    }
```

The test doubles, each after its `document_kind`:

- `crates/oku-slides-session/src/agent/tabs_tests.rs` (`Closed`), `crates/oku-slides-mcp/src/hub_tests.rs`
  and `crates/oku-slides-mcp/src/hub_thread_tests.rs` (`NoSession`):

  ```rust
      fn usage_owner(&self, _tab: &str) -> Option<String> {
          None
      }
  ```

- `crates/oku-slides-mcp/tests/common/mod.rs`: `ClosedTabs` →
  `fn usage_owner(&self, _tab: &str) -> Option<String> { self.0.usage_owner("onglet-ferme") }`;
  `HiddenTab` → `{ self.0.usage_owner(tab) }`; `OneFileElsewhere` and `GatedTabs` →
  `{ self.session.usage_owner(tab) }` (each `fn usage_owner(&self, tab: &str) -> Option<String>`).
- `crates/oku-slides-mcp/tests/common/other_tabs.rs` (`OtherTabs`) → `{ self.session.usage_owner(tab) }`.
- `crates/oku-slides-agent/tests/common/mod.rs` (`TabSessions`, its own sessions first, as `document_kind`):

  ```rust
      fn usage_owner(&self, tab: &str) -> Option<String> {
          if self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).contains_key(tab) {
              return Some(tab.to_owned());
          }
          oku_slides_session::Workspace::usage_owner(&self.1, tab)
      }
  ```

- [ ] **Step 8: Run the tests** — `cargo nextest run -p oku-slides-session` (background), then
`cargo nextest run -p oku-slides-mcp -p oku-slides-agent --no-run`, `cargo check -p oku-slides-app` and
`cargo clippy -p oku-slides-session -p oku-slides-mcp -p oku-slides-agent -p oku-slides-app --all-targets -- -D warnings`.
Expected: PASS, no warning (no unused import: `deck_usage_tests` imports only `fresh_dir` and `session` here).

- [ ] **Step 9: Docs** (zero net lines in `ARCHITECTURE.md`, at its 50-line cap)
  - Line 8, replace `file (`deck_file.rs`, `files.rs`, `merge.rs`).` with
    `file (`deck_file.rs`, `files.rs`, `merge.rs`), agent usage (`deck_usage.rs`: never dirty, routed by `workspace_usage.rs`).`
  - Line 40, replace `resume, same deck, agent, help,` with `resume, same deck, agent, usage (`usage_owner`), help,`.
  - Line 45, replace `` `deck/` holds `deck.loro`, `document.json`, `` with `` `deck/` holds `deck.loro`, `document.json` (`usage_unsaved` too), `usage.jsonl` (written on change), ``.
  - `crates/oku-slides-session/AGENTS.md` is at the lint's 60-line cap: extend line 14 (the resume-write rule)
    instead of adding one. Replace its ending `an unknown state version is unreadable: log, start blank. (spec:2026-09-26-resume-state-design)` with
    `an unknown state version is unreadable: log, start blank; record agent usage only in a deck tab's own document (the stash in history), never setting dirty, and snapshot it at once. (spec:2026-09-26-resume-state-design, spec:2026-10-30-deck-usage-ledger-design)`
  - Run `just agents-lint`. Expected: no finding.

- [ ] **Step 10: Commit** — message file in the scratchpad, `git add crates/oku-slides-session crates/oku-slides-mcp crates/oku-slides-agent/tests src-tauri/src/state.rs`
as its own command, index checked (`git diff --cached --stat`), then `git commit -F <file>`:
`feat(session)!: record agent usage in the owning deck, kept by the resume (t-1300, t-1296)`
(`!`: `Sessions` gains a required method; its body says so in a `BREAKING CHANGE:` footer).

### Task 5: Save the ledger on close and the `store_in_deck` setting (platform, session, docs)

Two user decisions (spec, "Capture" and "Setting"): a clean deck tab closed with records not yet in its
file writes the file alone, silently, only when the file is unchanged on disk since its last save; and
`[usage] store_in_deck` (default `true`) turns the whole feature off: nothing is recorded, and a save
writes no `usage.jsonl` (records read from a file someone else wrote are dropped from what this app
saves). The silent write never renders: the deck is clean, so the PDF on disk already shows it; only its
sources are embedded again (`oku_slides_pdf::embed_sources(pdf: &[u8], zip: &[u8]) -> Result<Vec<u8>, PdfError>`
replaces the previous blob and prunes it), into the very bytes whose hash is the deck's `saved_hash`.

**Files:**
- Modify: `crates/oku-slides-platform/src/config.rs` (`Settings` :37-71, new `UsageSettings` after `AuthorSettings` :340-344), `crates/oku-slides-platform/src/config.default.toml` (after `[author]` :151-154), `crates/oku-slides-platform/src/config_tests.rs`, `crates/oku-slides-platform/ARCHITECTURE.md` (line 15)
- Modify: `crates/oku-slides-session/src/lib.rs` (:454), `crates/oku-slides-session/src/deck_file.rs` (`write_file` :184-196), `crates/oku-slides-session/src/deck_usage.rs`
- Modify: `crates/oku-slides-session/src/workspace_usage.rs`, `crates/oku-slides-session/src/workspace_deck_packs_close.rs` (`close_tab` :209-218), `crates/oku-slides-session/src/workspace_windows.rs` (`close_window` :37)
- Test: `crates/oku-slides-session/src/deck_usage_tests.rs`, `crates/oku-slides-session/src/workspace_usage_tests.rs`
- Docs: `docs/aide/en.json`, `docs/aide/fr.json` (slides `s-fichiers`, `s-env-interface`, `s-format-zip`),
  `crates/oku-slides-core/AGENTS.md` (the add-only rule Task 2 extended), `crates/oku-slides-session/ARCHITECTURE.md` (line 8)

**Interfaces:**
- Consumes: Task 4 (`Document.usage_unsaved`, `DocDefaults.store_usage`, `Workspace::usage_owner`), Task 2 (`SourceBundle::to_zip_with(usage: bool)`).
- Produces: `oku_slides_platform::config::UsageSettings { store_in_deck: bool }`, `Settings.usage`;
  `Session::save_pending_usage(&mut self)` (crate-private); `Session::write_pdf(&self, path: &Path, pdf: &[u8]) -> Result<String>`
  (crate-private); `Workspace::save_pending_usages(ws: &Mutex<Workspace>, tabs: &[String])` (`pub(super)`).

- [ ] **Step 1: Write the failing platform tests** (`config_tests.rs`, after `default_settings_have_no_author_name`)

```rust
#[test]
fn defaults_usage_is_stored_in_the_deck() {
    assert_that!(Settings::defaults().unwrap().usage.store_in_deck).is_true();
}

#[test]
fn env_usage_store_in_deck_false_turns_it_off() {
    let s = Settings::from_sources(None, env(&[("OKU_SLIDES_USAGE__STORE_IN_DECK", "false")])).unwrap();
    assert_that!(s.usage.store_in_deck).is_false();
}
```

- [ ] **Step 2: Write the failing session tests** (`deck_usage_tests.rs`; its import becomes
`use crate::test_support::{fresh_dir, rename_title, session, session_with, settings_on, touch_elsewhere};`)

```rust
/// A session whose `[usage] store_in_deck` is `store`, on the roadmap fixture.
fn storing(store: bool) -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-usage-setting-");
    let mut settings = settings_on(data.path());
    settings.usage.store_in_deck = store;
    let mut s = session_with(settings);
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap_or_else(|e| panic!("fixture: {}", e.one_line()));
    (data, s)
}

#[test]
fn record_usage_with_store_in_deck_off_records_nothing() {
    let (_data, mut s) = storing(false);
    assert_that!(s.record_usage(record(1))).is_false();
    assert_that!(s.usage_ledger().map(UsageLedger::len)).is_equal_to(Some(0));
    assert_that!(s.doc.usage_unsaved).is_false();
}

#[test]
fn save_with_store_in_deck_off_writes_no_usage_entry() {
    let mut on = session();
    on.record_usage(record(1));
    let (_folder, path) = saved(&mut on, "shared");
    let (_data, mut off) = storing(false);
    off.open(&path).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    let (_copy, copy) = saved(&mut off, "copy");
    assert_that!(ledger_in(&copy).is_empty()).is_true();
}

#[test]
fn save_pending_usage_clean_deck_writes_its_file() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "pending");
    s.record_usage(record(1));
    s.save_pending_usage();
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
    assert_that!(s.doc.usage_unsaved).is_false();
    assert_that!(s.doc.saved_hash.clone()).is_equal_to(Some(oku_slides_core::asset_id(&std::fs::read(&path).unwrap())));
}

#[test]
fn save_pending_usage_dirty_deck_writes_nothing() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "dirty");
    s.record_usage(record(1));
    s.apply("x", vec![rename_title("Changé")], None).unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
    let before = std::fs::read(&path).unwrap();
    s.save_pending_usage();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
    assert_that!(s.doc.usage_unsaved).is_true();
}

#[test]
fn save_pending_usage_file_changed_on_disk_writes_nothing() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "changed");
    s.record_usage(record(1));
    touch_elsewhere(&path);
    let before = std::fs::read(&path).unwrap();
    s.save_pending_usage();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
    assert_that!(s.doc.usage_unsaved).is_true();
}

#[test]
fn save_pending_usage_deleted_file_is_not_recreated() {
    let mut s = session();
    let (_folder, path) = saved(&mut s, "deleted");
    s.record_usage(record(1));
    std::fs::remove_file(&path).unwrap();
    s.save_pending_usage();
    assert_that!(path.exists()).is_false();
}
```

Close tests, in `workspace_usage_tests.rs` (`deck_packs` leaves `t-1` saved and clean); its imports gain
`use crate::test_support::{rename_title, touch_elsewhere};`:

```rust
/// The file of `t-1`.
fn deck_path(f: &Packs) -> std::path::PathBuf {
    f.with("t-1", |s| s.doc.path.clone()).unwrap_or_else(|| panic!("a file"))
}

#[test]
fn close_clean_deck_with_unsaved_usage_writes_the_file() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    f.close_keeping("t-1").unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
}

#[test]
fn close_with_file_changed_on_disk_leaves_the_file_alone() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    touch_elsewhere(&path);
    let before = std::fs::read(&path).unwrap();
    f.close_keeping("t-1").unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
}

#[test]
fn close_dirty_deck_is_refused_and_writes_nothing() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    f.with("t-1", |s| s.apply("x", vec![rename_title("Changé")], None))
        .unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
    let before = std::fs::read(&path).unwrap();
    assert_that!(f.close_keeping("t-1")).is_err();
    assert_that!(std::fs::read(&path).unwrap()).is_equal_to(before);
}

#[test]
fn close_discarding_writes_no_usage() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    tab_session(&f, "t-1").record_usage(record(1));
    f.close("t-1");
    assert_that!(ledger_in(&path).is_empty()).is_true();
}

#[test]
fn close_window_with_clean_deck_writes_its_usage() {
    let (f, _pin) = deck_packs();
    let path = deck_path(&f);
    let other = f.new_tab();
    f.send(json!({"cmd": "move_tab", "tab": other}));
    tab_session(&f, "t-1").record_usage(record(1));
    f.send(json!({"cmd": "close_window", "window": "w-1"}));
    assert_that!(ledger_in(&path).len()).is_equal_to(1);
}
```

(`Packs::close` discards, `Packs::close_keeping` keeps, both in `workspace_packs_fixtures.rs`; `move_tab`
without a window moves the tab to a new window, as `to_new_window` does in `workspace_deck_packs_close_tests.rs`,
so `w-1` is not the only window.) Also import `ledger_in` next to `record`:
`use crate::deck_usage::deck_usage_tests::{ledger_in, record};`.

- [ ] **Step 3: Run to fail** — `cargo nextest run -p oku-slides-platform usage` and
`cargo nextest run -p oku-slides-session usage`. Expected: compile errors (`usage`, `save_pending_usage`),
and the `close_clean_*` and `close_window_*` tests failing (the file keeps no usage).

- [ ] **Step 4: The setting** (`oku-slides-platform`)

In `Settings` (after `author`):

```rust
    /// Agent token usage kept in each deck.
    pub usage: UsageSettings,
```

After `impl AuthorSettings`:

```rust
/// Agent token usage, recorded per turn in each deck file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct UsageSettings {
    /// Records each turn's tokens, model and cost in its deck; off: none recorded nor saved.
    pub store_in_deck: bool,
}
```

`config.default.toml`, after the `[author]` block:

```toml
[usage]
# Each agent turn's tokens, model and cost are kept in its deck file (`usage.jsonl`), so a shared
# deck shows them too. Off: nothing is recorded, and a saved deck carries no usage.
store_in_deck = true
```

`ARCHITECTURE.md` line 15: replace `` `SttSettings`, …) `` with `` `SttSettings`, `UsageSettings`, …) `` (zero net lines).

- [ ] **Step 5: Wire the setting into the session**

`lib.rs:454`:

```rust
        let doc_defaults = DocDefaults {
            undo_limit: settings.history.limit,
            author: settings.author.resolved(),
            store_usage: settings.usage.store_in_deck,
        };
```

`deck_file.rs`: `write_file` keeps the render and hands its PDF to a new `write_pdf`, which the silent
save also uses:

```rust
    /// Écrit le document (PDF et sources) dans `path` ; son empreinte.
    pub(crate) fn write_file(&mut self, path: &Path) -> Result<String> {
        let pdf = self.with_compiled(RemoteCompiled::pdf)?;
        self.write_pdf(path, &pdf)
    }

    /// Writes `pdf` with the document's sources embedded to `path`; its hash.
    ///
    /// `pdf` is a fresh render, or the file's own bytes when only the
    /// sources changed. `[usage] store_in_deck` off writes no ledger.
    pub(crate) fn write_pdf(&self, path: &Path, pdf: &[u8]) -> Result<String> {
        let zip = self.doc.sources.to_zip_with(self.doc_defaults.store_usage).context(SourcesSnafu)?;
        let out = oku_slides_pdf::embed_sources(pdf, &zip).context(PdfSnafu)?;
        let bytes = out.len();
        let hash = oku_slides_core::asset_id(&out);
        self.storage.write_atomic(path, out).context(WriteSnafu { path })?;
        tracing::info!(path = %path.display(), bytes, "{}", self.localizer.tr("log-document-saved"));
        Ok(hash)
    }
```

(every session save, copy and move goes through `write_file`; `refuse_linked_merge`'s trial copy keeps
`to_zip()`: it is never written.)

- [ ] **Step 6: The silent save on close** — in `deck_usage.rs` add
`use oku_slides_platform::storage::StorageErrorKind;` and, inside `impl Session`:

```rust
    /// Re-embeds the sources of a clean deck whose file lacks its agent usage; for a closing tab.
    ///
    /// No render: the deck is clean, so the file on disk already shows it.
    /// Only in deck mode, for a file still there whose bytes are those of the
    /// last save; otherwise the records stay unsaved, logged, and an unsaved
    /// deck is left to the close's own question.
    pub(crate) fn save_pending_usage(&mut self) {
        if !matches!(self.mode, Mode::Deck) || self.doc.dirty || !self.doc.usage_unsaved {
            return;
        }
        let (Some(path), Some(saved)) = (self.doc.path.clone(), self.doc.saved_hash.clone()) else { return };
        let pdf = match self.storage.read(&path) {
            Ok(bytes) => bytes,
            // Deleted: not written again behind the user's back.
            Err(e) if e.kind() == StorageErrorKind::NotFound => return,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %oku_slides_platform::errors::one_line(&e),
                    "agent usage not saved: the file is unreadable"
                );
                return;
            }
        };
        if oku_slides_core::asset_id(&pdf) != saved {
            tracing::warn!(path = %path.display(), "agent usage not saved: the file changed on disk");
            return;
        }
        match self.write_pdf(&path, &pdf) {
            Ok(hash) => {
                self.doc.saved_hash = Some(hash);
                self.doc.usage_unsaved = false;
                // A close refused later keeps a resume that matches the file.
                self.persist(true);
            }
            Err(e) => tracing::warn!(error = %e.one_line(), "agent usage not saved"),
        }
    }
```

(The log is a warning, as the other resume and save fallbacks of this crate; the spec says "logged".)

In `workspace_usage.rs` (add `use crate::Session;`):

```rust
    /// Saves each awake clean deck of `tabs` that holds agent usage its file lacks.
    ///
    /// The close's pre-phase, lock released; a failure is logged, never refuses the close.
    pub(super) fn save_pending_usages(ws: &Mutex<Self>, tabs: &[String]) {
        for tab in tabs {
            if let Err(e) = Self::with_quiet(ws, tab, Session::save_pending_usage).0 {
                tracing::warn!(tab = %tab, error = %e.one_line(), "agent usage not saved on close");
            }
        }
    }
```

(`Workspace::with_quiet(ws, tab, f) -> (Result<R>, bool)`, `workspace_deck_packs.rs:114`.
`ensure_closable` already needs every closing tab awake, so this wakes none the close would not. A dirty
deck is skipped by `save_pending_usage` itself, so a close the dirty check then refuses wrote nothing.)

`workspace_deck_packs_close.rs`, `close_tab`'s `(None, false)` arm:

```rust
            (None, false) => {
                Self::flush_linked(ws, tab);
                Self::save_pending_usages(ws, &[tab.to_owned()]);
                false
            }
```

`workspace_windows.rs`, `close_window`, right after `Self::flush_closing(ws, &closing, &discarded);`:

```rust
        let kept: Vec<String> = closing.iter().filter(|t| !discarded.contains(t)).cloned().collect();
        Self::save_pending_usages(ws, &kept);
```

A quit writes nothing: the resume keeps the records and `usage_unsaved`, and the next close or save
writes them.

The CLI needs no change: `save` and `import` build their bundle from a JSON deck or a PPTX (empty
ledger), and `render` / `open` never write a deck back; `serve` saves through the session, which
reads the setting above.

- [ ] **Step 7: Help** (`docs/aide/en.json`, `docs/aide/fr.json`). Their indentation is not uniform, so a
JSON round trip would rewrite the whole file: insert text at exact anchors instead. Run, from the repo root:

```bash
python3 -I - <<'PY'
TEXTS = {
    "en": ("Help", "Agent usage",
           ": each turn's tokens, model and cost go into the deck file; ask the agent what the deck cost. OKU_SLIDES_USAGE__STORE_IN_DECK=false keeps none.",
           ": the name written in the history of decks you edit.",
           ": false records no agent usage in decks, and saves none.",
           "one file per pinned theme version (the theme and its extends closure)",
           "agent usage, one turn per line (absent when empty)"),
    "fr": ("Aide", "Usage de l'agent",
           " : les tokens, le modèle et le coût de chaque tour vont dans le fichier du deck ; demandez à l'agent ce qu'a coûté le deck. OKU_SLIDES_USAGE__STORE_IN_DECK=false n'en garde aucun.",
           " : le nom inscrit dans l'historique des decks que vous modifiez.",
           " : false n'inscrit aucun usage de l'agent dans les decks, et n'en enregistre aucun.",
           "une version de thème épinglée (le thème et la fermeture de ses extends)",
           "usage de l'agent, un tour par ligne (absent s'il est vide)"),
}

def bullet(bold, rest):
    return ('        {\n         "runs": [\n          {\n           "t": "%s",\n           "bold": true\n'
            '          },\n          {\n           "t": "%s"\n          }\n         ],\n         "list": "bullet"\n'
            '        },\n') % (bold, rest)

def once(text, anchor, new):
    assert text.count(anchor) == 1, anchor
    return text.replace(anchor, new)

for lang, (help_word, title, body, author, env, themes, usage) in TEXTS.items():
    path = f"docs/aide/{lang}.json"
    text = open(path, encoding="utf-8").read()
    # s-fichiers: a bullet before the Help one.
    help_bullet = '        {\n         "runs": [\n          {\n           "t": "%s",\n' % help_word
    text = once(text, help_bullet, bullet(title, body) + help_bullet)
    # s-env-interface: a bullet after OKU_SLIDES_AUTHOR__NAME's.
    author_end = '           "t": "%s"\n          }\n         ],\n         "list": "bullet"\n        },\n' % author
    text = once(text, author_end, author_end + bullet("OKU_SLIDES_USAGE__STORE_IN_DECK", env))
    # s-format-zip: a line after the themes one.
    themes_line = '<hex>/theme.json         %s"\n          }\n         ]\n        }\n' % themes
    usage_line = ('        {\n         "runs": [\n          {\n           "t": "usage.jsonl                     %s"\n'
                  '          }\n         ]\n        }\n') % usage
    text = once(text, themes_line, themes_line[:-1] + ",\n" + usage_line)
    open(path, "w", encoding="utf-8").write(text)
PY
```

Check the result: `python3 -c "import json; [json.load(open(f'docs/aide/{l}.json')) for l in ('en','fr')]"`
(valid JSON), `git diff --stat docs/aide` (only additions, about 36 lines per file). Check the help still
renders: `cargo nextest run -p oku-slides-session help` (the help tests parse, validate and render every
shipped language). If `s-fichiers` or `s-env-interface` now overflows its box, shorten the new bullet,
never drop an older one.

- [ ] **Step 8: Run** — `cargo nextest run -p oku-slides-platform -p oku-slides-session` (background),
`cargo check -p oku-slides-app`, `cargo clippy -p oku-slides-platform -p oku-slides-session --all-targets -- -D warnings`.
Expected: PASS, the `close_*` tests included.

- [ ] **Step 9: Docs**
  - `crates/oku-slides-platform/AGENTS.md` needs no rule (the setting follows the existing "embedded
    default" decision).
  - `crates/oku-slides-core/AGENTS.md`: the add-only rule Task 2 wrote gains its one exception. Replace
    `- Keep bundle assets, fonts, packs and the usage ledger add-only: merge by union, never remove one. (spec:2026-09-27-sources-loro-design, plan:2026-10-01-packs-portables, spec:2026-10-30-deck-usage-ledger-design)` with
    `- Keep bundle assets, fonts, packs and the usage ledger add-only: merge by union, never remove one; only a save with [usage] store_in_deck off drops the ledger. (spec:2026-09-27-sources-loro-design, plan:2026-10-01-packs-portables, spec:2026-10-30-deck-usage-ledger-design)`
  - `crates/oku-slides-session/ARCHITECTURE.md` line 8: replace
    `(`deck_usage.rs`: never dirty, routed by `workspace_usage.rs`)` (Task 4's text) with
    `(`deck_usage.rs`: never dirty, re-embedded without render when a clean tab closes, routed by `workspace_usage.rs`; `[usage] store_in_deck`)`
    (zero net lines).
  - Run `just agents-lint`. Expected: no finding.

- [ ] **Step 10: Commit** — `git add crates/oku-slides-platform crates/oku-slides-session crates/oku-slides-core/AGENTS.md docs/aide`
as its own command, index checked, `git commit -F <file>`:
`feat(session,platform): save agent usage when a clean tab closes; [usage] store_in_deck (t-1306, t-1296)`.

### Task 6: Capture usage at the end of a turn (agent)

**Files:**
- Modify: `crates/oku-slides-agent/Cargo.toml` (`agent-client-protocol` gains the feature `unstable_end_turn_token_usage`; `uuid` from `[dev-dependencies]` to `[dependencies]`, add `time`). The root `Cargo.toml` is not touched.
- Create: `crates/oku-slides-agent/src/usage_board.rs`, `crates/oku-slides-agent/src/usage_board_tests.rs`
- Modify: `crates/oku-slides-agent/src/lib.rs` (`mod usage_board;`, alphabetical, after `turns`)
- Modify: `crates/oku-slides-agent/src/link.rs` (`Current` :126, `Inner` :135, `Link::new`/`with_model` :164-182, `switch` :236, `forget` :366, `launch` :386, `on_notification` :417-447, `Serving` :449-477, end of `run` :567-570), `link_tests.rs` (:56)
- Modify: `crates/oku-slides-agent/src/stack.rs` (:202), `choice.rs` (`switch_to` :275)
- Modify: `crates/oku-slides-agent/src/turn.rs` (imports, doc :27-29, return type :49, end :125-129)
- Modify: `crates/oku-slides-agent/src/tab_agent.rs` (`CLOSE_GRACE` doc :31-34, `prompt` :319, `reopen` :425); it stays at 500 lines
- Modify: `crates/oku-slides-agent/src/bin/fake_agent.rs` (doc header :33-36, imports :43-49, `Turn` :155-177, `play` :523, prompt responder :573)
- Modify: `crates/oku-slides-agent/tests/common/mod.rs` (:133), `tests/launch.rs` (:147, :166, :188, :211, :227)
- Create: `crates/oku-slides-agent/tests/usage.rs`
- Docs: `crates/oku-slides-agent/ARCHITECTURE.md` (three lines extended, zero net lines)

**Interfaces:**
- Consumes: `UsageRecord`, `UsageTokens`, `UsageCost`, `UsageTab`, `UsageRecord::sanitized`, `UsageCost::is_valid` (Task 1);
  `TabSession::record_usage(&self, UsageRecord)`, `Session::usage_ledger(&self) -> Option<&UsageLedger>` (Task 4).
- Produces (all `pub(crate)` unless said): `Link::for_preset(preset: &str, launch: Launch, model: Option<String>) -> Link` (pub, replaces
  `Link::with_model`); `Link::switch(&self, preset: &str, launch: Launch, model: Option<String>)` (pub, gains `preset`);
  `Link::preset(&self) -> Option<String>`; `Link::usage(&self) -> &UsageBoard`;
  `UsageBoard::{told(&self, generation: u64, id: &SessionId, cost: Option<&Cost>), resumed(&self, generation: u64, id: &SessionId),
  take_turn_cost(&self, generation: u64, id: &SessionId) -> Option<UsageCost>, forget(&self, generation: u64, id: &SessionId),
  forget_generation(&self, generation: u64)}`; `turn_cost(base: Option<&UsageCost>, now: &UsageCost) -> UsageCost`;
  `tokens_of(u: &Usage) -> UsageTokens`; `usage_tab(kind: DocumentKind) -> Option<UsageTab>`;
  `tells(replaying: Option<bool>, session: &SessionId, what: &'static str) -> bool` (the session-state guard,
  moved from `link.rs` to `usage_board.rs`, used by `on_notification` for options and cost);
  `async fn record_turn(ctx: &Ctx, link: &Link, generation: u64, id: &SessionId, usage: Option<&Usage>)`;
  `turn::run` returns `agent_client_protocol::Result<(Pending, Option<Usage>)>`.
- Produces for Task 9 — the fake agent's scenario syntax (`fixtures/agent/*.json`, any `Turn`, so also a
  `by_prompt` entry): `"usage": [total, input, output]` (three integers, sent in that turn's prompt reply) and
  `"cost": { "amount": <f64>, "currency": "<ISO 4217>" }` (sent **verbatim** as the session's **cumulative**
  `usage_update.cost` after the turn's chunks; the fake never adds anything up). The app records a turn's share,
  the growth since the session's previous cost, so a scenario that wants two turns of 0.1 and 0.3321 USD scripts
  cumulative amounts 0.1 then 0.4321. Example `by_prompt` entry:
  `"coût un": { "usage": [120, 100, 20], "cost": { "amount": 0.1, "currency": "USD" }, "chunks": ["Un."] }`.
  The same entry played twice sends the same total twice: its second share is 0.

Why the preset lives in `Link`'s `Current` and not in `Launch`: `Launch::for_preset` and
`AgentChoice::launch_of` receive an `AgentPreset`, never its id, and `Launch` has 18 literals in
tests. `Current` already changes launch, epoch and model together in `switch`; the preset id joins
them. A `Link::new` (tests, `oku-slides-local-agent` tests) has no preset: its turns record nothing.

Why a resumed session's first cost is `None`: ACP's `UsageUpdate.cost` is the session's cumulative
cost. After `session/load` or `session/resume` the first one seen covers every turn before the
restart, already recorded; charging it to one turn would count them twice. It becomes the base only.

- [ ] **Step 1: Enable the feature and move the dependencies**

Only the agent crate reads `PromptResponse.usage`, so the feature is enabled there, not in
`workspace.dependencies`. In `crates/oku-slides-agent/Cargo.toml`, `[dependencies]`:

```toml
agent-client-protocol = { workspace = true, features = ["unstable_end_turn_token_usage"] }
```

Then delete `uuid.workspace = true` from `[dev-dependencies]`, and add to `[dependencies]`, after
`base64.workspace = true`:

```toml
time.workspace = true
uuid.workspace = true
```

(`uuid` has `v4` in `workspace.dependencies`; `OffsetDateTime::now_utc` needs no `time` feature.)
Run `cargo check -p oku-slides-agent -p oku-slides-local-agent --tests`: both build (cargo unifies
the feature for every crate of the build; they only call `PromptResponse::new`, never a struct
literal, and the types are `#[non_exhaustive]`).

- [ ] **Step 2: Write the failing unit tests** — `crates/oku-slides-agent/src/usage_board_tests.rs`

```rust
use agent_client_protocol::schema::v1::{Cost, SessionId, Usage};
use oku_slides_core::{UsageCost, UsageTab, UsageTokens};
use oku_slides_session::DocumentKind;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn c(amount: f64, currency: &str) -> UsageCost {
    UsageCost { amount, currency: currency.into() }
}

fn told(amount: f64, currency: &str) -> Cost {
    Cost::new(amount, currency)
}

/// Plays turns of one session: each is told its costs in order, then the turn's share is taken.
fn shares(resumed: bool, turns: &[Vec<Cost>]) -> Vec<Option<UsageCost>> {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    if resumed {
        board.resumed(1, &id);
    }
    turns
        .iter()
        .map(|seen| {
            for cost in seen {
                board.told(1, &id, Some(cost));
            }
            board.take_turn_cost(1, &id)
        })
        .collect()
}

#[rstest]
#[case::first_turn_is_whole(false, vec![vec![told(0.5, "USD")]], vec![Some(c(0.5, "USD"))])]
#[case::growing_total_gives_the_growth(
    false,
    vec![vec![told(0.5, "USD")], vec![told(0.75, "USD")]],
    vec![Some(c(0.5, "USD")), Some(c(0.25, "USD"))]
)]
#[case::last_update_of_the_turn_counts(false, vec![vec![told(0.25, "USD"), told(0.5, "USD")]], vec![Some(c(0.5, "USD"))])]
#[case::smaller_total_is_a_new_session(
    false,
    vec![vec![told(0.5, "USD")], vec![told(0.125, "USD")]],
    vec![Some(c(0.5, "USD")), Some(c(0.125, "USD"))]
)]
#[case::currency_change_restarts(
    false,
    vec![vec![told(0.5, "USD")], vec![told(0.25, "EUR")]],
    vec![Some(c(0.5, "USD")), Some(c(0.25, "EUR"))]
)]
#[case::turn_without_update_has_no_cost(false, vec![vec![told(0.5, "USD")], vec![]], vec![Some(c(0.5, "USD")), None])]
#[case::resumed_first_cost_is_only_the_base(
    true,
    vec![vec![told(2.0, "USD")], vec![told(2.5, "USD")]],
    vec![None, Some(c(0.5, "USD"))]
)]
#[case::resumed_without_cost_waits_for_one(
    true,
    vec![vec![], vec![told(2.0, "USD")], vec![told(2.5, "USD")]],
    vec![None, None, Some(c(0.5, "USD"))]
)]
#[case::not_finite_amount_is_ignored(false, vec![vec![told(f64::NAN, "USD")]], vec![None])]
#[case::negative_amount_is_ignored(false, vec![vec![told(-1.0, "USD")]], vec![None])]
#[case::lower_case_currency_is_ignored(false, vec![vec![told(1.0, "usd")]], vec![None])]
#[case::implausible_amount_is_ignored(false, vec![vec![told(2e9, "USD")]], vec![None])]
fn take_turn_cost_sequence_gives_each_turn_its_share(
    #[case] resumed: bool,
    #[case] turns: Vec<Vec<Cost>>,
    #[case] want: Vec<Option<UsageCost>>,
) {
    assert_that!(shares(resumed, &turns)).is_equal_to(want);
}

#[test]
fn told_without_cost_changes_nothing() {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    board.told(1, &id, None);
    assert_that!(board.take_turn_cost(1, &id)).is_none();
}

#[test]
fn forget_session_makes_its_next_cost_whole() {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    board.told(1, &id, Some(&told(0.5, "USD")));
    board.take_turn_cost(1, &id);
    board.forget(1, &id);
    board.told(1, &id, Some(&told(0.75, "USD")));
    assert_that!(board.take_turn_cost(1, &id)).is_equal_to(Some(c(0.75, "USD")));
}

#[test]
fn forget_generation_keeps_other_generations() {
    let board = UsageBoard::default();
    let id = SessionId::new("s");
    board.told(1, &id, Some(&told(0.5, "USD")));
    board.told(2, &id, Some(&told(0.25, "USD")));
    board.forget_generation(1);
    assert_that!(board.take_turn_cost(1, &id)).is_none();
    assert_that!(board.take_turn_cost(2, &id)).is_equal_to(Some(c(0.25, "USD")));
}

#[rstest]
#[case::base_absent(None, c(0.5, "USD"), c(0.5, "USD"))]
#[case::growth(Some(c(0.5, "USD")), c(0.75, "USD"), c(0.25, "USD"))]
#[case::equal_total_is_free(Some(c(0.5, "USD")), c(0.5, "USD"), c(0.0, "USD"))]
#[case::reset(Some(c(0.5, "USD")), c(0.125, "USD"), c(0.125, "USD"))]
#[case::other_currency(Some(c(0.5, "USD")), c(0.25, "EUR"), c(0.25, "EUR"))]
fn turn_cost_base_and_total_gives_share(#[case] base: Option<UsageCost>, #[case] now: UsageCost, #[case] want: UsageCost) {
    assert_that!(turn_cost(base.as_ref(), &now)).is_equal_to(want);
}

#[test]
fn tokens_of_reply_without_optional_counts_keeps_them_unknown() {
    let want = UsageTokens { total: 30, input: 20, output: 10, thought: None, cache_read: None, cache_write: None };
    assert_that!(tokens_of(&Usage::new(30, 20, 10))).is_equal_to(want);
}

#[test]
fn tokens_of_reply_with_optional_counts_keeps_them() {
    let usage = Usage::new(30, 20, 10).thought_tokens(4).cached_read_tokens(100).cached_write_tokens(7);
    let t = tokens_of(&usage);
    assert_that!((t.thought, t.cache_read, t.cache_write)).is_equal_to((Some(4), Some(100), Some(7)));
}

#[rstest]
#[case::deck(DocumentKind::Deck, Some(UsageTab::Deck))]
#[case::pack(DocumentKind::Pack, Some(UsageTab::Pack))]
#[case::theme(DocumentKind::Theme, Some(UsageTab::Theme))]
#[case::help(DocumentKind::Help, None)]
fn usage_tab_document_kind_gives_record_tab(#[case] kind: DocumentKind, #[case] want: Option<UsageTab>) {
    assert_that!(usage_tab(kind)).is_equal_to(want);
}

#[rstest]
#[case::routed_live(Some(false), true)]
#[case::routed_replaying(Some(true), false)]
#[case::unknown_session(None, false)]
fn tells_route_state_decides_whether_a_state_update_counts(#[case] replaying: Option<bool>, #[case] want: bool) {
    assert_that!(tells(replaying, &SessionId::new("s"), "cost")).is_equal_to(want);
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-agent usage_board` (in the background the first time).
Expected: compile errors, `UsageBoard`, `tells` not found.

- [ ] **Step 4: Implement `usage_board.rs`**

```rust
//! Each session's cumulative cost, turned into each turn's share, and the turn's record.
//!
//! ACP sends the session's cost so far in `usage_update`; a turn's share is the growth since the
//! previous turn. [`tells`] decides whether a session-state update counts (not replayed, from a
//! routed session). [`record_turn`] builds the [`UsageRecord`] when a reply arrives and files it
//! in the tab's deck through `TabSession::record_usage`.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use agent_client_protocol::schema::v1::{Cost, SessionId, Usage};
use oku_slides_core::{UsageCost, UsageRecord, UsageTab, UsageTokens};
use oku_slides_session::DocumentKind;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::link::Link;
use crate::tab_agent::Ctx;

/// What one session told about its cost.
#[derive(Debug, Default)]
struct Seen {
    /// The cumulative cost at the end of the previous turn that saw one.
    base: Option<UsageCost>,
    /// The latest cumulative cost, if one arrived since the previous turn ended.
    fresh: Option<UsageCost>,
    /// Reopened by `session/load` or `session/resume`: its first cost covers earlier turns.
    resumed: bool,
}

/// The cumulative cost each live session told, by launch and session.
#[derive(Debug, Default)]
pub(crate) struct UsageBoard(Mutex<HashMap<(u64, SessionId), Seen>>);

impl UsageBoard {
    /// Notes a `usage_update` of a routed session; no cost, or an implausible one, changes nothing.
    pub(crate) fn told(&self, generation: u64, id: &SessionId, cost: Option<&Cost>) {
        let Some(cost) = cost else { return };
        let cost = UsageCost { amount: cost.amount, currency: cost.currency.clone() };
        if !cost.is_valid() {
            tracing::debug!(session = ?id, "agent cost not finite, out of 0..=1e9 or without ISO code: ignored");
            return;
        }
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        map.entry((generation, id.clone())).or_default().fresh = Some(cost);
    }

    /// Session `id` was reopened: its first cost sets the base and charges no turn.
    pub(crate) fn resumed(&self, generation: u64, id: &SessionId) {
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        map.entry((generation, id.clone())).or_default().resumed = true;
    }

    /// The ending turn's cost; `None` when no cost arrived during it, or it is a resumed base.
    pub(crate) fn take_turn_cost(&self, generation: u64, id: &SessionId) -> Option<UsageCost> {
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let seen = map.get_mut(&(generation, id.clone()))?;
        let now = seen.fresh.take()?;
        let share = (seen.base.is_some() || !seen.resumed).then(|| turn_cost(seen.base.as_ref(), &now));
        seen.base = Some(now);
        share
    }

    /// Drops session `id` of launch `generation` (new conversation, tab closed).
    pub(crate) fn forget(&self, generation: u64, id: &SessionId) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).remove(&(generation, id.clone()));
    }

    /// Drops the sessions of an agent process that ended.
    pub(crate) fn forget_generation(&self, generation: u64) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).retain(|(g, _), _| *g != generation);
    }
}

/// Whether a session-state update counts: from a routed session, not replayed.
///
/// `replaying`: the route's replay flag, `None` without a route. A late replay of
/// `session/load` tells past state (t-612).
pub(crate) fn tells(replaying: Option<bool>, session: &SessionId, what: &'static str) -> bool {
    match replaying {
        Some(false) => true,
        Some(true) => {
            tracing::debug!(?session, what, "replayed session state: ignored");
            false
        }
        None => {
            tracing::debug!(?session, what, "state of an unknown session: ignored");
            false
        }
    }
}

/// A turn's share of a cumulative cost: the growth since `base`, or `now` whole after a reset.
///
/// A smaller total or another currency means the agent started counting again.
pub(crate) fn turn_cost(base: Option<&UsageCost>, now: &UsageCost) -> UsageCost {
    match base {
        Some(b) if b.currency == now.currency && now.amount >= b.amount => {
            UsageCost { amount: now.amount - b.amount, currency: now.currency.clone() }
        }
        _ => now.clone(),
    }
}

/// The record's token counts from ACP's end-of-turn usage.
pub(crate) fn tokens_of(u: &Usage) -> UsageTokens {
    UsageTokens {
        total: u.total_tokens,
        input: u.input_tokens,
        output: u.output_tokens,
        thought: u.thought_tokens,
        cache_read: u.cached_read_tokens,
        cache_write: u.cached_write_tokens,
    }
}

/// The record's tab for a tab of `kind`; a help tab records nothing.
pub(crate) fn usage_tab(kind: DocumentKind) -> Option<UsageTab> {
    match kind {
        DocumentKind::Deck => Some(UsageTab::Deck),
        DocumentKind::Pack => Some(UsageTab::Pack),
        DocumentKind::Theme => Some(UsageTab::Theme),
        DocumentKind::Help => None,
    }
}

/// Files the ended turn's usage in the tab's deck; it never fails the turn.
///
/// The turn's cost is taken even when nothing is recorded, so the next turn's share starts
/// here. Nothing is recorded for a help tab, nor by a `Link` without preset (tests).
pub(crate) async fn record_turn(ctx: &Ctx, link: &Link, generation: u64, id: &SessionId, usage: Option<&Usage>) {
    let cost = link.usage().take_turn_cost(generation, id);
    let Some(tab) = usage_tab(ctx.kind) else { return };
    let Some(agent) = link.preset() else {
        tracing::debug!(tab = %ctx.tab.tab(), "link without preset: usage not recorded");
        return;
    };
    let record = UsageRecord {
        turn: Uuid::new_v4(),
        at: OffsetDateTime::now_utc(),
        tab,
        agent,
        model: link.models().session(generation, id).map(|o| o.current),
        tokens: usage.map(tokens_of),
        cost,
        // Fields a newer app wrote; a record made here has none.
        extra: serde_json::Map::new(),
    };
    // The agent's values are untrusted: cut to the ledger's checks, never written unreadable.
    let Some(record) = record.sanitized() else {
        tracing::warn!(tab = %ctx.tab.tab(), "usage record without agent id: not recorded");
        return;
    };
    let session = ctx.tab.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || session.record_usage(record)).await {
        tracing::warn!(tab = %ctx.tab.tab(), error = %e, "usage not recorded: blocking task lost");
    }
}

#[cfg(test)]
#[path = "usage_board_tests.rs"]
mod usage_board_tests;
```

Add `mod usage_board;` to `lib.rs` after `mod turns;`.

- [ ] **Step 5: Keep the preset and the board in `Link`** (`link.rs`)

`use crate::usage_board::{UsageBoard, tells};` next to the `session_model` import.

`Current` gets the preset id, after `epoch`:

```rust
    /// The agent preset id (an `[agent.presets]` key); `None`: a link built for tests.
    preset: Option<String>,
```

`Inner` gets, after `models`:

```rust
    /// The cost the sessions told, by launch and session.
    usage: Arc<UsageBoard>,
```

Replace `Link::new` and `Link::with_model` (:164-182) with:

```rust
    /// A link that launches nothing before the first [`Link::connect`]; sessions keep the agent's model.
    ///
    /// Without a preset id, its turns record no usage (tests).
    #[must_use]
    pub fn new(launch: Launch) -> Self {
        Self::build(None, launch, None)
    }

    /// The link of preset `preset`; each session is asked to use `model` if the agent offers it.
    #[must_use]
    pub fn for_preset(preset: &str, launch: Launch, model: Option<String>) -> Self {
        Self::build(Some(preset.to_owned()), launch, model)
    }

    fn build(preset: Option<String>, launch: Launch, model: Option<String>) -> Self {
        Self {
            inner: Arc::new(Inner {
                launch: StdMutex::new(Current { launch, epoch: 0, preset, model }),
                current_stop: StdMutex::default(),
                failure: StdMutex::default(),
                state: Mutex::new(State::default()),
                routes: Routes::default(),
                models: Arc::default(),
                usage: Arc::default(),
                shutdown: CancellationToken::new(),
            }),
        }
    }
```

After `models()` (:216):

```rust
    /// The preset id of the current agent; `None`: a link built for tests.
    pub(crate) fn preset(&self) -> Option<String> {
        self.inner.launch.lock().unwrap_or_else(PoisonError::into_inner).preset.clone()
    }

    /// The cost told by the sessions, shared with the tab actors.
    pub(crate) fn usage(&self) -> &UsageBoard {
        &self.inner.usage
    }
```

`switch` (:236) takes the preset id with the launch, under the same lock:

```rust
    /// Switches agent: the next [`Link::connect`] launches `launch` of preset `preset`, using `model`.
    ///
    /// The live connection stops at once (its turns fail as on any stop); a launch in progress is
    /// interrupted, and `connect` relaunches right away with `launch`. Nothing launches after
    /// `shutdown`.
    pub fn switch(&self, preset: &str, launch: Launch, model: Option<String>) {
        {
            let mut current = self.inner.launch.lock().unwrap_or_else(PoisonError::into_inner);
            current.launch = launch;
            current.epoch += 1;
            current.preset = Some(preset.to_owned());
            current.model = model;
        }
```

(the rest of `switch` unchanged). `forget` (:366) forgets the session's cost too:

```rust
    pub(crate) fn forget(&self, generation: u64, id: SessionId) {
        self.inner.models.forget(generation, &id);
        self.inner.usage.forget(generation, &id);
        self.inner.routes.lock().unwrap_or_else(PoisonError::into_inner).remove(&(generation, id));
    }
```

`Serving` gets `usage: Arc<UsageBoard>,` after `models`; `launch` fills it with
`usage: self.inner.usage.clone(),`; `run` destructures it
(`let Self { routes, models, usage, stop, closed, generation, epoch } = self;`), clones it into the
notification closure and passes it on:

```rust
                {
                    let (routes, models, usage) = (routes.clone(), models.clone(), usage.clone());
                    async move |n: SessionNotification, _cx: ConnectionTo<Agent>| {
                        on_notification(&routes, &models, &usage, generation, epoch, n);
                        Ok(())
                    }
                },
```

and the end of `run` forgets the launch's costs with its models:

```rust
        models.forget_generation(generation);
        usage.forget_generation(generation);
```

`on_notification` (:417-447): both session-state updates go through one guard, so a cost replayed
by `session/load` or told for an unknown session is ignored exactly like config options (t-612):

```rust
/// Files a notification in the open turn of its session.
///
/// `n` comes from connection `generation` of launch epoch `epoch`. The agent sends a turn's
/// updates before returning its response, on the same connection; outside a turn, or replayed
/// after `session/load`, it is ignored. A routed session's new config options are told to
/// `models`, and its cost to `usage`, in or out of a turn, unless replayed.
fn on_notification(
    routes: &Routes,
    models: &ModelBoard,
    usage: &UsageBoard,
    generation: u64,
    epoch: u64,
    n: SessionNotification,
) {
    tracing::trace!(?n, "acp");
    let route = routes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&(generation, n.session_id.clone()))
        .map(|r| (r.events.clone(), r.hub.clone(), r.replaying.load(Ordering::Acquire)));
    let replaying = route.as_ref().map(|r| r.2);
    match &n.update {
        SessionUpdate::ConfigOptionUpdate(update) => {
            if tells(replaying, &n.session_id, "options") {
                models.told(generation, epoch, &n.session_id, model_options(&update.config_options));
            }
            return;
        }
        SessionUpdate::UsageUpdate(update) => {
            if tells(replaying, &n.session_id, "cost") {
                usage.told(generation, &n.session_id, update.cost.as_ref());
            }
            return;
        }
        _ => {}
    }
    match route {
        // … the existing three arms, unchanged …
    }
}
```

(The `match route { … }` that follows keeps its three arms as they are; only the old
`if let SessionUpdate::ConfigOptionUpdate` block is replaced. `tells` lives in `usage_board.rs`, so
`link.rs`, already over 500 lines, grows only by the board's plumbing.)

- [ ] **Step 6: Pass the preset id at both launch points**

`stack.rs` :202 — the id is `selected()`, the key of the `active()` preset read just above:

```rust
        let link = Link::for_preset(
            platform.settings.agent.selected(),
            AgentChoice::launch_of(a, platform),
            model,
        );
```

`choice.rs` `switch_to` :275:

```rust
        self.link.switch(id, Self::launch_of(preset, &self.platform), model);
```

Tests: `tests/launch.rs` :147, :166, :188, :211, :227 become `h.link.switch("fake", launch, None);`,
`link_tests.rs` :56 `link.switch("autre", launch("/nonexistent/oku-slides-agent-autre"), None);`.
`tests/common/mod.rs` :133 becomes `let link = Link::for_preset("fake", launch, None);` so the
harness's turns record usage. `cargo check -p oku-slides-agent --tests` builds.

- [ ] **Step 7: Return the reply's usage from `turn::run`** (`turn.rs`)

Import `Usage` (`use agent_client_protocol::schema::v1::{CancelNotification, ContentBlock, PromptRequest, SessionId, TextContent, Usage};`),
change the doc's second paragraph to "Returns what happened during the turn (handled next) and the
reply's token usage, `None` when the agent sent none.", the return type to
`agent_client_protocol::Result<(Pending, Option<Usage>)>`, and the end to:

```rust
    ctx.end_turn().await;
    let response = response?;
    ctx.emit(AgentEvent::TurnEnded { turn, stop: as_text(&response.stop_reason) });
    ctx.emit(AgentEvent::idle());
    Ok((pending, response.usage))
}
```

A cancelled turn has a reply (`StopReason::Cancelled`) and is recorded; a turn whose request fails
has none and records nothing.

- [ ] **Step 8: Record in `TabAgent`** (`tab_agent.rs`)

In `prompt` (:319), the `Ok` arm of the `match crate::turn::run(…)` becomes (one line added; the
`Err` arm is unchanged):

```rust
            Ok((pending, usage)) => {
                crate::usage_board::record_turn(&self.ctx, &self.link, live.generation, &id, usage.as_ref()).await;
                if pending.new_conversation {
                    self.new_conversation().await;
                }
                pending.prompt.map(Command::Prompt)
            }
```

It runs before `new_conversation`, which forgets the session (and its cost). In `reopen`, the
`Ok(options)` arm marks the reopened session after `adopt`:

```rust
            Ok(options) => {
                // Un rejeu tardif (agent non conforme) reste hors du tour jusqu'au premier prompt (t-612).
                self.adopt(live, turn, &id, !live.caps.resume, options.as_deref());
                self.link.usage().resumed(live.generation, &id);
                Ok(Reopened::Resumed(id))
            }
```

Those two lines would take `tab_agent.rs` from 500 to 502. The `CLOSE_GRACE` doc (:31-34), French
and four lines, is translated in two, so the file stays at 500:

```rust
/// Wait for the connection's end after a failed request: ACP fails pending requests before
/// signalling the end, and the next prompt must relaunch the agent, not reuse the old connection.
```

Everything else lives in `usage_board.rs`. Check: `wc -l crates/oku-slides-agent/src/tab_agent.rs` → 500.

- [ ] **Step 9: Script usage in the fake agent** (`bin/fake_agent.rs`)

Imports: add `Cost`, `Usage`, `UsageUpdate` to the `agent_client_protocol::schema::v1` list.
`Turn` gets, after `parallel`:

```rust
    /// End-of-turn usage `[total, input, output]`, sent in the prompt reply.
    #[serde(default)]
    usage: Option<[u64; 3]>,
    /// The session's cumulative cost, sent as `usage_update` after the chunks.
    #[serde(default)]
    cost: Option<FakeCost>,
```

with, after `Turn`:

```rust
/// A cumulative session cost of the scenario.
#[derive(Debug, Clone, Deserialize)]
struct FakeCost {
    amount: f64,
    currency: String,
}
```

In `play`, after the `for text in &turn.chunks` loop and before `if turn.wait_cancel`:

```rust
    if let Some(c) = &turn.cost {
        let update = UsageUpdate::new(0, 200_000).cost(Cost::new(c.amount, c.currency.clone()));
        cx.send_notification(SessionNotification::new(session.clone(), SessionUpdate::UsageUpdate(update)))?;
    }
```

The prompt responder (:573) sends the usage:

```rust
                        let usage = turn.usage.map(|[total, input, output]| Usage::new(total, input, output));
                        responder.respond(PromptResponse::new(stop).usage(usage))
```

Doc header, after the `no_options_on_reopen` line (:36):

```rust
//! A turn's `usage` (`[total, input, output]`) goes in its prompt reply; its `cost`
//! (`{ "amount", "currency" }`) is sent as is, as the session's cumulative cost, in a
//! `usage_update` after its chunks: the scenario writes the running total, never a turn's share.
```

- [ ] **Step 10: Write the fake-agent tests** — `crates/oku-slides-agent/tests/usage.rs`

```rust
//! Each turn's usage lands in the tab's deck: tokens, model, preset and cost share.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::PoisonError;
use std::time::Duration;

use common::*;
use oku_slides_agent::AgentEvent;
use oku_slides_core::{UsageCost, UsageRecord, UsageTab, UsageTokens};
use oku_slides_session::agent::AgentUi;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// The deck's records once it holds `n` (recording follows the turn's end; 5 s at most).
async fn records(h: &Harness, n: usize) -> Vec<UsageRecord> {
    for _ in 0..250 {
        let got: Vec<UsageRecord> = h
            .session("t-1")
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .usage_ledger()
            .map(|l| l.records().cloned().collect())
            .unwrap_or_default();
        if got.len() >= n {
            return got;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("{n} usage records expected");
}

/// Plays the scenario's turns on `t-1`, one prompt each, then returns the deck's records.
async fn played(script: Value, prompts: usize) -> Vec<UsageRecord> {
    let h = start(script).await;
    play_turns(&h, prompts).await;
    records(&h, prompts).await
}

async fn play_turns(h: &Harness, prompts: usize) {
    let t = h.tab("t-1");
    for i in 0..prompts {
        t.agent.prompt(format!("consigne {i}"), AgentUi::default());
        wait_turn_ended(t).await;
    }
}

fn usd(amount: f64) -> Option<UsageCost> {
    Some(UsageCost { amount, currency: "USD".into() })
}

#[tokio::test(flavor = "multi_thread")]
async fn turn_with_usage_and_cost_records_them_with_preset_and_model() {
    let script = json!({
        "models": [{ "id": "opus-test", "name": "Opus" }],
        "turns": [{ "usage": [30, 20, 10], "cost": { "amount": 0.5, "currency": "USD" } }],
    });
    let r = played(script, 1).await;
    assert_that!(r).has_length(1);
    assert_that!(r[0].tab).is_equal_to(UsageTab::Deck);
    assert_that!(r[0].agent.as_str()).is_equal_to("fake");
    assert_that!(r[0].model.as_deref()).is_equal_to(Some("opus-test"));
    let tokens = UsageTokens { total: 30, input: 20, output: 10, thought: None, cache_read: None, cache_write: None };
    assert_that!(r[0].tokens).is_equal_to(Some(tokens));
    assert_that!(r[0].cost.clone()).is_equal_to(usd(0.5));
}

#[tokio::test(flavor = "multi_thread")]
async fn turn_without_usage_records_unknown_tokens_cost_and_model() {
    let r = played(json!({ "turns": [{ "chunks": ["ok"] }] }), 1).await;
    assert_that!(r).has_length(1);
    assert_that!(r[0].tokens).is_none();
    assert_that!(r[0].cost.clone()).is_none();
    assert_that!(r[0].model.clone()).is_none();
}

#[tokio::test(flavor = "multi_thread")]
async fn two_turns_with_growing_cost_record_each_turn_share() {
    let script = json!({ "turns": [
        { "cost": { "amount": 0.5, "currency": "USD" } },
        { "cost": { "amount": 0.75, "currency": "USD" } },
    ] });
    let r = played(script, 2).await;
    let costs: Vec<_> = r.iter().map(|r| r.cost.clone()).collect();
    assert_that!(costs).is_equal_to(vec![usd(0.5), usd(0.25)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn each_turn_gets_its_own_turn_id() {
    let r = played(json!({ "turns": [{}] }), 2).await;
    assert_that!(r[0].turn).is_not_equal_to(r[1].turn);
}

#[tokio::test(flavor = "multi_thread")]
async fn resumed_session_first_cost_is_only_the_base() {
    let script = json!({ "resume": true, "turns": [
        { "cost": { "amount": 2.0, "currency": "USD" } },
        { "cost": { "amount": 2.5, "currency": "USD" } },
    ] });
    let h = start(script).await;
    seed_session(&h, "sess-42", &h.link.agent_id()).await;
    play_turns(&h, 2).await;
    let costs: Vec<_> = records(&h, 2).await.iter().map(|r| r.cost.clone()).collect();
    assert_that!(logged(&h.log, "session/resume")).has_length(1);
    assert_that!(costs).is_equal_to(vec![None, usd(0.5)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelled_turn_is_recorded() {
    let h = start(json!({ "turns": [{ "wait_cancel": true, "usage": [3, 2, 1] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("long".into(), AgentUi::default());
    until(&mut rx, |e| matches!(e, AgentEvent::UserText { .. })).await;
    t.agent.cancel();
    let e = until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    assert_that!(e).is_equal_to(AgentEvent::TurnEnded { turn: 1, stop: "cancelled".into() });
    assert_that!(records(&h, 1).await[0].tokens.map(|t| t.total)).is_equal_to(Some(3));
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_turn_records_nothing() {
    let h = start(json!({ "turns": [{ "error": "boom" }, { "usage": [3, 2, 1] }] })).await;
    let t = h.tab("t-1");
    let mut rx = t.agent.subscribe();
    t.agent.prompt("un".into(), AgentUi::default());
    wait_error(&mut rx).await;
    t.agent.prompt("deux".into(), AgentUi::default());
    wait_turn_ended(t).await;
    // Only the second turn, the one with a reply.
    let r = records(&h, 1).await;
    assert_that!(r).has_length(1);
    assert_that!(r[0].tokens.map(|t| t.total)).is_equal_to(Some(3));
}
```

The cancelled turn waits like `tests/turns.rs:65` (the fake's `wait_cancel` still sends the
reply's `usage`); the fake answers an `error` turn with a JSON-RPC error and keeps going
(header :18-19), so the second prompt runs on the same session.

- [ ] **Step 11: Run**

Run (in the background): `cargo nextest run -p oku-slides-agent`, then
`cargo check -p oku-slides-local-agent --tests` and
`cargo clippy -p oku-slides-agent --all-targets -- -D warnings` (the workspace lints include
`clippy::pedantic`: `record_turn` borrows `usage`, `tokens_of` borrows, `usage_tab` takes the `Copy`
`DocumentKind` by value). Expected: PASS, no warning.

- [ ] **Step 12: Docs** — `crates/oku-slides-agent/ARCHITECTURE.md`, three existing lines extended, no line added:
  - the `link` bullet: "`Link::switch` changes agent and its wanted model" → "`Link::switch` changes agent, its preset id and wanted model";
  - the `session_model` line ends with: "; `usage_board`: `UsageBoard`, each session's cumulative cost (replays and unknown sessions ignored, a reopened session's first one only the base), `record_turn` files the turn's `UsageRecord` in its deck (`TabSession::record_usage`), `tells` guards session-state updates."
  - the `turn` bullet starts: "- `turn` (returns the reply's `Usage`; `prompt_images`: …".

- [ ] **Step 13: Commit** — message file `$SCRATCHPAD/msg-task6.txt`:

```
feat(agent)!: record each turn's tokens, model, preset and cost (t-1301, t-1296)

BREAKING CHANGE: `Link::with_model` is replaced by `Link::for_preset(preset, launch, model)`, and
`Link::switch` takes the preset id first.
```

`git add Cargo.lock crates/oku-slides-agent` as its own command, check `git diff --cached --stat`
holds only those, then `git commit -F "$SCRATCHPAD/msg-task6.txt"`.

### Task 7: `usage` and `show_usage` tools (session panel, MCP, local agent)

**Files:**
- Create: `crates/oku-slides-session/src/agent/usage_table.rs`, `usage_table_tests.rs` (the nine cells of a usage row, shared by the box and the CLI)
- Create: `crates/oku-slides-session/src/agent/usage_panel.rs`, `usage_panel_tests.rs`
- Modify: `crates/oku-slides-session/src/agent/mod.rs` (`pub(crate) mod usage_table;`, `mod usage_panel;`, `pub use usage_panel::usage_panel;`), `crates/oku-slides-session/src/lib.rs` (`pub use agent::usage_table::{…};`)
- Create: `crates/oku-slides-mcp/src/usage_tools.rs`; `crates/oku-slides-mcp/src/lib.rs` (`mod usage_tools;`, alphabetical)
- Modify: `crates/oku-slides-mcp/src/tool_kinds.rs` (+ `tool_kinds_tests.rs`), `tools.rs` (`list_all` :283), `handler.rs` (`call` :48), `surface_tools.rs` (`no_turn` :36 becomes `pub(crate)`), `tool_error.rs` (enum, `Localize` :95), `tool_error_tests.rs` (`KEYS`), `src/tools_tests.rs` (name lists :163-265)
- Create: `crates/oku-slides-mcp/tests/usage.rs`; modify `crates/oku-slides-mcp/tests/kinds.rs`
- Modify: `crates/oku-slides-local-agent/src/families.rs` (`families_of` :52)
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (after `packs-title`, en :150), `*/agent.ftl` (after `agent-error-packs-deck-mode`, en :130)
- Modify: `ui/src/agent/Markdown.tsx` (`case "Table"` :121), `ui/src/agent/Markdown.test.tsx` (:57), `ui/src/styles.css` (`.md table` :696), `ui/src/styles.test.ts`
- Docs: `crates/oku-slides-mcp/ARCHITECTURE.md` (:19), `crates/oku-slides-session/ARCHITECTURE.md` (:31), zero net lines each (both are at 50)

**Interfaces:**
- Consumes: `aggregate`, `By`, `Range::parse`, `UsageRangeError` (with `Localize`), `UsageReport { by, rows, totals }`, `UsageRow { key: Option<String>, turns, turns_without_tokens, input, output, thought, cache_read, cache_write, total, costs }`, `CostTotal { currency, amount, partial }`, `UsageRecord` (`Deserialize`), `shown_char(char) -> bool` (Tasks 1, 3); `TabSession::usage_ledger(&self) -> oku_slides_session::Result<Option<UsageLedger>>` and `TabSession::record_usage(&self, UsageRecord)` (Task 4, blocking: call them off the async thread).
- Produces, re-exported at the `oku_slides_session` root (Task 8 prints the same table):
  - `pub const USAGE_COLUMNS: usize = 9;` — key, turns, input, output, reasoning, cache read, cache write, total, cost; 0-based, total is column 7 and cost column 8.
  - `pub fn usage_headers(by: By, l: &Localizer) -> [String; USAGE_COLUMNS]` — localized headers; the key column names `by`.
  - `pub fn usage_cells(row: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS]` — one report row as plain text: no character `shown_char` refuses, `usage-unknown` for a row without key, `–` for an empty cell.
  - `pub fn usage_total_cells(totals: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS]` — the totals row, its key cell `usage-row-total`.
  - Cells are plain text, not Markdown: the box escapes them, the CLI prints them.
- Produces: `pub fn oku_slides_session::agent::usage_panel(id: &str, report: &UsageReport, l: &Localizer) -> Surface`;
  `Kinds::DECK_PACK_OR_THEME`; MCP tools `usage {by?, since?, until?}` → the `UsageReport` as JSON and
  `show_usage {by?, since?, until?}` → `{"shown": id}`, listed for deck, pack and theme tabs, not help tabs;
  `ToolError::UsageNoDeck`, `ToolError::UsageBadRange { source: UsageRangeError }`. mcp gains no `time` dependency.

A help tab belongs to no deck and records nothing, so it does not list the tools. The guidance "use
`show_usage` when the user asks what the deck cost" therefore lives in the `show_usage` description, not in
`instructions/deck.md`: that text is also the help tab's (`for_kind`), and
`instructions_for_a_kind_name_only_its_tools` refuses a backticked tool a help tab does not list.

- [ ] **Step 1: Fluent keys** (all 20 locales, same order, English first; `crates/oku-slides-i18n/AGENTS.md`:
  no hard-coded 1 in a plural branch, no space before a CJK counter, th noun-number-classifier, pt-BR).
  `session.ftl`, after `packs-title`, English:

```
usage-title = Agent usage
usage-badge-turns = { $count ->
    [one] { $count } turn
   *[other] { $count } turns
}
usage-badge-tokens = { $count ->
    [one] { $count } token
   *[other] { $count } tokens
}
usage-badge-no-tokens = { $count ->
    [one] { $count } turn without token counts
   *[other] { $count } turns without token counts
}
usage-badge-cost = { $amount } { $currency }
usage-badge-cost-partial = { $amount } { $currency } (partial)
usage-col-day = Day
usage-col-month = Month
usage-col-model = Model
usage-col-agent = Agent
usage-col-tab = Tab
usage-col-deck = Deck
usage-col-turns = Turns
usage-col-input = Input
usage-col-output = Output
usage-col-thought = Reasoning
usage-col-cache-read = Cache read
usage-col-cache-write = Cache write
usage-col-total = Total
usage-col-cost = Cost
usage-unknown = unknown
usage-row-total = Total
usage-empty = No agent turn recorded yet.
```

French:

```
usage-title = Usage de l'agent
usage-badge-turns = { $count ->
    [one] { $count } tour
   *[other] { $count } tours
}
usage-badge-tokens = { $count ->
    [one] { $count } token
   *[other] { $count } tokens
}
usage-badge-no-tokens = { $count ->
    [one] { $count } tour sans décompte de tokens
   *[other] { $count } tours sans décompte de tokens
}
usage-badge-cost = { $amount } { $currency }
usage-badge-cost-partial = { $amount } { $currency } (partiel)
usage-col-day = Jour
usage-col-month = Mois
usage-col-model = Modèle
usage-col-agent = Agent
usage-col-tab = Onglet
usage-col-deck = Présentation
usage-col-turns = Tours
usage-col-input = Entrée
usage-col-output = Sortie
usage-col-thought = Raisonnement
usage-col-cache-read = Cache lu
usage-col-cache-write = Cache écrit
usage-col-total = Total
usage-col-cost = Coût
usage-unknown = inconnu
usage-row-total = Total
usage-empty = Aucun tour d'agent enregistré pour l'instant.
```

`agent.ftl`, after `agent-error-packs-deck-mode`, English, then French:

```
agent-error-usage-no-deck = This tab's usage belongs to no deck: only a pack or theme tab the user opened from a deck, or a deck's own pack, counts toward that deck.
```

```
agent-error-usage-no-deck = L'usage de cet onglet n'appartient à aucune présentation : seul un onglet de pack ou de thème que l'utilisateur a ouvert depuis une présentation, ou le pack propre d'une présentation, compte pour elle.
```

The bad-range text is core's (`UsageRangeError`, Task 3): no `agent-error-usage-bad-range` key.
Translate the 18 other locales.

`usage-badge-cost` and `usage-badge-cost-partial` also format the cost cell of the table.

- [ ] **Step 2: Write the failing table tests** — `crates/oku-slides-session/src/agent/usage_table_tests.rs`

```rust
use oku_slides_core::{By, Range, UsageRecord, UsageReport, aggregate};
use oku_slides_i18n::Localizer;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::{USAGE_COLUMNS, usage_cells, usage_headers, usage_total_cells};

/// A deck turn of `day`, with or without token counts; built from JSON, as read from a ledger.
pub(crate) fn record(n: u128, day: &str, model: &str, tokens: bool) -> UsageRecord {
    let tokens = tokens.then(|| {
        json!({ "total": 1850, "input": 1840, "output": 10, "thought": null, "cache_read": 5, "cache_write": null })
    });
    serde_json::from_value(json!({
        "turn": Uuid::from_u128(n), "at": format!("{day}T14:02:11Z"), "tab": "deck", "agent": "claude",
        "model": model, "tokens": tokens, "cost": { "amount": 0.04, "currency": "USD" },
    }))
    .unwrap()
}

/// The report of one deck's `records`.
pub(crate) fn report(records: &[UsageRecord], by: By) -> UsageReport {
    aggregate(records.iter().map(|r| ("", r)), by, Range::ALL)
}

fn first_row(records: &[UsageRecord], by: By, l: &Localizer) -> [String; USAGE_COLUMNS] {
    usage_cells(&report(records, by).rows[0], l)
}

#[rstest]
#[case(By::Day, "Day")]
#[case(By::Month, "Month")]
#[case(By::Model, "Model")]
#[case(By::Agent, "Agent")]
#[case(By::Tab, "Tab")]
#[case(By::Deck, "Deck")]
fn usage_headers_key_column_names_the_grouping(#[case] by: By, #[case] expected: &str) {
    assert_that!(usage_headers(by, &Localizer::new(Some("en")))[0].as_str()).is_equal_to(expected);
}

#[test]
fn usage_headers_in_french_end_with_total_then_cost() {
    let h = usage_headers(By::Model, &Localizer::new(Some("fr")));
    assert_that!(h[7].as_str()).is_equal_to("Total");
    assert_that!(h[8].as_str()).is_equal_to("Coût");
}

#[test]
fn usage_cells_row_with_tokens_and_cost_fills_every_column() {
    let cells = first_row(&[record(1, "2026-10-30", "m", true)], By::Day, &Localizer::new(Some("en")));
    let expected = ["2026-10-30", "1", "1840", "10", "0", "5", "0", "1850", "0.0400 USD"].map(str::to_owned);
    assert_that!(cells).is_equal_to(expected);
}

#[test]
fn usage_cells_row_without_cost_shows_the_placeholder() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.cost = None;
    assert_that!(first_row(&[r], By::Day, &Localizer::new(Some("en")))[8].as_str()).is_equal_to("–");
}

#[test]
fn usage_cells_partial_cost_says_so() {
    let mut no_cost = record(2, "2026-10-30", "m", true);
    no_cost.cost = None;
    let cells = first_row(&[record(1, "2026-10-30", "m", true), no_cost], By::Day, &Localizer::new(Some("en")));
    assert_that!(cells[8].as_str()).is_equal_to("0.0400 USD (partial)");
}

#[test]
fn usage_cells_row_without_model_is_unknown_in_french() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.model = None;
    assert_that!(first_row(&[r], By::Model, &Localizer::new(Some("fr")))[0].as_str()).is_equal_to("inconnu");
}

#[test]
fn usage_cells_key_with_control_bidi_and_format_characters_drops_them() {
    // A record read without `is_valid` (as `aggregate` takes it): the cell still never reaches a
    // terminal with an escape sequence or a reordering mark.
    let hostile = "a\u{1b}[31mb\u{202E}c\u{200B}d\u{2028}e";
    let cells = first_row(&[record(1, "2026-10-30", hostile, true)], By::Model, &Localizer::new(Some("en")));
    assert_that!(cells[0].as_str()).is_equal_to("a[31mbcde");
}

#[test]
fn usage_total_cells_key_is_the_total_label() {
    let r = report(&[record(1, "2026-10-29", "m", true), record(2, "2026-10-30", "m", false)], By::Day);
    let cells = usage_total_cells(&r.totals, &Localizer::new(Some("fr")));
    assert_that!(cells[0].as_str()).is_equal_to("Total");
    assert_that!(cells[1].as_str()).is_equal_to("2");
}
```

The day key reaches `usage_cells` as `aggregate` formats it (`YYYY-MM-DD`, Task 3).

- [ ] **Step 3: Run them to see them fail** — `cargo nextest run -p oku-slides-session usage_table` (background the first time: a cold build). Expected: compile error, `usage_table` not found.

- [ ] **Step 4: Implement `usage_table.rs`**

```rust
//! The nine cells of a usage report row, shared by the `show_usage` box and the CLI.
//!
//! Keys come from an untrusted ledger: every cell drops what [`shown_char`] refuses, so a terminal
//! shows it as text. Cells are plain text; the box escapes them for Markdown itself.

use oku_slides_core::{By, CostTotal, UsageRow, shown_char};
use oku_slides_i18n::Localizer;

/// Columns of a usage table: the key, seven turn and token counts, then the cost.
pub const USAGE_COLUMNS: usize = 9;

/// A cell with nothing to show; keeps the columns aligned.
const EMPTY_CELL: &str = "–";

/// The localized headers of a report grouped by `by`.
#[must_use]
pub fn usage_headers(by: By, l: &Localizer) -> [String; USAGE_COLUMNS] {
    let key = match by {
        By::Day => "usage-col-day",
        By::Month => "usage-col-month",
        By::Model => "usage-col-model",
        By::Agent => "usage-col-agent",
        By::Tab => "usage-col-tab",
        By::Deck => "usage-col-deck",
    };
    [
        key,
        "usage-col-turns",
        "usage-col-input",
        "usage-col-output",
        "usage-col-thought",
        "usage-col-cache-read",
        "usage-col-cache-write",
        "usage-col-total",
        "usage-col-cost",
    ]
    .map(|id| l.tr(id))
}

/// One report row as plain text; a row without key shows `usage-unknown`.
#[must_use]
pub fn usage_cells(row: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS] {
    let key = row.key.as_deref().map_or_else(|| l.tr("usage-unknown"), shown);
    cells(key, row, l)
}

/// The totals row as plain text, its key cell `usage-row-total`.
#[must_use]
pub fn usage_total_cells(totals: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS] {
    cells(l.tr("usage-row-total"), totals, l)
}

/// One currency's cost, marked partial when some turns sent none.
pub(crate) fn usage_cost(c: &CostTotal, l: &Localizer) -> String {
    let id = if c.partial { "usage-badge-cost-partial" } else { "usage-badge-cost" };
    let amount = format!("{:.4}", c.amount);
    l.tr_with(id, &[("amount", amount.into()), ("currency", shown(&c.currency).into())])
}

fn cells(key: String, r: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS] {
    let costs: Vec<String> = r.costs.iter().map(|c| usage_cost(c, l)).collect();
    let cost = if costs.is_empty() { EMPTY_CELL.to_owned() } else { costs.join(", ") };
    let key = if key.is_empty() { EMPTY_CELL.to_owned() } else { key };
    [
        key,
        r.turns.to_string(),
        r.input.to_string(),
        r.output.to_string(),
        r.thought.to_string(),
        r.cache_read.to_string(),
        r.cache_write.to_string(),
        r.total.to_string(),
        cost,
    ]
}

/// `s` without the characters a name may not show.
fn shown(s: &str) -> String {
    s.chars().filter(|&c| shown_char(c)).collect()
}

#[cfg(test)]
#[path = "usage_table_tests.rs"]
pub(crate) mod usage_table_tests;
```

`UsageRow.thought`, `cache_read` and `cache_write` are the summed `u64`s of the turns that reported them
(Task 3). `shown_char` is core's (Task 1): it refuses control characters, Unicode format characters
(bidi marks and overrides, zero-width characters, U+061C, U+FEFF) and U+2028/U+2029. A key filtered down
to nothing shows the placeholder, so no column shifts.

In `agent/mod.rs`, alphabetical after `turn_view`: `mod usage_panel;`, `pub(crate) mod usage_table;` and
`pub use usage_panel::usage_panel;`. In `lib.rs`, next to the other `pub use` lines:
`pub use agent::usage_table::{USAGE_COLUMNS, usage_cells, usage_headers, usage_total_cells};`.

- [ ] **Step 5: Write the failing panel tests** — `crates/oku-slides-session/src/agent/usage_panel_tests.rs`

```rust
use oku_slides_core::{By, UsageReport};
use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{md_escape, usage_panel};
use crate::agent::usage_table::usage_table_tests::{record, report};
use crate::deck_packs::deck_packs_tests::{component, text_of};

fn two_turns() -> UsageReport {
    report(&[record(1, "2026-10-29", "claude-opus-5-5", true), record(2, "2026-10-30", "claude-opus-5-5", false)], By::Day)
}

#[test]
fn usage_panel_french_titles_the_box_and_counts_turns() {
    let s = usage_panel("usage-1", &two_turns(), &Localizer::new(Some("fr")));
    assert_that!(text_of(&s, "title")).is_equal_to("Usage de l'agent".to_owned());
    assert_that!(text_of(&s, "turns")).is_equal_to("2 tours".to_owned());
}

#[test]
fn usage_panel_tokens_badge_uses_the_plural_form() {
    let one = report(&[record(1, "2026-10-30", "m", false)], By::Day);
    let s = usage_panel("usage-1", &one, &Localizer::new(Some("en")));
    assert_that!(text_of(&s, "tokens")).is_equal_to("0 tokens".to_owned());
    assert_that!(text_of(&s, "no-tokens")).is_equal_to("1 turn without token counts".to_owned());
}

#[test]
fn usage_panel_french_table_has_localized_headers_and_a_total_row() {
    let s = usage_panel("usage-1", &two_turns(), &Localizer::new(Some("fr")));
    let table = text_of(&s, "table");
    assert_that!(table.as_str()).starts_with("| Jour | Tours | Entrée | Sortie | Raisonnement |");
    assert_that!(table.as_str()).contains("| 2026\\-10\\-29 | 1 | 1840 | 10 |");
    assert_that!(table.as_str()).contains("| **Total** | 2 |");
}

#[test]
fn usage_panel_cost_cell_is_escaped_like_every_cell() {
    let mut no_cost = record(2, "2026-10-30", "m", true);
    no_cost.cost = None;
    let r = report(&[record(1, "2026-10-30", "m", true), no_cost], By::Day);
    let s = usage_panel("usage-1", &r, &Localizer::new(Some("en")));
    assert_that!(text_of(&s, "table").as_str()).contains(r"| 0\.0400 USD \(partial\) |");
}

#[test]
fn usage_panel_row_without_cost_shows_the_placeholder_cell() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.cost = None;
    let s = usage_panel("usage-1", &report(&[r], By::Day), &Localizer::new(Some("en")));
    // Eight columns of numbers or text, then the cost placeholder: no column shifts.
    assert_that!(text_of(&s, "table").as_str()).contains("| 1850 | – |");
}

#[test]
fn usage_panel_empty_report_says_so_and_has_no_table() {
    let s = usage_panel("usage-1", &report(&[], By::Day), &Localizer::new(Some("fr")));
    assert_that!(text_of(&s, "table")).is_equal_to("Aucun tour d'agent enregistré pour l'instant.".to_owned());
    assert_that!(component(&s, "cost-0")).is_equal_to(serde_json::Value::Null);
}

#[test]
fn usage_panel_unknown_model_key_is_localized() {
    let mut r = record(1, "2026-10-30", "m", true);
    r.model = None;
    let s = usage_panel("usage-1", &report(&[r], By::Model), &Localizer::new(Some("fr")));
    assert_that!(text_of(&s, "table").as_str()).contains("| inconnu | 1 |");
}

#[test]
fn md_escape_link_and_pipe_are_plain_text() {
    assert_that!(md_escape("[x](https://evil)|")).is_equal_to(r"\[x\]\(https\:\/\/evil\)\|".to_owned());
}

#[test]
fn md_escape_control_and_bidi_characters_are_dropped() {
    assert_that!(md_escape("a\nb\u{1b}[31mc\u{202E}d")).is_equal_to(r"ab\[31mcd".to_owned());
}

#[test]
fn usage_panel_model_key_with_markup_is_escaped_in_the_table() {
    let r = report(&[record(1, "2026-10-30", "[x](https://evil)|", true)], By::Model);
    let s = usage_panel("usage-1", &r, &Localizer::new(Some("en")));
    assert_that!(text_of(&s, "table").as_str()).contains(r"| \[x\]\(https\:\/\/evil\)\| | 1 |");
}

#[test]
fn usage_panel_hostile_model_key_stays_in_its_cell() {
    // Backticks, pipes, emphasis, a bidi override and a line break: one escaped, single-line cell.
    let hostile = "`x`|**y**\u{202E}\nz";
    let r = report(&[record(1, "2026-10-30", hostile, true)], By::Model);
    let table = text_of(&usage_panel("usage-1", &r, &Localizer::new(Some("en"))), "table");
    assert_that!(table.as_str()).contains(r"| \`x\`\|\*\*y\*\*z | 1 |");
    // Header, delimiter, one row, the totals: the key broke no line.
    assert_that!(table.lines().count()).is_equal_to(4);
}
```

`deck_packs_tests` is `pub(crate)` and its `component` / `text_of` read a surface's components by id
(`crate::deck_packs::deck_packs_tests`, as `deck_packs_panel_tests.rs:9` imports them). The model
`[x](https://evil)|` passes `UsageRecord::is_valid` (no control or format character), so `aggregate` keeps
it; the hostile key does not, and is fed to `aggregate` directly, as a forged report would be.

- [ ] **Step 6: Implement `usage_panel.rs`**

```rust
//! The `show_usage` box: totals as badges, then the rows as a Markdown table.
//!
//! Cells come from [`super::usage_table`] as plain text; [`md_escape`] keeps every one plain in Markdown.

use oku_slides_a2ui::{Component, ComponentKind, Dynamic, TextVariant};
use oku_slides_core::{UsageReport, shown_char};
use oku_slides_i18n::Localizer;
use serde_json::json;

use super::forms::{Place, Surface, surface};
use super::usage_table::{USAGE_COLUMNS, usage_cells, usage_cost, usage_headers, usage_total_cells};

/// Builds the usage box of `report`, filed in the turn's thread by the hub.
#[must_use]
pub fn usage_panel(id: &str, report: &UsageReport, l: &Localizer) -> Surface {
    let t = &report.totals;
    let mut badges = vec![
        badge("turns", l.tr_with("usage-badge-turns", &[("count", t.turns.into())])),
        badge("tokens", l.tr_with("usage-badge-tokens", &[("count", t.total.into())])),
    ];
    if t.turns_without_tokens > 0 {
        badges.push(badge("no-tokens", l.tr_with("usage-badge-no-tokens", &[("count", t.turns_without_tokens.into())])));
    }
    // A badge is plain text, not Markdown: the cost needs no escaping there.
    for (i, c) in t.costs.iter().enumerate() {
        badges.push(badge(&format!("cost-{i}"), usage_cost(c, l)));
    }
    let ids: Vec<String> = badges.iter().map(|b| b.id.clone()).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let text = if report.rows.is_empty() { l.tr("usage-empty") } else { table(report, l) };
    let mut components = vec![
        Component::column("root", &["title", "badges", "table"]),
        Component::text("title", l.tr("usage-title"), Some(TextVariant::H4)),
        Component::row("badges", &refs),
        Component::new("table", ComponentKind::Markdown { text: Dynamic::Literal(text) }),
    ];
    components.extend(badges);
    surface(id, Place::Thread, None, components, json!({}))
}

fn badge(id: &str, text: String) -> Component {
    Component::new(id, ComponentKind::Badge { text: Dynamic::Literal(text), tone: None })
}

/// The GFM table: headers, delimiter, one line per row, then the totals in bold.
fn table(report: &UsageReport, l: &Localizer) -> String {
    let mut out = line(&usage_headers(report.by, l).map(|c| md_escape(&c)));
    out.push('|');
    out.push_str(&"---|".repeat(USAGE_COLUMNS));
    out.push('\n');
    for row in &report.rows {
        out.push_str(&line(&usage_cells(row, l).map(|c| md_escape(&c))));
    }
    let mut total = usage_total_cells(&report.totals, l).map(|c| md_escape(&c));
    total[0] = format!("**{}**", total[0]);
    out.push_str(&line(&total));
    out
}

fn line(cells: &[String; USAGE_COLUMNS]) -> String {
    format!("| {} |\n", cells.join(" | "))
}

/// Backslash-escapes ASCII punctuation and drops what `shown_char` refuses: text, never markup.
fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars().filter(|&c| shown_char(c)) {
        if ch.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
#[path = "usage_panel_tests.rs"]
mod usage_panel_tests;
```

Every cell, the cost cell and the headers included, goes through `md_escape`: a translation may hold `|`
or `*`, and the currency comes from the ledger. The `**…**` of the total is added after escaping, so it
stays markup. `\n` is a control character, so a key never breaks the table's line. `Component::row` and
`Component::column` take `&[&str]` (`oku-slides-a2ui/src/component.rs:480-487`); `surface` is
`forms::surface` (`forms.rs:277`, `pub(crate)`); the hub's `show_static` sets `turn` and `live = false`
itself (`hub_turns.rs:197`).

- [ ] **Step 7: Run** — `cargo nextest run -p oku-slides-session usage_table usage_panel`. Expected: PASS.

- [ ] **Step 8: `Kinds::DECK_PACK_OR_THEME`** — in `tool_kinds.rs`, after `THEME`:

```rust
    /// Deck, pack and theme tabs: what belongs to a deck, never a help tab.
    pub(crate) const DECK_PACK_OR_THEME: Self = Self::DECK.or(Self::PACK).or(Self::THEME);
```

Add to the rstest of `tool_kinds_tests.rs`:

```rust
#[case(Kinds::DECK_PACK_OR_THEME, DocumentKind::Help, false)]
#[case(Kinds::DECK_PACK_OR_THEME, DocumentKind::Theme, true)]
```

- [ ] **Step 9: ToolErrors** — `tool_error.rs`, after `PacksDeckMode`:

```rust
    #[snafu(display("this tab's usage belongs to no deck"))]
    UsageNoDeck { backtrace: Backtrace },
    #[snafu(display("invalid usage date range"))]
    UsageBadRange { source: oku_slides_core::UsageRangeError, backtrace: Backtrace },
```

Add `UsageBadRange, UsageNoDeck` to the alphabetical `use ToolError::{…}` of `localize`, and the arms:

```rust
            UsageNoDeck { .. } => l.tr("agent-error-usage-no-deck"),
            // The agent's own dates, told back by core in the user's language.
            UsageBadRange { source, .. } => source.localize(l),
```

In `tool_error_tests.rs` add `"agent-error-usage-no-deck",` to `KEYS` (after `"agent-error-empty-dictation"`)
and, importing `UsageNoDeckSnafu`:

```rust
#[test]
fn usage_no_deck_localized_in_french_is_its_key() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(UsageNoDeckSnafu.build().localize(&fr)).is_equal_to(fr.tr("agent-error-usage-no-deck"));
}
```

- [ ] **Step 10: Write the failing MCP tests** — `crates/oku-slides-mcp/tests/usage.rs`

```rust
//! The `usage` and `show_usage` tools: the linked deck's ledger, as JSON or a thread box.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use std::sync::Arc;

use common::workspace_harness::start_in_pack_tab;
use common::*;
use oku_slides_core::{Range, UsageRecord};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::PAPIER_THEME;
use oku_slides_session::agent::{Sessions, TabSession};
use serde_json::json;
use speculoos::prelude::*;

/// A deck turn, as a ledger line holds it.
fn record(n: u128) -> UsageRecord {
    serde_json::from_value(json!({
        "turn": uuid::Uuid::from_u128(n), "at": "2026-10-30T14:02:11Z", "tab": "deck", "agent": "claude",
        "model": "claude-opus-5-5",
        "tokens": { "total": 1850, "input": 1840, "output": 10, "thought": null, "cache_read": 5, "cache_write": null },
        "cost": { "amount": 0.04, "currency": "USD" },
    }))
    .unwrap()
}

/// Records `n` in `tab`'s deck, off the async thread as the agent stack does.
async fn record_in(tab: TabSession, n: u128) {
    tokio::task::spawn_blocking(move || tab.record_usage(record(n))).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_on_a_deck_tab_reports_its_recorded_turns() {
    let h = start().await;
    record_in(h.tab.clone(), 1).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = json_of(&call(&c, "usage", json!({ "by": "model" })).await);
    assert_that!(r["by"].as_str()).is_equal_to(Some("model"));
    assert_that!(r["totals"]["turns"].as_u64()).is_equal_to(Some(1));
    assert_that!(r["rows"][0]["key"].as_str()).is_equal_to(Some("claude-opus-5-5"));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_since_after_until_is_refused_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "usage", json!({ "since": "2026-10-31", "until": "2026-10-30" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    // The harness localizer is fr: the core error's own French text.
    let expected = Range::parse(Some("2026-10-31"), Some("2026-10-30")).unwrap_err().localize(&Localizer::new(Some("fr")));
    assert_that!(text_of(&r)).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_malformed_day_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "usage", json!({ "since": "30/10/2026" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_in_a_pack_tab_opened_from_a_deck_reads_that_deck() {
    let (h, ws, _tab) = start_in_pack_tab().await;
    let sessions: Arc<dyn Sessions> = ws.clone();
    record_in(TabSession::new("t-1", sessions), 1).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = json_of(&call(&c, "usage", json!({})).await);
    assert_that!(r["totals"]["turns"].as_u64()).is_equal_to(Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_in_a_theme_tab_without_deck_is_refused_no_deck() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "usage", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(Localizer::new(Some("fr")).tr("agent-error-usage-no-deck"));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_usage_files_its_box_in_the_open_turn() {
    let h = start().await;
    record_in(h.tab.clone(), 1).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn();
    let r = json_of(&call(&c, "show_usage", json!({})).await);
    let id = r["shown"].as_str().unwrap().to_owned();
    assert_that!(id.as_str()).starts_with("usage");
    assert_that!(open_ids(&h)).contains(id);
}

#[tokio::test(flavor = "multi_thread")]
async fn show_usage_outside_a_turn_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "show_usage", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let fr = Localizer::new(Some("fr"));
    assert_that!(text_of(&r)).is_equal_to(fr.tr_with("agent-error-no-turn", &[("tool", "show_usage".into())]));
    assert_that!(open_ids(&h)).is_empty();
}
```

`start()` serves the deck tab of a single `Mutex<Session>` (`common/mod.rs:220`); `start_in_pack_tab`
opens `acme` from deck tab `t-1` as the user does, so `Tab.opener` links it (`workspace_harness.rs:143`);
`start_theme_tab` is a lone theme tab with no deck. Each relies on Task 4's routing.

Lists: in `src/tools_tests.rs` add `"show_usage"` (after `"show_themes"`, or after `"show_params"` in
`PACK_TOOLS` and `THEME_TOOLS`) and `"usage"` (after `"theme_tokens"`) to `DECK_TOOLS`, `PACK_TOOLS` and
`THEME_TOOLS`; drop `#[case::help(DocumentKind::Help, DECK_TOOLS)]` and add:

```rust
/// A help tab lists the deck's tools but the usage ones: it belongs to no deck.
#[test]
fn list_for_help_is_the_deck_tools_without_usage() {
    let mut names: Vec<String> = list_for(DocumentKind::Help).into_iter().map(|t| t.name).collect();
    names.sort();
    let expected: Vec<String> =
        DECK_TOOLS.iter().filter(|n| !matches!(**n, "usage" | "show_usage")).map(|n| (*n).to_owned()).collect();
    assert_that!(names).is_equal_to(expected);
}
```

In `tests/kinds.rs`, add `"usage", "show_usage"` to the `has` lists of `deck`, `pack` and `theme`, and to `lacks` of `help`.

- [ ] **Step 11: Run them to see them fail** — `cargo nextest run -p oku-slides-mcp usage list_for kinds_has`. Expected: `usage` is `UnknownTool`, lists differ.

- [ ] **Step 12: Implement `usage_tools.rs`**

```rust
//! Usage tools: the linked deck's agent turns summed, as JSON or as a thread box.

use oku_slides_core::{By, Range, UsageReport, aggregate};
use oku_slides_session::agent::usage_panel;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{OptionExt, ResultExt};

use crate::tool_error::{SessionSnafu, TaskSnafu, ToolError, UsageBadRangeSnafu, UsageNoDeckSnafu};
use crate::tools::Tools;

/// Row grouping offered to the agent: one deck, so no `deck`.
#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UsageBy {
    /// One row per UTC day.
    #[default]
    Day,
    /// One row per UTC month.
    Month,
    /// One row per model.
    Model,
    /// One row per agent preset.
    Agent,
    /// One row per tab kind: deck, pack or theme.
    Tab,
}

impl From<UsageBy> for By {
    fn from(b: UsageBy) -> Self {
        match b {
            UsageBy::Day => Self::Day,
            UsageBy::Month => Self::Month,
            UsageBy::Model => Self::Model,
            UsageBy::Agent => Self::Agent,
            UsageBy::Tab => Self::Tab,
        }
    }
}

/// The report asked by `usage` and `show_usage`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UsageArgs {
    /// Row grouping: day (default), month, model, agent or tab.
    #[serde(default)]
    pub by: UsageBy,
    /// First UTC day kept, as YYYY-MM-DD.
    #[serde(default)]
    pub since: Option<String>,
    /// Last UTC day kept, as YYYY-MM-DD.
    #[serde(default)]
    pub until: Option<String>,
}

impl Tools {
    pub(crate) async fn usage(&self, args: Value) -> CallToolResult {
        let args: UsageArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let report = self.usage_report(args).await;
        self.reply(report, |report| self.json_result(&report))
    }

    /// The box goes in the call's turn, like `render`: refused outside a turn.
    pub(crate) async fn show_usage(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let args: UsageArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let Some(turn) = turn else { return self.no_turn("show_usage") };
        let report = self.usage_report(args).await;
        self.reply(report, |report| {
            let id = self.hub.next_id("usage");
            let surface = usage_panel(&id, &report, &self.localizer.current());
            if self.hub.show_static(turn, surface) {
                self.json_result(&json!({ "shown": id }))
            } else {
                self.no_turn("show_usage")
            }
        })
    }

    /// The linked deck's report; reading the ledger blocks, so it runs off the async thread.
    async fn usage_report(&self, args: UsageArgs) -> Result<UsageReport, ToolError> {
        let range = Range::parse(args.since.as_deref(), args.until.as_deref()).context(UsageBadRangeSnafu)?;
        let (session, by) = (self.session.clone(), By::from(args.by));
        tokio::task::spawn_blocking(move || {
            session.usage_ledger().map(|l| l.map(|ledger| aggregate(ledger.records().map(|r| ("", r)), by, range)))
        })
        .await
        .context(TaskSnafu)?
        .context(SessionSnafu)?
        .context(UsageNoDeckSnafu)
    }
}
```

In `surface_tools.rs:36`, `fn no_turn` becomes `pub(crate) fn no_turn`. Register in `list_all`, after `render`:

```rust
            (tool::<crate::usage_tools::UsageArgs>("usage", "Agent usage recorded in this deck (in a pack or theme tab: the deck it belongs to): tokens, turns and cost by day, month, model, agent or tab, plus totals. Costs are per currency, partial when some turns sent none. Returns the report as JSON."), Kinds::DECK_PACK_OR_THEME),
            (tool::<crate::usage_tools::UsageArgs>("show_usage", "Show the agent usage report in the thread: badges for the totals, then a table of rows. Use it when the user asks what this deck cost or how many tokens it used; `usage` returns the same report as JSON. Returns {shown}."), Kinds::DECK_PACK_OR_THEME),
```

In `handler.rs` `call`, after `"render"`: `"usage" => self.usage(args).await,` and `"show_usage" => self.show_usage(turn, args).await,`.

- [ ] **Step 13: Local agent** — `families.rs`, `families_of`: add `| "usage" | "show_usage"` to the `Configure`
  group (`"set_setting" | … | "cancel_agent_model_download" => &[Configure]`). `families_every_mcp_tool_has_a_family`
  guards it.

- [ ] **Step 14: Wide tables scroll in their own box** — WebKit drops the table role of a `<table>` set to
  `display: block` (VoiceOver no longer reads rows and columns), so the table keeps its display and a
  wrapper scrolls. Test first, in `ui/src/agent/Markdown.test.tsx`, replacing the GFM table test (:57):

```ts
  test("Markdown GFM table renders inside a focusable scrolling box", () => {
    expect(html("| a | b |\n|---|---|\n| 1 | 2 |")).toBe(
      '<div class="md-table" tabindex="0"><table><thead><tr><th>a</th><th>b</th></tr></thead>'
        + "<tbody><tr><td>1</td><td>2</td></tr></tbody></table></div>",
    );
  });
```

and in `ui/src/styles.test.ts`:

```ts
test("a wide Markdown table scrolls sideways in its box and keeps its table display", () => {
  const box = ruleOf(APP, ".md .md-table");
  expect(box).toMatch(/overflow-x:\s*auto/);
  expect(box).toMatch(/max-width:\s*100%/);
  expect(ruleOf(APP, ".md table")).not.toMatch(/display:/);
});
```

Run `cd ui && bun test src/agent/Markdown.test.tsx src/styles.test.ts`. Expected: both fail (no wrapper, no rule).

In `ui/src/agent/Markdown.tsx`, `case "Table"` becomes:

```tsx
    case "Table": {
      const head = node.getChild("TableHeader");
      const rows = node.getChildren("TableRow");
      // A wide table scrolls in its own box; `tabIndex` lets the keyboard scroll it too.
      return (
        <div key={key} className="md-table" tabIndex={0}>
          <table>
            {head && <thead>{row(head, text, "th", 0)}</thead>}
            {rows.length > 0 && <tbody>{rows.map((r, i) => row(r, text, "td", i))}</tbody>}
          </table>
        </div>
      );
    }
```

In `ui/src/styles.css:696`, the table loses its margin to the box:

```css
.md .md-table { max-width: 100%; overflow-x: auto; margin: 0 0 8px; }
.md table { border-collapse: collapse; font-size: 14px; }
```

`.md > :last-child { margin-bottom: 0; }` (:683) applies to the box as it did to the table. The global
`:focus-visible` outline (:141) shows when the keyboard focuses the box. Run the two test files again. Expected: PASS.

- [ ] **Step 15: Run** — `cargo nextest run -p oku-slides-mcp -p oku-slides-local-agent -p oku-slides-agent -p oku-slides-session -p oku-slides-i18n` (background), `cargo clippy -p oku-slides-session -p oku-slides-mcp --all-targets -- -D warnings`, and `cd ui && bun test src/agent/Markdown.test.tsx src/styles.test.ts && bunx tsc --noEmit`.
  Expected: PASS, including `tools_list_input_schema_descriptions_are_plain_english`,
  `instructions_for_a_kind_name_only_its_tools` (help names no usage tool) and
  `every_error_key_is_translated_in_every_shipped_language`.

- [ ] **Step 16: Docs** (zero net lines) — mcp `ARCHITECTURE.md:19` ends
  `` `voice_tools` (seven tools, checked before asking, no API target argument), `usage_tools` (`usage`, `show_usage`: the linked deck's ledger, a thread box; not in help tabs). ``;
  session `ARCHITECTURE.md:31`: `` surfaces (`forms`, `turn_view`, `receipt`, `render`, `surface_ops`, `usage_panel` over the shared `usage_table` cells), `address`, `variants`, ``.

- [ ] **Step 17: Commit** — `feat(mcp,session,ui)!: usage and show_usage tools (t-1302, t-1296)`; the `!`
  because help tabs no longer list every deck tool and `oku_slides_session` gains public items Task 8
  depends on. Stage `crates/oku-slides-session crates/oku-slides-mcp crates/oku-slides-local-agent crates/oku-slides-i18n ui/src/agent/Markdown.tsx ui/src/agent/Markdown.test.tsx ui/src/styles.css ui/src/styles.test.ts`.

### Task 8: `oku-slides usage` (CLI)

**Files:**
- Create: `crates/oku-slides-cli/src/usage_cmd.rs`, `crates/oku-slides-cli/src/usage_cmd_tests.rs`
- Create: `crates/oku-slides-cli/tests/usage.rs`
- Modify: `crates/oku-slides-cli/src/main.rs` (`mod` list :7-14, `Cmd` :50-111, `command` :113-166, `run` :442-534)
- Modify: `crates/oku-slides-i18n/src/locales/*/cli.ftl` (after `cli-arg-ui`, en :81)
- Docs: `crates/oku-slides-cli/ARCHITECTURE.md` (49 lines: merge into the `bench.rs` bullet, net 0 or +1 line, never past 50)

**Interfaces:**
- Consumes: `aggregate`, `By`, `Range::parse`, `UsageReport { by, rows, totals }`, `UsageRow`, `UsageLedger::records`,
  `UsageRecord.turn`, `shown_char` (Tasks 1, 3); `SourceBundle.usage` and the ledger `Issue`s (ids `issue-usage-*`,
  read through `Issue.message.id()`) in `Opened.issues` (Task 2); `read_document`, `EXTENSION` (session); the shared
  table of Task 7, re-exported from the `oku_slides_session` root: `USAGE_COLUMNS: usize = 9`,
  `usage_headers(by: By, l: &Localizer) -> [String; USAGE_COLUMNS]` (column 0 named after `by`, then turns, input,
  output, reasoning, cache read, cache write, total (7), cost (8)), `usage_cells(row: &UsageRow, l: &Localizer) ->
  [String; USAGE_COLUMNS]` and `usage_total_cells(totals: &UsageRow, l: &Localizer) -> [String; USAGE_COLUMNS]`
  (plain, terminal-safe text: key and currency already filtered by `shown_char`, `usage-unknown` for a row without
  key, `–` for an empty cell, `usage-row-total` as the totals' key); `crate::registry`, `crate::log_issues`,
  `Out::data` (main.rs).
- Produces: `Cmd::Usage { decks, by, since, until, json }`; `usage_cmd::{CliBy, UsageArgs, usage, format_table}`.

Design notes (from the review): the CLI reads each deck with `read_document` and an **empty** widget
registry (`registry(app_fonts)` with no pack set returns `empty_registry()`, no render worker, no
probe), logs `Opened.issues` **once** (no `load_bundle`, which logs then probes), labels a deck by its
path without `.oku-slides.pdf` (or without its extension for a JSON deck), and counts a turn found in
several decks once (a copy or a merge holds the same turns), keeping the first deck given. Dates are
parsed by core (`Range::parse`): the CLI has no `time` dependency. The CLI builds no cell itself:
headers and cells come from session's `usage_headers` / `usage_cells`, the ones the `show_usage` box
uses, so the two tables cannot drift; the CLI prints them as they are. Only ledger issues (`issue-usage-*`) are logged: with an empty
registry every widget of the deck would otherwise show up as unknown. A deck label comes from the
file system, not from the ledger, so it goes through `printable`, which drops every character core's
`shown_char` refuses (controls, bidi and other format characters, line and paragraph separators).

- [ ] **Step 1: Fluent keys** — `cli.ftl` of all 20 locales, after `cli-arg-ui`, same order. English:

```
cli-cmd-usage = Sum the agent usage recorded in decks
cli-arg-usage-decks = Decks (.oku-slides.pdf or JSON); a turn found in several counts once
cli-arg-usage-by = Group rows by day, month, model, agent, tab or deck (UTC days)
cli-arg-usage-since = First UTC day kept (YYYY-MM-DD)
cli-arg-usage-until = Last UTC day kept (YYYY-MM-DD)
cli-arg-usage-json = Print the report as JSON
```

French:

```
cli-cmd-usage = Additionner l'usage de l'agent enregistré dans des decks
cli-arg-usage-decks = Decks (.oku-slides.pdf ou JSON) ; un tour présent dans plusieurs compte une fois
cli-arg-usage-by = Regrouper les lignes par jour, mois, modèle, agent, onglet ou deck (jours UTC)
cli-arg-usage-since = Premier jour UTC gardé (AAAA-MM-JJ)
cli-arg-usage-until = Dernier jour UTC gardé (AAAA-MM-JJ)
cli-arg-usage-json = Afficher le rapport en JSON
```

Translate the 18 others following `crates/oku-slides-i18n/AGENTS.md` (keep `YYYY-MM-DD` in each
locale's own spelling of the pattern, never a literal brace). No error key here: a bad range is the
core `UsageRangeError`, localized by its own key (Task 3).

- [ ] **Step 2: Write the failing unit tests** — `crates/oku-slides-cli/src/usage_cmd_tests.rs`

```rust
use oku_slides_core::{By, Issue, Range, UsageLedger, UsageRecord, aggregate};
use oku_slides_i18n::Localizer;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

/// A record read from its JSON line, as `usage.jsonl` holds it (no `time` dependency here).
fn record(turn: u128, at: &str, model: Option<&str>, input: u64, cost: Option<(f64, &str)>) -> UsageRecord {
    let line = serde_json::json!({
        "turn": format!("00000000-0000-0000-0000-{turn:012x}"),
        "at": at,
        "tab": "deck",
        "agent": "claude",
        "model": model,
        "tokens": { "total": input + 10, "input": input, "output": 10, "thought": null, "cache_read": 3, "cache_write": null },
        "cost": cost.map(|(amount, currency)| serde_json::json!({ "amount": amount, "currency": currency })),
    });
    serde_json::from_value(line).unwrap()
}

fn report(records: &[UsageRecord], by: By) -> UsageReport {
    aggregate(records.iter().map(|r| ("d", r)), by, Range::ALL)
}

#[rstest]
#[case(CliBy::Day, By::Day)]
#[case(CliBy::Month, By::Month)]
#[case(CliBy::Model, By::Model)]
#[case(CliBy::Agent, By::Agent)]
#[case(CliBy::Tab, By::Tab)]
#[case(CliBy::Deck, By::Deck)]
fn cli_by_maps_to_the_core_grouping_of_the_same_name(#[case] cli: CliBy, #[case] by: By) {
    assert_that!(By::from(cli)).is_equal_to(by);
}

#[rstest]
#[case("talks/q4.oku-slides.pdf", "talks/q4")]
#[case("talks/draft.json", "talks/draft")]
#[case("q4", "q4")]
fn deck_label_drops_the_deck_extension(#[case] path: &str, #[case] label: &str) {
    assert_that!(deck_label(std::path::Path::new(path))).is_equal_to(label.to_owned());
}

#[test]
fn once_per_turn_turn_in_two_decks_keeps_the_first_deck() {
    let shared = record(1, "2026-10-30T10:00:00Z", Some("m"), 100, None);
    let a: UsageLedger = [shared.clone()].into_iter().collect();
    let b: UsageLedger = [shared, record(2, "2026-10-30T11:00:00Z", Some("m"), 7, None)].into_iter().collect();
    let decks = vec![("a".to_owned(), a), ("b".to_owned(), b)];
    let kept: Vec<(&str, u128)> = once_per_turn(&decks).iter().map(|(l, r)| (*l, r.turn.as_u128())).collect();
    assert_that!(kept).is_equal_to(vec![("a", 1), ("b", 2)]);
}

#[test]
fn format_table_lines_share_one_width_and_end_with_the_total() {
    let l = Localizer::new(Some("en"));
    let rs = [record(1, "2026-10-30T10:00:00Z", Some("m"), 1234, None), record(2, "2026-10-31T10:00:00Z", Some("m"), 5, None)];
    let text = format_table(&report(&rs, By::Day), &l);
    let lines: Vec<&str> = text.lines().collect();
    assert_that!(lines).has_length(4);
    assert_that!(lines[0]).starts_with(l.tr("usage-col-day").as_str());
    assert_that!(lines[3]).starts_with(l.tr("usage-row-total").as_str());
    let widths: Vec<usize> = lines.iter().map(|x| x.chars().count()).collect();
    assert_that!(widths.windows(2).all(|w| w[0] == w[1])).is_true();
}

#[test]
fn format_table_numbers_are_right_aligned_and_plain() {
    let l = Localizer::new(Some("en"));
    let rs = [record(1, "2026-10-30T10:00:00Z", Some("m"), 1234, None), record(2, "2026-10-31T10:00:00Z", Some("m"), 5, None)];
    let text = format_table(&report(&rs, By::Day), &l);
    let total = text.lines().last().unwrap_or_default();
    assert_that!(total).contains("  1239  ");
}

#[test]
fn format_table_without_model_shows_the_localized_unknown_key_in_french() {
    let l = Localizer::new(Some("fr"));
    let text = format_table(&report(&[record(1, "2026-10-30T10:00:00Z", None, 1, None)], By::Model), &l);
    assert_that!(text.lines().nth(1).unwrap_or_default()).starts_with(l.tr("usage-unknown").as_str());
}

#[test]
fn format_table_row_without_cost_shows_a_placeholder() {
    let l = Localizer::new(Some("en"));
    let text = format_table(&report(&[record(1, "2026-10-30T10:00:00Z", Some("m"), 1, None)], By::Day), &l);
    assert_that!(text.lines().nth(1).unwrap_or_default().trim_end()).ends_with("–");
}

#[test]
fn format_table_partial_cost_uses_the_partial_message() {
    let l = Localizer::new(Some("fr"));
    let rs = [
        record(1, "2026-10-30T10:00:00Z", Some("m"), 1, Some((0.25, "USD"))),
        record(2, "2026-10-30T11:00:00Z", Some("m"), 1, None),
    ];
    let text = format_table(&report(&rs, By::Day), &l);
    let want = l.tr_with("usage-badge-cost-partial", &[("amount", "0.2500".into()), ("currency", "USD".into())]);
    assert_that!(text.lines().last().unwrap_or_default()).contains(want.as_str());
}

#[test]
fn format_table_keeps_currencies_apart_in_one_cell() {
    let l = Localizer::new(Some("en"));
    let rs = [
        record(1, "2026-10-30T10:00:00Z", Some("m"), 1, Some((0.25, "USD"))),
        record(2, "2026-10-30T11:00:00Z", Some("m"), 1, Some((1.5, "EUR"))),
    ];
    let total = format_table(&report(&rs, By::Day), &l).lines().last().unwrap_or_default().to_owned();
    assert_that!(total).contains("0.2500 USD");
    assert_that!(total).contains("1.5000 EUR");
}

#[rstest]
#[case("a\u{1b}[2Jb\u{7}", "a[2Jb")]
#[case("q4\u{202e}fdp.json", "q4fdp.json")]
#[case("a\u{2028}b\u{feff}c\u{200b}", "abc")]
fn printable_drops_characters_core_refuses(#[case] text: &str, #[case] shown: &str) {
    assert_that!(printable(text)).is_equal_to(shown.to_owned());
}

#[test]
fn deck_label_drops_a_bidi_override_of_the_path() {
    assert_that!(deck_label(std::path::Path::new("a\u{202e}b.oku-slides.pdf"))).is_equal_to("ab".to_owned());
}

#[test]
fn usage_issues_keeps_only_the_ledger_issues() {
    let issues = vec![Issue::usage_line_invalid(2), Issue::pack_version_mismatch("abc"), Issue::usage_too_large()];
    let kept: Vec<&str> = usage_issues(&issues).iter().map(|i| i.message.id()).collect();
    assert_that!(kept).is_equal_to(vec!["issue-usage-line-invalid", "issue-usage-too-large"]);
}
```

`usage_issues` keeps the issues whose Fluent id starts with `issue-usage-`; the cost amounts and
the `–` placeholder are `usage_cells`' (Task 7 tests them), these tests only check that the CLI
lays them out.

- [ ] **Step 3: Write the failing integration tests** — `crates/oku-slides-cli/tests/usage.rs`

They run the real binary on real `.oku-slides.pdf` files: `save` writes a deck, then the test puts a
hand-written `usage.jsonl` into its sources zip (an invalid line can only be planted this way, since
`UsageLedger::insert` refuses it).

```rust
//! `oku-slides usage`, run for real on saved decks whose sources hold a hand-written ledger.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{FIXTURE, oku_slides, workdir};
use oku_slides_core::Issue;
use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

fn run(cmd: &mut std::process::Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"))
}

/// One `usage.jsonl` line: `input` tokens on `at`, cost in USD when given.
fn line(turn: u128, at: &str, input: u64, usd: Option<f64>) -> String {
    serde_json::json!({
        "turn": format!("00000000-0000-0000-0000-{turn:012x}"),
        "at": at, "tab": "deck", "agent": "claude", "model": "claude-opus-5-5",
        "tokens": { "total": input + 1, "input": input, "output": 1, "thought": null, "cache_read": null, "cache_write": null },
        "cost": usd.map(|amount| serde_json::json!({ "amount": amount, "currency": "USD" })),
    })
    .to_string()
}

/// `FIXTURE` saved as `<dir>/<name>.oku-slides.pdf`, its sources holding `lines` as `usage.jsonl`.
fn deck_with_usage(dir: &Path, name: &str, lines: &[String]) -> PathBuf {
    let pdf = dir.join(format!("{name}.oku-slides.pdf"));
    let saved = run(oku_slides(dir, "en").args(["save", FIXTURE]).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    let bytes = std::fs::read(&pdf).unwrap_or_else(|e| panic!("lecture du PDF : {e}"));
    let found = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("sources du PDF : {e}"))
        .unwrap_or_else(|| panic!("PDF sans sources"));
    // `save` wrote no ledger (no agent turn): copy every entry, then add `usage.jsonl`.
    let mut src = zip::ZipArchive::new(Cursor::new(found.zip)).unwrap_or_else(|e| panic!("zip : {e}"));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let entry = src.by_index_raw(i).unwrap_or_else(|e| panic!("entrée {i} : {e}"));
        w.raw_copy_file(entry).unwrap_or_else(|e| panic!("copie de l'entrée {i} : {e}"));
    }
    w.start_file("usage.jsonl", zip::write::SimpleFileOptions::default()).unwrap_or_else(|e| panic!("usage.jsonl : {e}"));
    w.write_all(lines.join("\n").as_bytes()).unwrap_or_else(|e| panic!("usage.jsonl : {e}"));
    let zip = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let pdf_bytes = oku_slides_pdf::embed_sources(&bytes, &zip).unwrap_or_else(|e| panic!("embed : {e}"));
    std::fs::write(&pdf, pdf_bytes).unwrap_or_else(|e| panic!("écriture du PDF : {e}"));
    pdf
}

/// `usage --json` of `decks` with `extra` arguments, parsed.
fn usage_json(dir: &Path, decks: &[&Path], extra: &[&str]) -> serde_json::Value {
    let out = run(oku_slides(dir, "en").arg("usage").args(decks).args(extra).arg("--json"));
    assert_that!(out.status.success()).is_true();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("JSON : {e}"))
}

fn keys(report: &serde_json::Value) -> Vec<String> {
    report["rows"].as_array().map_or_else(Vec::new, |rows| rows.iter().map(|r| r["key"].as_str().unwrap_or("?").to_owned()).collect())
}

#[test]
fn usage_by_deck_over_two_decks_keys_rows_by_path_without_extension() {
    let dir = workdir("usage-by-deck");
    let a = deck_with_usage(&dir, "alpha", &[line(1, "2026-10-30T10:00:00Z", 1234, None)]);
    let b = deck_with_usage(&dir, "beta", &[line(2, "2026-10-30T11:00:00Z", 4321, None)]);
    let report = usage_json(&dir, &[&a, &b], &["--by", "deck"]);
    let label = |n: &str| dir.join(n).display().to_string();
    assert_that!(keys(&report)).is_equal_to(vec![label("alpha"), label("beta")]);
    assert_that!(report["totals"]["input"].as_u64()).is_equal_to(Some(5555));
}

#[test]
fn usage_turn_in_two_decks_counts_once() {
    let dir = workdir("usage-dedup");
    let shared = line(1, "2026-10-30T10:00:00Z", 1234, Some(0.25));
    let a = deck_with_usage(&dir, "original", &[shared.clone()]);
    let b = deck_with_usage(&dir, "copie", &[shared, line(2, "2026-10-30T11:00:00Z", 7, None)]);
    let report = usage_json(&dir, &[&a, &b], &[]);
    assert_that!(report["totals"]["turns"].as_u64()).is_equal_to(Some(2));
    assert_that!(report["totals"]["input"].as_u64()).is_equal_to(Some(1241));
}

#[test]
fn usage_since_keeps_only_the_days_from_it() {
    let dir = workdir("usage-since");
    let a = deck_with_usage(&dir, "d", &[line(1, "2026-10-29T23:59:00Z", 1000, None), line(2, "2026-10-30T00:01:00Z", 37, None)]);
    let report = usage_json(&dir, &[&a], &["--since", "2026-10-30"]);
    assert_that!(keys(&report)).is_equal_to(vec!["2026-10-30".to_owned()]);
    assert_that!(report["totals"]["input"].as_u64()).is_equal_to(Some(37));
}

#[test]
fn usage_json_deck_without_ledger_reports_no_turn() {
    let dir = workdir("usage-none");
    let report = usage_json(&dir, &[Path::new(FIXTURE)], &[]);
    assert_that!(report["totals"]["turns"].as_u64()).is_equal_to(Some(0));
    assert_that!(keys(&report)).is_empty();
}

#[test]
fn usage_invalid_line_is_logged_once_and_the_command_succeeds() {
    let dir = workdir("usage-invalid");
    let a = deck_with_usage(&dir, "d", &[line(1, "2026-10-30T10:00:00Z", 12, None), "{pas du json}".to_owned()]);
    let out = run(oku_slides(&dir, "en").arg("usage").arg(&a).arg("--json"));
    assert_that!(out.status.success()).is_true();
    let issue = Issue::usage_line_invalid(2).localize(&Localizer::new(Some("en")));
    assert_that!(String::from_utf8_lossy(&out.stderr).matches(issue.as_str()).count()).is_equal_to(1);
}

#[test]
fn usage_logs_no_widget_issue_of_the_deck() {
    // Read with an empty registry, every widget of the fixture is unknown: none of that is logged.
    let dir = workdir("usage-quiet");
    let a = deck_with_usage(&dir, "d", &[line(1, "2026-10-30T10:00:00Z", 12, None)]);
    let out = run(oku_slides(&dir, "en").arg("usage").arg(&a).arg("--json"));
    assert_that!(out.status.success()).is_true();
    let prefix = Localizer::new(Some("en")).tr_with("cli-deck-issue", &[("issue", "".into())]);
    assert_that!(String::from_utf8_lossy(&out.stderr).contains(prefix.trim_end())).is_false();
}

#[test]
fn usage_since_after_until_fails_without_a_report() {
    let dir = workdir("usage-range");
    let out = run(oku_slides(&dir, "fr").args(["usage", FIXTURE, "--since", "2026-10-31", "--until", "2026-10-30"]));
    assert_that!(out.status.success()).is_false();
    assert_that!(out.stdout.is_empty()).is_true();
}

#[test]
fn usage_unreadable_day_fails_without_a_report() {
    let dir = workdir("usage-day");
    let out = run(oku_slides(&dir, "en").args(["usage", FIXTURE, "--since", "30/10/2026"]));
    assert_that!(out.status.success()).is_false();
    assert_that!(out.stdout.is_empty()).is_true();
}

#[test]
fn usage_table_in_french_has_localized_headers_and_total() {
    let dir = workdir("usage-table");
    let a = deck_with_usage(&dir, "d", &[line(1, "2026-10-30T10:00:00Z", 4321, Some(0.4321))]);
    let out = run(oku_slides(&dir, "fr").arg("usage").arg(&a));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let fr = Localizer::new(Some("fr"));
    assert_that!(text.lines().next().unwrap_or_default()).starts_with(fr.tr("usage-col-day").as_str());
    let total = text.lines().last().unwrap_or_default().to_owned();
    assert_that!(total).starts_with(fr.tr("usage-row-total").as_str());
    assert_that!(total).contains("4321");
    assert_that!(total).contains("0.4321 USD");
}
```

The localized range errors themselves (key + a non-English locale) are tested in core (Task 3);
here only the command's behavior is (fails, prints no report). `ZipArchive::by_index_raw`,
`ZipWriter::raw_copy_file` and `zip::write::SimpleFileOptions` are zip 9.0.4's (the workspace pin).

- [ ] **Step 4: Run the tests to see them fail** — `cargo nextest run -p oku-slides-cli usage`.
Expected: compile errors (`usage_cmd` not found, no `usage` subcommand).

- [ ] **Step 5: Implement `usage_cmd.rs`**

```rust
//! `oku-slides usage`: the agent usage recorded in decks, summed.
//!
//! Each deck is read without rendering (no pack set, so an empty widget registry and no render
//! worker); only its ledger issues are logged, once. A turn held by several decks (a copy, a merge)
//! counts once, for the first deck given. [`aggregate`] makes the only report; the table's headers
//! and cells are the `show_usage` box's ([`usage_headers`], [`usage_cells`], [`usage_total_cells`]).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use color_eyre::eyre::{Result, WrapErr, eyre};

use oku_slides_core::{By, Issue, Range, SourceLimits, UsageLedger, UsageRecord, UsageReport, aggregate, shown_char};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_session::{EXTENSION, USAGE_COLUMNS, read_document, usage_cells, usage_headers, usage_total_cells};

use crate::out::Out;

/// What `--by` groups the rows by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CliBy {
    Day,
    Month,
    Model,
    Agent,
    Tab,
    Deck,
}

impl From<CliBy> for By {
    fn from(by: CliBy) -> Self {
        match by {
            CliBy::Day => By::Day,
            CliBy::Month => By::Month,
            CliBy::Model => By::Model,
            CliBy::Agent => By::Agent,
            CliBy::Tab => By::Tab,
            CliBy::Deck => By::Deck,
        }
    }
}

/// The `usage` command's arguments, as parsed by clap.
#[derive(Debug)]
pub(crate) struct UsageArgs {
    pub decks: Vec<PathBuf>,
    pub by: CliBy,
    pub since: Option<String>,
    pub until: Option<String>,
    pub json: bool,
}

/// `usage`: prints the summed ledgers of `args.decks`, as a table or JSON.
pub(crate) fn usage(
    platform: &Platform,
    out: &mut Out<'_>,
    app_fonts: &Fonts,
    limits: SourceLimits,
    args: UsageArgs,
) -> Result<()> {
    let l = out.localizer();
    let range = Range::parse(args.since.as_deref(), args.until.as_deref()).map_err(|e| eyre!(e.localize(l)))?;
    // No pack set: `registry` returns the empty registry, without probing or the render worker.
    let registry = crate::registry(app_fonts)?;
    let mut decks = Vec::with_capacity(args.decks.len());
    for path in &args.decks {
        let mut opened = read_document(&platform.storage, path, limits, &registry)
            .wrap_err_with(|| l.tr_with("cli-reading", &[("path", printable(&path.display().to_string()).into())]))?;
        crate::log_issues(l, &usage_issues(&opened.issues));
        decks.push((deck_label(path), std::mem::take(&mut opened.sources.usage)));
    }
    let records = once_per_turn(&decks);
    let report = aggregate(records.iter().copied(), args.by.into(), range);
    let text = if args.json { serde_json::to_string_pretty(&report)? } else { format_table(&report, l) };
    out.data(&text)?;
    Ok(())
}

/// The ledger issues among `issues`: the others (unknown widgets, packs) are noise here.
pub(crate) fn usage_issues(issues: &[Issue]) -> Vec<Issue> {
    issues.iter().filter(|i| i.message.id().starts_with("issue-usage-")).cloned().collect()
}

/// A deck's label: its path without `.oku-slides.pdf`, or without the extension of a JSON deck.
///
/// The path comes from the file system: [`printable`] drops what a terminal would interpret.
pub(crate) fn deck_label(path: &Path) -> String {
    let shown = path.display().to_string();
    let label = match shown.strip_suffix(EXTENSION) {
        Some(stem) => stem.to_owned(),
        None => path.with_extension("").display().to_string(),
    };
    printable(&label)
}

/// Every record of `decks` with its deck label; a turn already seen in an earlier deck is skipped.
pub(crate) fn once_per_turn(decks: &[(String, UsageLedger)]) -> Vec<(&str, &UsageRecord)> {
    let mut seen = BTreeSet::new();
    decks
        .iter()
        .flat_map(|(label, ledger)| ledger.records().map(move |r| (label.as_str(), r)))
        .filter(|(_, r)| seen.insert(r.turn))
        .collect()
}

/// The report as an aligned table: header, one line per row, then the totals.
///
/// The key column is left-aligned, the others right-aligned; columns are two spaces apart and
/// every line has the same width (counted in characters).
pub(crate) fn format_table(report: &UsageReport, l: &Localizer) -> String {
    let mut lines = vec![usage_headers(report.by, l)];
    lines.extend(report.rows.iter().map(|row| usage_cells(row, l)));
    lines.push(usage_total_cells(&report.totals, l));
    let widths: Vec<usize> =
        (0..USAGE_COLUMNS).map(|c| lines.iter().map(|line| line[c].chars().count()).max().unwrap_or(0)).collect();
    lines.iter().map(|line| aligned(line, &widths)).collect::<Vec<_>>().join("\n")
}

fn aligned(line: &[String; USAGE_COLUMNS], widths: &[usize]) -> String {
    let cell = |(i, (text, width)): (usize, (&String, &usize))| {
        if i == 0 { format!("{text:<width$}") } else { format!("{text:>width$}") }
    };
    line.iter().zip(widths).enumerate().map(cell).collect::<Vec<_>>().join("  ")
}

/// `text` without the characters core's [`shown_char`] refuses, safe to print on a terminal.
pub(crate) fn printable(text: &str) -> String {
    text.chars().filter(|&c| shown_char(c)).collect()
}

#[cfg(test)]
#[path = "usage_cmd_tests.rs"]
mod usage_cmd_tests;
```

`std::mem::take` needs `UsageLedger: Default` (Task 1 derives it). `registry` and `log_issues`
stay private in `main.rs`: a child module reaches them as `crate::…`.

- [ ] **Step 6: Wire it in `main.rs`**

`mod usage_cmd;` (alphabetical, after `sse`), `use crate::usage_cmd::{CliBy, UsageArgs};`, a `Cmd`
variant after `Schema`:

```rust
    Usage {
        #[arg(required = true)]
        decks: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = CliBy::Day)]
        by: CliBy,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        json: bool,
    },
```

in `command`, after `schema`:

```rust
        .mut_subcommand("usage", |c| {
            c.about(help("cli-cmd-usage"))
                .mut_arg("decks", |a| a.help(help("cli-arg-usage-decks")))
                .mut_arg("by", |a| a.help(help("cli-arg-usage-by")))
                .mut_arg("since", |a| a.help(help("cli-arg-usage-since")))
                .mut_arg("until", |a| a.help(help("cli-arg-usage-until")))
                .mut_arg("json", |a| a.help(help("cli-arg-usage-json")))
        })
```

and in `run`, after `Cmd::Schema`:

```rust
        Cmd::Usage { decks, by, since, until, json } => {
            usage_cmd::usage(platform, out, &app_fonts, limits, UsageArgs { decks, by, since, until, json })?;
        }
```


- [ ] **Step 7: Run the tests** — `cargo nextest run -p oku-slides-cli usage` and
`cargo nextest run -p oku-slides-i18n`. Expected: PASS. Then by hand, in a fresh temp data dir:

```bash
OKU_SLIDES_PATHS__DATA="$(mktemp -d)" OKU_SLIDES_PATHS__DECKS="$(mktemp -d)" just oku-slides usage --help
```

Expected: the six help lines in the system locale.

- [ ] **Step 8: Docs** — `crates/oku-slides-cli/ARCHITECTURE.md`: replace the `bench.rs` bullet with
one that also names the new module, no other line added:

```
- `bench.rs`: render, export and save latencies. `usage_cmd.rs`: `oku-slides usage`, decks' ledgers
  (read without rendering, a turn counted once) summed by core `aggregate`, as a table or `--json`.
```

Check `wc -l crates/oku-slides-cli/ARCHITECTURE.md` ≤ 50.

- [ ] **Step 9: Commit** — stage `crates/oku-slides-cli crates/oku-slides-i18n` (separate `git add`,
then `git diff --cached --stat` right before committing); message file in the scratchpad:
`feat(cli): oku-slides usage sums decks' agent usage (t-1303, t-1296)`.

### Task 9: End-to-end

**Files:**
- Modify: `fixtures/agent/e2e.json` (`by_prompt`)
- Create: `ui/e2e/290-usage.cjs` (290 is the next free number after `280-coller-autre-fenetre.cjs`)

**Interfaces:**
- Consumes: the fake agent's scripted `usage: [total, input, output]` and `cost: {amount, currency}` (Task 6); the fake
  sends `cost.amount` **verbatim** as the session's cumulative `UsageUpdate.cost`, so a turn's recorded cost is its
  amount minus the previous cumulative amount of the session; `show_usage` (Task 7) answering `{"shown": "<surface id>"}`;
  the box's GFM table (Task 7, the same nine columns as `usage_headers`): key, turns, input, output, reasoning,
  cache read, cache write, total, cost; last row the totals; French headers `Tours`, `Entrée`, `Coût` (session.ftl).
- Produces: nothing.

Design notes (from the review): earlier specs already ran turns on `t-1` (without usage), and a
resumed session's first cost-bearing turn records `cost: null` (Task 6). So the spec never asserts
absolute values: it plays a first cost-bearing turn whose cumulative cost is 0.1 (it records 0.1, or
`null` on a resumed session: either way the base is now 0.1), reads the box once (A), plays a second
prompt whose cumulative cost is 0.4321, reads the box again (B), and asserts **deltas** of the totals
row between A and B: 4321 input tokens and 0.4321 − 0.1 = 0.3321 USD, numbers no other spec produces.
Two prompts with different cumulative amounts are needed: the same amount twice would make the
counted turn cost 0. Between A and B two turns are recorded: A's own `show_usage` turn (no tokens,
no cost) and the counted one. Columns are found by their header text, never by index, and a turn is
awaited by counting the thread's turns before the prompt, then waiting for that `nth` one, so a
reply of an earlier identical prompt never satisfies the wait.

- [ ] **Step 1: Script the prompts** in `fixtures/agent/e2e.json` `by_prompt`:

```json
"pose la base de coût": { "chunks": ["Base posée."], "usage": [12, 10, 2], "cost": { "amount": 0.1, "currency": "USD" } },
"compte les tokens": { "chunks": ["Compté."], "usage": [5432, 4321, 1111], "cost": { "amount": 0.4321, "currency": "USD" } },
"combien a coûté ce deck": { "mcp": [{ "tool": "show_usage", "args": { "by": "tab" } }], "chunks": ["Voici l'usage."] }
```

- [ ] **Step 2: Write `ui/e2e/290-usage.cjs`** (skeleton of `240-agent-themes.cjs`: `errors`
collected from `pageerror` and console errors, asserted empty at the end, `console.log` then
`b.close()`, exit 1 on failure)

```js
// Agent usage: a counted turn (the fake agent's tokens and cost) enters the deck's ledger, and the
// thread's `show_usage` box shows it. Other specs already played turns on t-1, so only the deltas
// of the totals row between two boxes are checked.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", log in E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince, waitForTool } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
/** French column headers (session.ftl `usage-col-turns`, `-input`, `-cost`, Task 7). */
const HEAD = { turns: 'Tours', input: 'Entrée', cost: 'Coût' };

const prompt = (p, text) => post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text, ui: {} });
/** A cell's number: digits and dot kept, 0 for "–". */
const num = (text) => Number(text.replace(/[^\d.]/g, '') || '0');
/** The thread's turns holding `text`. */
const turnsOf = (p, text) => p.locator('.agent-turn', { hasText: text });

/** Plays a prompt to the end of its turn: its reply shows in the thread, in a new turn. */
async function turn(p, text, reply) {
  const before = await turnsOf(p, text).count();
  const from = logOffset(LOG);
  await prompt(p, text);
  await until(p, async () => logSince(LOG, from).some((m) => m.method === 'session/prompt'), `"${text}" must reach the fake agent`, 15000);
  await turnsOf(p, text).nth(before).getByText(reply).waitFor();
}

/** Asks for the usage box; returns its totals row as `{ column header: cell }` and whether it fits. */
async function usageBox(p) {
  const asked = 'combien a coûté ce deck';
  const before = await turnsOf(p, asked).count();
  const from = logOffset(LOG);
  await prompt(p, asked);
  const result = await waitForTool(p, from, 'show_usage');
  const { shown } = JSON.parse(result.content[0].text);
  const box = p.locator(`[data-surface-id="${shown}"]`);
  await box.locator('table').waitFor();
  await turnsOf(p, asked).nth(before).getByText("Voici l'usage.").waitFor();
  const head = await box.locator('table th').allInnerTexts();
  const cells = await box.locator('table tr').last().locator('td').allInnerTexts();
  assert.equal(cells.length, head.length, `totals row: ${cells.join(' | ')}`);
  const totals = Object.fromEntries(head.map((h, i) => [h.trim(), cells[i]]));
  for (const h of Object.values(HEAD)) assert.ok(h in totals, `missing column ${h}: ${head.join(' | ')}`);
  const fits = await box.evaluate((el) => {
    const list = el.closest('.agent-turns');
    return list === null || list.scrollWidth <= list.clientWidth;
  });
  return { totals, fits };
}

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');

  // 1. A first cost-bearing turn: the session's cost base is now 0.1, resumed or not.
  await turn(p, 'pose la base de coût', 'Base posée.');

  // 2. First box: the starting totals.
  const a = await usageBox(p);

  // 3. The counted turn (cumulative cost 0.4321), then the second box.
  await turn(p, 'compte les tokens', 'Compté.');
  const bx = await usageBox(p);

  // 4. Deltas: box A's own turn (no tokens) and the counted one; 4321 input, 0.4321 - 0.1 USD.
  const delta = (h) => num(bx.totals[h]) - num(a.totals[h]);
  const shown = `${JSON.stringify(a.totals)} -> ${JSON.stringify(bx.totals)}`;
  assert.equal(delta(HEAD.turns), 2, `turns: ${shown}`);
  assert.equal(delta(HEAD.input), 4321, `input: ${shown}`);
  assert.ok(bx.totals[HEAD.cost].includes('USD'), `cost: ${shown}`);
  assert.ok(Math.abs(delta(HEAD.cost) - 0.3321) < 1e-9, `cost: ${shown}`);

  // 5. The nine-column table scrolls inside its box: the thread does not widen.
  assert.ok(bx.fits, 'the usage box widens the thread');

  assert.deepEqual(errors, []);
  console.log('usage : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

`num` reads only the first currency's amount: no other spec sends a cost, so the totals row holds
USD alone. If the cost cell ever holds several currencies, the assertion fails loudly rather than
summing them. Amounts are printed with four decimals; the `1e-9` tolerance only absorbs the float
subtraction.

- [ ] **Step 3: Run** — `just e2e 4999` in the background (logs under
`.superpowers/sdd/2026-10-30-deck-usage-ledger/`; the port must be free). Expected: every spec
passes, `usage : ok` among them. If 290 fails, read `E2E_AGENT_LOG` (the fake agent's log) before
changing anything.

- [ ] **Step 4: Commit** — stage `fixtures/agent/e2e.json ui/e2e/290-usage.cjs`:
`test(ui): e2e of the agent usage box (t-1304, t-1296)`.

### Task 10: Final gate

- [ ] **Step 1: Full gate** — `just ci` in the background with `nohup`, log in
`.superpowers/sdd/2026-10-30-deck-usage-ledger/ci.log` (nextest with `--retries 2` under load if a
flaky timing test fails). Expected: fmt, clippy `-D warnings`, tsc, fonts-lfs, agents-lint,
commit-lint, nextest, UI tests, audit all green.

- [ ] **Step 2: The app crate** — `cargo check -p oku-slides-app` (nextest excludes it; it holds
`AppSessions`, the second `Sessions` implementation). Expected: no error, no warning.

- [ ] **Step 3: Consolidation pass (G10)** — each must print nothing beyond the one expected hit:

```bash
rg -n "fn aggregate\b" crates
rg -n "fn md_escape\b" crates
rg -n "Date::parse|format_description!" crates/oku-slides-mcp crates/oku-slides-cli crates/oku-slides-session crates/oku-slides-agent
rg -n "deny_unknown_fields" crates/oku-slides-core/src/usage.rs
rg -n "time\." crates/oku-slides-mcp/Cargo.toml crates/oku-slides-cli/Cargo.toml
rg -n "fn usage_cells\b|fn usage_headers\b|fn usage_total_cells\b" crates
rg -n "usage-col-" crates --glob '*.rs'
```

Expected: one `aggregate` (core `usage_report.rs`), one `md_escape` (session `usage_panel.rs`), no
date parsing outside core, no `deny_unknown_fields` in the ledger, no `time` dependency in mcp or cli,
one each of `usage_cells`, `usage_headers`, `usage_total_cells` (session `agent/usage_table.rs`), and `usage-col-` keys
named only in `usage_table.rs` (the CLI and the box hold no column list of their own).

- [ ] **Step 4: Size and docs** — every `ARCHITECTURE.md` touched stays ≤ 50 lines; a source file
this branch **creates** stays ≤ 500 lines, one it **modifies** ≤ 1000 (several hosts, such as
`resume.rs`, were past 500 before this branch; the new code went into new files):

```bash
for f in crates/*/ARCHITECTURE.md; do [ "$(wc -l < "$f")" -le 50 ] || echo "too long: $f"; done
git diff --name-only --diff-filter=A main... -- '*.rs' | while read -r f; do [ "$(wc -l < "$f")" -le 500 ] || echo "new, over 500: $f"; done
git diff --name-only --diff-filter=M main... -- '*.rs' | while read -r f; do [ "$(wc -l < "$f")" -le 1000 ] || echo "modified, over 1000: $f"; done
```

Expected: no output. Then set the spec's Status line to `implemented 2026-MM-DD (t-1296).`, and
check that the rules this branch added pass `just agents-lint`:

- `crates/oku-slides-core/AGENTS.md` (Task 2): the bundle's usage ledger is add-only (union on
  merge) except that `[usage] store_in_deck = false` drops it on save (Task 5's exception), and usage
  stays out of `Deck` and the replica, read last under its own cap;
- `crates/oku-slides-session/AGENTS.md` (Task 4): usage is recorded only in a deck tab's own
  document (the stash in history), never sets dirty, and is snapshotted at once.

- [ ] **Step 5: README** — `README.md`, four insertions, no new section:

  - **Command-line tool** code block, after the `schema` line:

    ```sh
    just oku-slides usage deck.oku-slides.pdf --by model --since 2026-10-01  # agent tokens and cost recorded in decks
    ```

    and, after the `--fonts` paragraph:

    ```markdown
    `usage` sums the agent usage recorded in each deck (`--by day|month|model|agent|tab|deck`, UTC
    days, `--since` and `--until` inclusive, `--json` for the raw report); a turn found in several
    decks (a copy, a merge) counts once. Costs are the agent's own, never summed across currencies.
    ```

  - **Agent** bullet list, after **Detached window**:

    ```markdown
    - **Usage**: each turn's tokens, model, agent and cost (when the agent reports them) are
      recorded in the deck file itself, never marking it modified; ask the agent what the deck
      cost, or run `oku-slides usage`. `[usage] store_in_deck = false` records nothing and saves
      decks without it.
    ```

  - **Settings** table, after the `[agent]` row:

    ```markdown
    | `[usage]` | `store_in_deck`: record agent usage in each deck (default `true`); `false` also drops it at the next save |
    ```

  - **Environment variables** table, after `OKU_SLIDES_AUTHOR__NAME`:

    ```markdown
    | `OKU_SLIDES_USAGE__STORE_IN_DECK` | `false`: no agent usage recorded in decks, and none kept at the next save |
    ```

- [ ] **Step 6: Sub-tasks** — each sub-task was marked done as its task's commit landed (t-1297 …
t-1304 and t-1306, Task 5's); confirm with `oku cli task show t-1296 --project slider` that only t-1305
is open, then mark t-1305 done. t-1296 stays open until the branch is merged into `main`.

- [ ] **Step 7: Commit** — the README and any fix-ups: `docs: usage ledger in the README, consolidation (t-1305, t-1296)`
(stage `README.md` and the fixed files with a separate `git add`, check `git diff --cached --stat`).
