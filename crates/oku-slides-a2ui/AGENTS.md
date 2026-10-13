# crates/oku-slides-a2ui — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Mark question surfaces with the typed Surface field, never by an ask- id prefix. (plan:2026-10-03-agent-thread-followups)

## Settled decisions

- Keep deny_unknown_fields on nested A2UI argument structs such as ChoiceOption so extra agent keys are refused. (plan:2026-10-03-agent-a2ui)
