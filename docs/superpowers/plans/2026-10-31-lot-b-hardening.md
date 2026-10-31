# Batch B: hardening and small robustness fixes (t-959, t-981, t-965, t-969, t-1142, t-1206, t-1075, t-1090)

**Parent task:** t-1343. **Sub-tasks:** t-1344 to t-1351, one per task below, in order.

**Goal:** Close seven small hardening tickets in one squash commit. One task per ticket (t-959 and
t-981 share one), then a final review and gates task.

## Task 1: Hidden controls stripped from every interpolated Fluent value (t-959, t-981)

Shared approach: strip at interpolation, in the one place each layer formats messages. Both
bundles run without isolation marks (`set_use_isolating(false)`, `useIsolating: false`), and a
name (imported theme, received pack, agent command) reaches dozens of messages; fixing call sites
one by one would miss some.

- Move `is_hidden_control` (bidi marks, embeddings, overrides, isolates, U+061C, zero-width
  space, word joiners, BOM, U+180E, soft hyphen, tag characters; ZWJ and ZWNJ stay) from
  `session/src/agent/bounds.rs` to `oku-slides-i18n` as `pub fn is_hidden_control`; `bounds`
  reuses it (one implementation).
- `Localizer::tr_with`: every `FluentValue::String` argument loses its hidden controls.
- UI `i18n.ts`: `stripHidden` with the same set; `t()` applies it to string arguments. A value
  wrapped by `isolate()` keeps its one FSI/PDI pair (its interior is stripped).
- Tests: i18n unit tests (`hidden_tests.rs`, `tr_with` in French: RLO/LRI/ZWSP stripped, ZWJ kept);
  regression `session/tests/regressions/issue_0981_pack_name_bidi.rs` (pack undo label);
  `i18n.test.ts` (string args stripped, isolate kept, numbers untouched).

## Task 2: Widget label and description length capped (t-965)

- `SchemaFault::TooLong` (`widget-fault-too-long`, 20 locales). `check_widget` refuses a widget
  label, param or column label, or `item-label` over `WIDGET_LABEL_MAX` (120 chars), a description
  over `WIDGET_DESCRIPTION_MAX` (500 chars).
- Regression `core/tests/regressions/issue_0965_widget_text_too_long.rs`.

## Task 3: Widget probe checks each schema with serde (t-969)

- `widget_probe::probe_file`: after the name check, `serde_json::from_value::<WidgetSchema>`; a
  failure sets that file aside with a diagnostic; the other files keep their schema.
- Regression `render/tests/regressions/issue_0969_*`: a hand-written schema dictionary with a
  mistyped field is set aside alone.

## Task 4: Apple finish timeout without a panic (t-1142)

- `Stt::apple_finish_timeout()`: `Duration::try_from_secs_f32`, else `ZeroSetting`
  (`stt.apple_finish_timeout_secs`), as t-1140 did for `api_timeout_secs`. `begin_recording`
  checks it before opening the stream (reported, back to idle); `transcribe_stream` returns it.
- Tests in `dictation_stream_tests.rs` (they need the test-only scripted engine, which `tests/`
  cannot reach): a negative, NaN or infinite value reports an error and no thread panics; same
  for `transcribe_samples` (`Model` error).

## Task 5: Bounded reads in the resume (t-1206)

- `read_doc` and `deck_unsaved` read `document.json` with `read_limited(MAX_JSON_BYTES)`.
- `read_doc` takes `SourceLimits`: `deck.loro` under `replica_bytes`, assets, fonts, packs and
  themes under one `total_bytes` budget (as for a PDF's sources).
- `UnknownTab` displays its id clipped (`bounds::clip`, `LABEL_MAX`) in logs.
- The app-exit close guard (ticket comment) is a Tauri/UI behavior, not a bounded read: split into
  t-1377.
- Tests: `Resume` is crate-private, so the bounded reads are tested in `resume_tests.rs`
  (`issue_1206_*`); the `UnknownTab` clip in `tests/regressions/issue_1206_unknown_tab_id_clipped.rs`.

## Task 6: Theme previews never blame a pack (t-1075)

- Checked, then closed without a code change. Theme tokens are unchecked JSON that pack code
  reads (`ctx.tokens`), so a theme can make a preview exceed its bounds. But the worker blames a
  pack only when it is alone in the slot (`client.rs` `blame`: `[pack]`), and the core refuses to
  remove `base` (`every deck keeps the base widget pack`): every preview slot holds `base` beside
  the deck's packs, so a theme preview never blames one. Variants go through the session's own
  fallback (`with_fallback`), whose control render with the never-blamed packs alone takes the
  theme layer too. A regression written to show the blame passed without any fix; dropped.

## Task 7: Typed A2UI button builder (t-1090)

- `Component::button_builder(id, label, action) -> ButtonBuilder` with `primary`, `danger`, `icon`,
  `build`; `Component::danger()` and `Component::icon()` are removed; their 5 call sites move to
  the builder. `ComponentKind` and the catalog are unchanged (narrow: the A2UI playbook session
  touches the catalog).

## Task 8: Final review and gates

- Multi-angle review of the diff (Rust picky and security reviewers); fixes; `just ci`; `just e2e`
  (UI `t()` changes).
- Review fixes: `safeArg` keeps every balanced FSI/PDI pair (a joined list of isolated titles,
  `closeFlow.ts`, kept one isolate per title); `UnknownTab` displays its id through
  `agent::bounds::label` (no line break in a log line); a zero `apple_finish_timeout_secs` is
  refused too.
- Kept: the resume reads use the same `total_bytes` budget as reopening the deck's own PDF; a
  resume over it is quarantined (kept on disk, notice shown), not deleted.
- Split into follow-ups: t-1379 (line separators and C0 controls in arguments), t-1380 (isolate
  every argument), t-1381 (cap param names, choice values, defaults), t-1382 (resume entry
  counts, one budget per resume, no size check when assets are added), t-1383 (other raw request
  strings in error messages, save dialog name).
- One squash commit on `main`, every ticket cited; close the tickets.
