# crates/oku-slides-core — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Bump SOURCES_VERSION whenever REPLICA_FORMAT changes. (ADR-0005, plan:2026-10-10-deck-en-arbre)
- Enforce list size bounds, nested totals included, in the core on every write path, agent included; define bound constants only there. (plan:2026-10-04-suivis-listes, t-528, t-495)
- Validate links and highlights in rich list columns with the same rules as other rich params. (plan:2026-10-04-suivis-listes, t-527)
- In SourceBundle::new, canonicalize before and after normalize_hosts so exact copies are dropped first. (plan:2026-10-09-finitions-ids, t-688)
- Never add a top-level Slide or Element field starting with the at sign; such meta keys are reserved. (plan:2026-10-10-deck-en-arbre)
- Wrap every LoroError with LoroSnafu. (plan:2026-10-09-elements-en-arbre)
- Name widget files [a-z][a-z0-9-]* up to 64 chars or _commun; other names, or non-hex64 pack dirs, fail with BadName. (plan:2026-10-02-creation-widgets)
- Treat an empty widget file, widgets.typ included, as absent so it never enters the PackVersion hash. (plan:2026-10-02-creation-widgets)
- Keep unknown widget kinds and params verbatim as Value, never erroring on read; let validate report them. (spec:2026-09-27-widgets-typst-design)
- Refuse ops that would nest elements deeper than MAX_SLOT_DEPTH, since recursive walkers rely on that bound. (plan:2026-09-28-widgets-typst-2)
- Allow a bind only on a plain parameter. (plan:2026-09-28-widgets-typst-2)
- Refuse set_grid and set_cell while a non-empty flow slot holds the content; a grid places only free elements. (plan:2026-09-28-widgets-typst-2, t-292)
- Take grid track bounds (1 to 24) only from core GRID_TRACKS; never redefine them elsewhere. (plan:2026-10-03-widgets-agent-a2ui)
- Discard only a pack version whose content does not hash to its folder name, report it, and open the rest. (spec:2026-10-01-packs-portables-design)
- Keep bundle assets, fonts and packs add-only: merge by union, never remove one. (spec:2026-09-27-sources-loro-design, plan:2026-10-01-packs-portables)
- Store empty or whitespace-only optional prose as None; never keep Some of an empty string. (spec:2026-09-27-sources-loro-design)
- Read sources zips with bounded take reads, capped capacity and a total budget; check deck.loro size before reading. (spec:2026-09-27-sources-loro-design)
- Restrict list column types to plain, rich, number, flag, choice, asset and a list nested one level deep. (plan:2026-10-04-parametre-liste, t-495)
- Never name a list column id or kind, and never put a slash in a param name, column name or item id. (plan:2026-10-04-parametre-liste)
- Address list items by stable id, never by index; an item field or nested list path is list/itemId/column, parsed by ListPath. (spec:2026-10-04-parametre-liste-design, t-495)
- Treat an absent list as empty; seed schema defaults only on new objects, never on paste or replace_element. (spec:2026-10-04-parametre-liste-design)
- When overriding a base widget, keep each list parameter identical to the base definition. (plan:2026-10-04-parametre-liste)
- Normalize host ids and columns on every host write, and in SourceBundle::restore as in SourceBundle::new. (plan:2026-10-08-fusion-cellules, t-631)
- Keep the materialized deck canonical and equal to the replica re-read; canonicalize before writing to the replica. (spec:2026-09-27-sources-loro-design)
- Route all list edits, deck or widget trial values, through ListOp::apply. (spec:2026-10-04-parametre-liste-design)
- Revalidate host params with check_host_params whenever an op changes a widget kind, as Op::SetKind does. (t-591)
- Allow a pack to override a base widget only with compatible param types, binds and category. (spec:2026-09-27-widgets-typst-design)

## Pitfalls

- Move elements across slides with the move_element op; a duplicated id loses concurrent edits. (ADR-0006)
- Duplicate test helpers in tests/common/mod.rs, since integration tests cannot see the crate's cfg(test) code. (plan:2026-09-28-widgets-typst-2)
- Expect any change to pack.json, including name or description, to produce a new content-addressed PackVersion. (plan:2026-10-01-packs-portables)
- Generate random element ids; concurrent adds with the same id produce duplicates that validate flags. (spec:2026-09-27-sources-loro-design)
- Beware Loro UndoManager on commits with several mov on the slides list; undo can lose elements or panic. (t-263)

## Testing

- Ensure proptest strategies provably generate non-empty lists, using a fixed seed or a count check. (plan:2026-10-04-suivis-listes, t-529)
- Commit the convergence proptest at with_cases(48); never restrict a strategy, pin minimal failures as tests. (plan:2026-10-10-deck-en-arbre)
- Test untrusted snapshots (truncated, random, huge): they must yield Corrupted or TooLarge, never panic. (spec:2026-09-27-sources-loro-design)
- Run deep-tree regression tests on a 256 KiB thread stack to catch Loro recursion overflows. (spec:2026-10-10-arbre-deck-robustesse-design)

## Settled decisions

- Never flatten or prune Loro history or assets when saving, so every past version still renders and merges. (ADR-0002)
- Keep Loro pinned at 1.16.2; an upgrade must keep loro_tree_probe and loro_deck_tree_probe passing. (ADR-0005, ADR-0006)
- Add no backward compatibility: refuse old deck, sources and replica formats; drop replaced ops, no aliases. (spec:2026-09-27-widgets-typst-design)
- Let an inconsistent deck open and report issues via validate, but make ops refuse badly typed values. (spec:2026-09-27-widgets-typst-design)
- Use SetSlideField only for slide name and description; change kind with SetKind and other params with SetParam. (plan:2026-09-28-widgets-typst-2)
- Use the registry-resolved grid (Slide::resolved_grid) for set_cell, validation and preview. (plan:2026-10-03-widgets-agent-a2ui)
- Keep set_grid as a deck op visible to the agent, with placed elements rescaled in Rust. (plan:2026-10-03-widgets-agent-a2ui)
- Validate widget schemas in Rust check_widget as the authority; Typst only registers the type. (spec:2026-10-04-parametre-liste-design)
- Treat deck templates as one atomic optional PackRef; never patch it field by field. (t-347)
