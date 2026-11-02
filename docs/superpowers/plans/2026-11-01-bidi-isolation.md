# Bidi isolation of string arguments — plan (t-1380)

Spec: `docs/superpowers/specs/2026-11-01-bidi-isolation-design.md`. UI isolates string placeables
through an `ISOLATE($var)` rewrite; the Rust localizer stays raw.

## Task 1 — Rust: pin a raw tr_with and record the decision

- Regression test `crates/oku-slides-i18n/tests/regressions/issue_1380_raw_localizer.rs`: `tr_with`
  with a Hebrew argument adds no FSI/PDI; declared in `tests/regressions/main.rs`.
- `crates/oku-slides-i18n/AGENTS.md`: only the UI `t()` isolates; `tr_with` stays raw.

## Task 2 — UI: ISOLATE rewrite and tPlain in i18n.ts

- Failing tests first in `ui/src/i18n.test.ts` (string isolated, number raw, selector matches and
  `*[other]` value isolated, `NUMBER()` intact, no nesting, `tPlain` raw, RTL regression t-1380).
- `setMessages` rewrites each resource's message and term values (variant values included,
  selectors untouched; attributes skipped, `t()` never formats them) before `addResource`; the
  bundle gets an `ISOLATE` function.
- `tPlain(id, args)`: `t()` without FSI/PDI.

## Task 3 — UI: callers

- Undo labels and default chart/table headers sent to the host use `tPlain` (`useLayoutInsert.ts`,
  `useSlideOps.ts`, `stageItems.ts`, `ChartDataGrid.tsx`, `TablePanel.tsx`); `fieldLabel` compares
  `tPlain` output. `ui-pasted-image` takes one `$size`, so width and height share one isolate.
- Drop single-value `isolate()` calls in `close.ts` and `closeFlow.ts`; keep per-item `isolate()`
  in joined lists. `ui/AGENTS.md`: text sent to the host uses `tPlain`.

## Task 4 — Tests and gates

- Update unit, render and e2e expectations reading interpolated strings (undo `aria-label`,
  pasted image size, close dialogs, tags).
- `just ci` green, `just e2e <free port>` green.
