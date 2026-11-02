# Bidi isolation of string arguments in `t()` — design (t-1380)

## Problem

Both Fluent bundles run with isolation off. `cleanArg` / `clean_arg` (t-959, t-981, t-1379) remove
bidi overrides and line controls from arguments, but the strong right-to-left letters of an Arabic or
Hebrew value still reorder the neutral characters and digits around it in the sentence
(`Supprimer « שלום » ?` shows its punctuation out of place).

Wrapping each string *value* in FSI/PDI inside `safeArg` was tried and reverted: a selector on a
string variable (`ui-field`, `ui-element-kind`: `{ $kind -> [kicker] … }`) no longer matched.

## Decision

- **UI (`ui/src/i18n.ts`)**: isolate *placeables*, not values. When messages are installed, every
  `{ $var }` placeable of every message and term value (including the patterns of select
  variants; attributes are skipped, `t()` never formats them) becomes a call to a bundle function `ISOLATE($var)`. Selectors keep the bare variable, so they
  still match the raw value.
- `ISOLATE` wraps a string in one FSI and one PDI; it returns numbers, dates and a missing variable
  unchanged (so `NUMBER($value)` and plurals keep working). A string that already is exactly one
  isolate (an `isolate()` result passed as an argument) is returned as it is, never nested.
- **Rust (`Localizer::tr_with`)**: stays raw (option R0). One platform localizer serves the UI, the
  agent (MCP tool errors), the CLI and the render worker; marks echoed by the agent are refused by
  `check_text` (HiddenCharacters) and the terminal must not print them. Localized Rust errors shown
  in the UI keep `clean_arg` only. A regression test pins this.

Rejected: `useIsolating: true` (wraps numbers, message references and select results too, far more
text churn); isolating values (breaks selectors); an isolating Rust localizer at UI boundaries
(about 30 boundaries, marks would leak into agent-visible panels and history).

## Callers

- **Text sent to the host** must stay raw: `tPlain(id, args)` formats like `t()` and removes FSI/PDI.
  Used by the undo labels stored in the replica history (`ui-undo-new-slide`, `ui-undo-add-widget`,
  `ui-undo-list-move`), which the agent can read, and the default chart series and table column
  names written into the deck.
- **Values that read as one unit** go in one argument: `ui-pasted-image` takes `$size`
  (`1200×800`), since two isolates would swap width and height in a right-to-left interface.
- **`fieldLabel`** compares the translation with the raw field name; it compares `tPlain` output.
- **Manual `isolate()` calls** around single values (`close.ts`, `closeFlow.ts`) are removed: `t()`
  now does it. A joined list keeps one `isolate()` per item (`closeFlow.ts`
  `ui-confirm-close-writes-pending-linked`, and now `GuidesInstallDialog`'s required names);
  `safeArg` already keeps balanced pairs, so the list is isolated as a whole and item by item.
- Display-only strings (text, `aria-label`, `title`, native dialogs in `merge.ts`) simply gain marks.

## Testing

- `ui/src/i18n.test.ts`: string placeable isolated; number unchanged; selector on a string still
  matches and its printed `*[other]` value is isolated; `NUMBER()` unchanged; an already isolated
  argument is not nested; `tPlain` has no marks; an RTL value regression (t-1380).
- `crates/oku-slides-i18n/tests/regressions/issue_1380_raw_localizer.rs`: `tr_with` adds no FSI/PDI.
- Existing unit, render and e2e tests that read interpolated strings are updated to the isolated
  text (e2e: undo `aria-label`, pasted image size).
- Rules: `crates/oku-slides-i18n/AGENTS.md` records that only the UI isolates; `ui/AGENTS.md` that
  text sent to the host uses `tPlain`.
