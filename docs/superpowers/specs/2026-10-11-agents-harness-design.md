# AGENTS.md harness — design

Task: t-833. Date: 2026-10-11. Status: approved design, awaiting plan.

## Goal

Give every coding agent working on this repo (Claude Code, Codex, Pi, ACP agents driven by the
app) the non-obvious rules it needs, placed next to the code they govern. The rules are
distilled from the ADRs (`docs/adr/`, 7 files), the specs and plans (`docs/superpowers/`,
30 + 41 files, ~570k words) and the Oku tickets of project `slider` (820: 63 open, 757
completed). Each rule cites its source and is checked against today's code.

## Decisions

| Question | Decision |
|---|---|
| Which agents read it | Several. `AGENTS.md` is the source of truth; each `CLAUDE.md` is the one line `@AGENTS.md`. |
| Relation to `ARCHITECTURE.md` | Two files, two roles: `ARCHITECTURE.md` describes the code, `AGENTS.md` prescribes how to work on it. `AGENTS.md` exists only where the corpus yields content. |
| Traceability | Every rule ends with a short reference: `(ADR-0006, t-746, spec:<name>, plan:<name>)`. |
| Keeping it current | A rule in the root `AGENTS.md` plus a mechanical check, `just agents-lint`, run by `just check`. |
| Build method | Extract by source (parallel subagents), consolidate by target (single pass). |
| Language | English, like all repo documentation (rule recorded in the root file and README). |

## 1. Hierarchy and file format

**Root.** The current root `CLAUDE.md` content moves to `AGENTS.md`; `CLAUDE.md` becomes
`@AGENTS.md`. The root file keeps language, commands, ports, gotchas and conventions, and
adds:

- the harness update rule: a fix or decision that creates, changes or voids a rule updates
  the nearest `AGENTS.md` in the same commit;
- a harness map: every nested `AGENTS.md` with one line saying when to read it. Codex only
  loads the root-to-cwd chain, not the file it edits, so the map is how it finds the rest.

Budget: 100 lines.

**Nested.** An `AGENTS.md` (60 lines max) plus a sibling `CLAUDE.md` containing exactly
`@AGENTS.md`, only where extraction yields rules. Likely targets: crates with an
`ARCHITECTURE.md`, `crates/oku-slides-core/src/replica/`, `ui/`, `src-tauri/`,
`typst/oku-slides/`, `docs/`. The final list comes out of consolidation.

Nested file format (empty sections are omitted; a rule is one line, two at most):

```markdown
# <area> — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants
- <rule that must never break> (ADR-0006, t-746)

## Pitfalls
- <thing that already went wrong and how to avoid it> (t-809)

## Testing
- <what to run / how to test this area, beyond `just test`> (spec:2026-10-10-arbre-deck-robustesse-design)

## Settled decisions
- <choice not to reopen, with the rejected alternative> (ADR-0002)
```

**Placement principle.** A rule lives in the deepest directory that covers everything it
concerns. A rule spanning core, session and UI goes to the root; one spanning only crates
goes to `crates/`.

## 2. Extraction and consolidation

**Candidate rule** (JSON lines in the session scratchpad, never committed):

```json
{"rule": "…", "kind": "invariant|pitfall|testing|decision",
 "target": "crates/oku-slides-core/src/replica", "source": "ADR-0006",
 "date": "2026-10-08", "evidence": "short quote or file:line", "supersedes": null}
```

**Extraction filters.** Keep a rule only if it is non-obvious (not deducible from reading the
code), actionable (says what to do or avoid, not how it is implemented) and durable (a
decision, invariant or pitfall; not a plan step, batch detail or TODO).

**Extractors** (8 subagents in parallel): ADRs (1), specs (2), plans (3; only pitfalls,
decisions, deviations and review notes, not steps), tickets (2: bugs and regressions; features
and decisions). For bugs, keep the cause and the guard; for features, only the decision they
fix. Open tickets yield known pitfalls, never rules.

**Consolidation** (one pass, by the orchestrating session):

1. Dedup: candidates saying the same thing merge and keep all their sources.
2. Supersession: on contradiction the most recent wins; at equal date, ADR > spec > plan >
   ticket. The loser is logged in the working file.
3. Check against today's code (grep or targeted read). A rule the code contradicts is dropped;
   a doubtful one goes to the user instead of being written.
4. Placement by the deepest-directory principle, then budget check. An overflowing file keeps
   the rules most costly to break; the rest go to the report's appendix.

**Checkpoint.** Before anything is written, the user gets a consolidation report per target
directory: rules kept, dropped, superseded and doubtful. The user approves or prunes.

## 3. `just agents-lint`

A bash recipe in the `justfile`, on the model of `fonts-lfs`; no new dependency. Takes
`root="."`; `just check` calls it. Excluded directories: `node_modules`, `target`,
`.worktrees`, `.claude`, `.git`. Errors are in French, like other tool output:
`✗ <file> : <reason>`.

| # | Check | Fails when |
|---|---|---|
| 1 | Each `AGENTS.md` has a sibling `CLAUDE.md` | it is missing, or its content is not exactly `@AGENTS.md` |
| 2 | No orphan `CLAUDE.md` | a `CLAUDE.md` has no sibling `AGENTS.md` |
| 3 | References resolve | `ADR-NNNN` without `docs/adr/NNNN-*.md`; `spec:`/`plan:` without a file of that name |
| 4 | Nested rules cite a source | a bullet under the four sections lacks a `(…)` with `ADR-`, `t-`, `spec:` or `plan:` |
| 5 | Budgets | root over 100 lines, nested over 60 |
| 6 | Root map is exact | a nested `AGENTS.md` missing from the map, or a map entry pointing nowhere |

`t-N` references are checked for form only; existence needs the local Oku daemon.

**Tests (TDD).** `just agents-lint-test` builds, in `mktemp`, one valid minimal tree and one
tree per failure case (six), and asserts `agents-lint` passes on the first and fails with the
expected message on each of the others. Written first, red, then the lint makes it green.

## 4. Rollout

Parent task t-833; one sub-task per plan step, same title, marked done as each step completes.
Work in a worktree `833-feature/agents-harness`, squashed onto `main`.

1. This spec, committed. The English-documentation rule already edited into `CLAUDE.md`
   and `README.md` on `main` lands in the branch's first commit.
2. `agents-lint-test` (red), `agents-lint` (green), wired into `just check`.
3. Extraction: 8 subagents write candidates to the scratchpad.
4. Consolidation and report. **Checkpoint: user approval.**
5. Writing: root `AGENTS.md` (migrated from `CLAUDE.md`, plus map and update rule), nested
   `AGENTS.md` files and their one-line `CLAUDE.md`; `just agents-lint` green.
6. End-to-end check: a Claude Code session and a Codex session started in
   `crates/oku-slides-core/src/replica/` each quote a rule from the local `AGENTS.md`.
7. Review and squash onto `main`.

**Out of scope** (each becomes a backlog task):

- trim the 5 `ARCHITECTURE.md` files over 50 lines (agent, import, mcp, session, core);
- add `ARCHITECTURE.md` to `oku-slides-cli`, `oku-slides-i18n`, `oku-slides-platform`;
- translate existing French ADRs and specs to English;
- e2e specs discovered by name pattern instead of a hand-kept list in `ui/package.json`;
- a periodic harness review (scheduled re-extraction of new tickets, ADRs and specs).

## Success criteria

- `just check` passes, `agents-lint` included.
- Every written rule has a resolvable source and was checked against the code.
- Claude Code and Codex both load the hierarchy (step 6).

## Cost

The extraction reads ~570k words through 8 subagents, plus code verification: several million
tokens. It is the main cost of this work.
