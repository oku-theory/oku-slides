# Lot H hardening — implementation plan

Parent task: t-1440 (one sub-task per step below).

**Goal:** close the deferred hardening findings of batches B and lot A: t-1379, t-1380, t-1381,
t-1382, t-1383, t-1376.

**Constraints:** root, `crates/`, i18n, core, session, platform, stt and ui `AGENTS.md`; TDD, each
fix with a regression test (`tests/regressions/issue_NNNN_*.rs`, or a side-by-side UI test); the
Rust and TS character sets stay identical; one squash commit citing every ticket.

## Task 1 — t-1379: line separators and C0 controls in interpolated arguments

`strip_hidden` / `stripHidden` become `clean_arg` / `cleanArg`: hidden controls removed as before,
and every `Cc` character (U+0000..U+001F, U+007F..U+009F: LF, CR, TAB, ESC, U+0085) plus U+2028 and
U+2029 mapped to a space, as `agent::bounds::label` does. Audit: no interpolated argument needs a
newline. Multi-line texts (render diagnostics) are appended after formatting
(`RenderError::localize`); nested in another message's `$reason` they now read on one line, which
is what a notice or a terminal line wants anyway.

## Task 2 — t-1380 (UI half): isolate every string argument in `t()`

Not done: left open for the user. Wrapping each string value in FSI/PDI (`safeArg`, as the ticket
suggests) was tried and reverted: a selector on a string variable (`ui-field`, `ui-element-kind`:
`{ $kind -> [kicker] … }`) no longer matches, and `fieldLabel` compares `t()` output with the raw
field name. A correct version isolates placeables, not values (`useIsolating: true`, which wraps
numbers too and changes every e2e text; or rewriting each parsed pattern's variable placeables
into an `ISOLATE($var)` call that wraps strings only), and still needs every caller that compares
or stores `t()` output audited. The Rust half has its own question: the platform localizer serves
the UI, the agent and CLI-hosted sessions alike, and marks in tool errors reach the model, whose
echo `check_text` rejects.

Decided and done in t-1380: placeable rewrite in the UI, Rust stays raw
(`docs/superpowers/specs/2026-11-01-bidi-isolation-design.md`).

## Task 3 — t-1383: request strings in session errors, save dialog name, hidden set, tick

- `Display` of `UnknownWindow`, `NoSlide`, `NoChart`, `NoTable`, `UnknownOwner`, `UnknownField`,
  `FieldNeeded`, `HostKey`, `UnknownAsset`, `UnknownTheme`, `InvalidWidgetName`, `UnknownWidget`,
  `TrialUndeclared`, `TrialFixed` cleans and clips its request strings with `bounds::label`; their
  localized messages pass the same labels (`tr_with` cleans but does not clip).
- `pickSavePath` proposes a name cleaned like an argument, path separators replaced by `-`.
- `is_hidden_control` / `HIDDEN` gain the Hangul fillers U+3164 and U+FFA0 (no text needs them).
  Variation selectors stay out: U+FE0F selects emoji presentation, which agent text and names
  carry; U+115F and U+1160 stay out too (conjoining jamo sequences use them). `label` still
  removes all of them.
- `stt.tick_ms` is a `NonZeroU64`: 0 is refused when the settings load.

## Task 4 — t-1381: caps on widget parameter names, choice values and string defaults

`check_widget` refuses (`SchemaFault::TooLong`, its message widened in all 20 locales to cover
names, values and lists) a parameter, column or `bind` name over `PARAM_NAME_MAX` (64), more than
`SCHEMA_LIST_MAX` (100) choice values or slot `accepts`, one of them over `WIDGET_LABEL_MAX`, and a
string default over `PLAIN_MAX` bytes. No shipped pack comes near. A user pack already embedded
in a deck past these caps is now set aside on load, like any refused widget.
`WidgetRegistry::from_probe` qualifies each widget's name before `check_widget`, so `assemble_set`
never blames a same-named widget of another pack; its bare-name branch goes.

## Task 5 — t-1382: resume entry counts and one read budget per tab

`read_files` (moved to `resume_read.rs`) charges each listed entry at least `ENTRY_BYTES`
(4 KiB), hidden ones included: `max_sources_mib` then also caps the entry count (65 536 at the
default); past it, `ResumeError::BudgetExhausted` sets the tab aside. Pack files are grouped by hex
in one pass (`group_packs`) instead of a scan per manifest. `read_doc` takes a `ReadBudget` made
once per tab restore and shared by its documents (`restore_pack_tab` reads two). The budget is
crate-private, so the tests are unit tests (`resume_read_tests.rs`, `resume_tests.rs`), as for
t-1313. Known edge: a pack tab's pack and preview both carry the pinned packs, now charged twice,
and the per-entry charge means a deck right at `max_sources_mib` may not resume; the write side
does not check the limit yet (follow-up t-1472).

## Task 6 — t-1376: cap each model file's size

`PinnedFile.size`: the file's size when pinned, an upper bound otherwise (STT: 16 MiB per small
file, twice the model's expected total for `model.bin`). A `.part` longer than it is never resumed;
one exactly at it (a pinned file cut before its rename) still finishes through the 416 path. An
announced length or a body past it stops the download (`TooLarge`, kind `Integrity`: the `.part`
is deleted, as for a wrong hash). The scripted hub is crate-private: unit tests in
`download_tests.rs`.

## Task 7 — review round and `just ci`

One independent review round (rust picky reviewer): fixed formatting, the remaining raw request
strings in session errors and their localized arguments, the budget error's name
(`BudgetExhausted`), a `read_doc`-level shared-budget test, `saveName` trimming, clipping and
fallback, doc first sentences. `AGENT_MODEL_SIZE` checked against the hub's `X-Linked-Size`.
