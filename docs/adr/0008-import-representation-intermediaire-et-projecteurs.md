# ADR-0008: Import through an intermediate representation and projectors

- **Status:** accepted
- **Date:** 2026-10-15
- **Tasks:** t-692, t-729
- **Spec:** [`2026-10-09-import-pptx-design.md`](../superpowers/specs/2026-10-09-import-pptx-design.md)

## Context

Importing a presentation has two independent axes: the **source format**
(PPTX today, PDF in a later phase) and the **target** (hi-fi: a generated theme
and pack that reproduce the source; retrofit: the content laid out on an
existing pack and theme). Written as one pass per pair, every reader would know
every target and the other way round, and an import could leave a deck half
written when a late step fails.

## Decision

- **One contract between readers and projectors: `SourceDeck`.** A reader
  (`pptx/`, later a PDF reader) produces a resolved `SourceDeck` (no inheritance
  left to compute, each styled value with its provenance). A projector
  (`hifi/`, `retrofit/`) reads only `source/`, `plan`, `report`, `coverage` and
  `fonts`, never a reader module. Adding a format or a target adds one sibling,
  not a pair.
- **Projectors are pure.** They take the `SourceDeck` and a target (registry,
  templates, theme tokens, ids and dates) and return an `ImportPlan`; they do no
  I/O and depend neither on the session nor on the UI.
- **The plan is made of `apply` ops.** `ImportPlan.ops` are the same ops the
  agent and the editor send, with theme, pack, assets and fonts beside them. The
  caller (CLI `oku-slides import`, later the app) opens a new deck, installs the
  files, then applies all ops **in a single transaction**: the import goes
  through the same validations as any edit and is undone in one step.
- **Losses are data.** Whatever a projector cannot carry is a `Loss` in the
  report, never a silent drop; coverage is measured on the resulting deck.

## Consequences

- A PDF reader only has to fill the `SourceDeck`; vocabulary that is still
  OOXML-shaped in it (t-728) must be named neutrally or documented first.
- Projector tests need no file: they build a `SourceDeck` by hand.
- Rendering-based checks (reference witness, visual gap, t-692 lot 4) stay
  outside the crate, in the CLI and the render worker.
- A bug in an op only surfaces at apply time; `check_budget` bounds the plan's
  size before applying it.
