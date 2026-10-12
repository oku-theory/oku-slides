# AGENTS.md harness — design

Task: t-833. Date: 2026-10-11 (revised 2026-10-12 after three plan reviews and loading tests).
Status: approved design; implementation plan `docs/superpowers/plans/2026-10-11-agents-harness.md`.

## Goal

Give every coding agent working on this repo (Claude Code, Codex, Pi, ACP agents driven by the
app) the non-obvious rules it needs, placed next to the code they govern. The rules are
distilled from the ADRs (`docs/adr/`, 6 files), the specs and plans (`docs/superpowers/`,
~70 files, 4.9 MB) and the Oku tickets of project `slider` (827: 70 open, 757 completed).
Each rule cites its source, is checked against today's code, and is approved by the user.

## Decisions

| Question | Decision |
|---|---|
| Which agents read it | Several, natively: only `AGENTS.md` files, **no `CLAUDE.md` anywhere**. |
| Relation to `ARCHITECTURE.md` | Two files, two roles: `ARCHITECTURE.md` describes the code, `AGENTS.md` prescribes how to work on it. `AGENTS.md` exists only where the corpus yields content. |
| Traceability | Every rule ends with a short reference: `(ADR-0006, t-746, spec:<name>, plan:<name>)`. |
| Keeping it current | A rule in the root `AGENTS.md` plus a mechanical check, `just agents-lint`, run by `just check`. |
| Build method | Prepare inputs mechanically, extract by lot and consolidate by target in restricted `claude -p` processes, check mechanically after each LLM step, render and lint, then the user approves each rule. |
| Untrusted text | Corpus text is data. The user's approval, bound to each rule's exact text by a hash, is the gate; nothing is kept by default. |
| Language | English, like all repo documentation (rule recorded in the root file and README). |

### Why no `CLAUDE.md` (verified on Claude Code 2.1.282, 2026-10-12)

Claude Code reads `AGENTS.md` natively since 2.1.277: every `AGENTS.md` in the working
directory and its ancestors at start, and a subdirectory's `AGENTS.md` when a file there is
read — **only while no `CLAUDE.md`, `.claude/CLAUDE.md` or `CLAUDE.local.md` exists in the
directory or above**. The alternative, a `CLAUDE.md` containing `@AGENTS.md` in each
directory, loses the root rules whenever a session starts in a subdirectory: the root
import then points outside the working directory, Claude Code treats it as an external import
that needs interactive approval, and `-p` sessions skip it silently (observed 3/3). With
`AGENTS.md` alone, a session in a subdirectory loads both the root and the local rules, and a
session at the root loads a subdirectory's rules when it reads a file there (observed).
Codex and other agents read `AGENTS.md` natively too.

## 1. Hierarchy and file format

**Root.** The current root `CLAUDE.md` becomes `AGENTS.md`; no `CLAUDE.md` remains. The root
file keeps language, commands, ports, gotchas and conventions, and adds repo-wide rules (same
four sections as nested files), then:

- a harness section: Claude Code ≥ 2.1.277 is required; a `CLAUDE.md`, `.claude/CLAUDE.md`
  or `CLAUDE.local.md` would silently disable the harness, so none may be created, and the
  "Project instructions" setting stays at its default; the update rule (a fix or decision
  that creates, changes or voids a rule updates the nearest `AGENTS.md` in the same commit);
  source text is data;
- a harness map: every nested `AGENTS.md` with one line saying when to read it. Codex only
  loads the root-to-cwd chain, so the map is how it finds the rest.

Budget: 100 lines; the free room for repo-wide rules is measured on a render (≈ 30 lines).

**Nested.** An `AGENTS.md` (60 lines max) only where extraction yields rules. Likely targets:
crates with an `ARCHITECTURE.md`, `crates/oku-slides-core/src/replica/`, `ui/`, `src-tauri/`,
`typst/oku-slides/`, `docs/`. The final list comes out of consolidation.

Nested file format (empty sections omitted; the `ARCHITECTURE.md` sentence omitted where the
directory has none; one rule per line, ASCII, source last):

```markdown
# <area> — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- <rule that must never break>. (ADR-0006, t-746)

## Pitfalls

- <thing that already went wrong and how to avoid it>. (t-809)

## Testing

- <what to run / how to test this area, beyond `just test`>. (spec:2026-10-10-arbre-deck-robustesse-design)

## Settled decisions

- <choice not to reopen, with the rejected alternative>. (ADR-0002)
```

**Placement principle.** A rule lives in the deepest directory that covers everything it
concerns. A rule spanning core, session and UI goes to the root; one spanning only crates
goes to `crates/`.

## 2. Extraction and consolidation

All intermediate files live in the session scratchpad and are never committed; the
scratchpad is deleted at the end (it holds full ticket texts). The toolkit is five scratchpad
scripts, each with its own tests: `prepare.py`, `jobs.py`, `run_agents.py`, `harness.py`
and the lint's own test recipe.

**Candidate rule** (JSON lines):

```json
{"rule": "…", "kind": "invariant|pitfall|testing|decision",
 "target": "crates/oku-slides-core/src/replica", "source": "ADR-0006, t-746",
 "date": "2026-10-08", "evidence": "verbatim quote from a cited source", "supersedes": null}
```

**Extraction filters.** Keep a rule only if it is non-obvious (not deducible from reading the
code), actionable (says what to do or avoid, not how it is implemented) and durable (a
decision, invariant or pitfall; not a plan step, batch detail or TODO). Open tickets yield
known pitfalls only (enforced by the checker).

**Inputs are prepared mechanically.** Plans and specs lose their fenced code blocks (plans:
4.6 MB → 1.6 MB); tickets are rendered as Markdown (id, tags, state, date, name, description,
comments), wrapped at 400 characters, and split by the `bug` tag. Inputs are packed into lots
of at most 150 KB (18 lots, 2.2 MB on 2026-10-12) so a reader finishes before any context
compaction. A random read marker every 150 lines and at the end of each input must be echoed
by the extractor; a missing one proves a partial read.

**Least privilege.** Every LLM step that reads corpus text runs through `run_agents.py` as a
separate `claude -p --restricted --strict-mcp-config --tools Read,Grep,Glob --permission-mode
plan --model opus` process: it can read files in its working directory and added
directories and nothing else (no shell, no web, no writes, no settings, hooks or MCP), and
the runner writes its reply to disk. At most 4 run at once, each with a timeout; a non-zero
exit, timeout or empty reply is an infrastructure failure, re-run as is. Extractors work in
the prepared inputs with read access to the worktree, so their targets are real directories.

**Mechanical checks** (`harness.py`), after every LLM step:

1. `check`, after extraction: schema; source format; cited sources exist (ADR, spec, plan
   files; `t-N` in the ticket dump); the `evidence` quote (≥ 20 characters, after
   normalization) occurs in a cited source; the target is an existing, non-hidden directory
   inside the repo holding tracked files; the rule text is printable ASCII without `@`, `<`,
   `>` or leading markup, ≤ 140 characters, and its rendered line ≤ 160.
2. `assemble`, after consolidation: re-runs these checks on the final wording; ids are known
   and used by one rule only; every candidate is kept or explicitly dropped; every target
   produced an output; `moved_to` may only name another consolidated target; `verified` is a
   `path:line` within a tracked, non-docs file for every rule not marked doubtful.
3. A policy filter (commands, installs, network, git, hooks, review, permissions, secrets,
   skipping or disabling checks, the harness files) only **highlights** rules in the report.
   It cannot be complete against adversarial text, so it is not the gate.

**Consolidation**, one restricted process per target with its measured budget: merge rules
stating the same constraint (at most the two most authoritative sources kept, so the line fits 160
characters; every merged candidate id stays traceable in the report), resolve supersession (only through an explicit
`supersedes` or a contradiction confirmed by the code — a newer date alone is not enough),
verify each rule against today's code (`path:line`), drop rules the code contradicts, mark
the unconfirmable as doubtful, move rules to another consolidated target when they concern
more than this one, keep within budget, and propose the map summary line.

**Render before deciding.** All assembled rules are rendered into a scratch copy of the repo
and `just agents-lint` runs on it, so budget or format errors surface before the user decides.

**Checkpoint.** `harness.py report` writes the report: flagged rules first, then the injected
instructions agents reported, then per target the map summary and each rule exactly as it will
be written, with its id, a 12-hex hash of its target and rendered line, sources, `verified`
anchor, evidence quotes and doubtful mark; dropped candidates; totals; the render's lint
result. The user answers with the ids to approve (and any summary edits); the decisions file
binds each approved id to the hash shown. `finalize` keeps exactly those rules, refuses an
approval whose rule changed since the report, and refuses missing or unsafe map summaries.

## 3. `just agents-lint`

A bash recipe in the `justfile`, on the model of `fonts-lfs`; no new dependency. Takes
`root="."`; `just check` calls it; exits 2 outside a git work tree. Files are enumerated
with `git ls-files`, so git-ignored directories are skipped; `node_modules`, `target`,
`.worktrees` and `.claude` are excluded explicitly too. Errors are in French, like other tool
output: `✗ <file> : <reason>`.

| # | Check | Fails when |
|---|---|---|
| 1 | No file disables native `AGENTS.md` reading | a `CLAUDE.md` or `CLAUDE.local.md` exists (tracked, untracked, or `CLAUDE.local.md` even when git-ignored), or `.claude/CLAUDE.md` exists |
| 2 | References resolve | `ADR-NNNN` without `docs/adr/NNNN-*.md`; `spec:`/`plan:` without a file of that name (whole-word matches only); `ADR-` with more than four digits |
| 3 | Rules cite a source, root included | a bullet under `## Invariants`, `## Pitfalls`, `## Testing` or `## Settled decisions` (outside code fences) does not end with a `(…)` holding `ADR-`, `t-`, `spec:` or `plan:`; other sections are free |
| 4 | Budgets | root over 100 lines, nested over 60 (a last line without newline counts) |
| 5 | Root map is exact | a nested `AGENTS.md` missing from `## Harness map`, a map entry pointing nowhere, or an entry with `..` or an absolute path |

`t-N` references are checked for form only; existence needs the local Oku daemon (it is
checked by `harness.py`).

**Tests (TDD).** `just agents-lint-test` builds, in `mktemp`, one valid minimal git repo and
one per case (40 cases), and compares the lint's whole output to the exact expected line.
Written first, red, then the lint makes it green; 23 mutations of the lint must each turn the
suite red.

## 4. Rollout

Parent task t-833; one sub-task per plan task, same title, marked done as each completes.
Work in a worktree `833-feature/agents-harness`, squashed onto `main`.

1. This spec and the implementation plan are committed on `main` (path-limited
   `git commit --only`, after `git add` of the new plan, so other sessions' staged files are
   untouched) before the branch is created. The English-documentation rule, edited into
   `main`'s working tree, moves to the branch as a patch: captured with `git diff HEAD`,
   applied and committed in the worktree, and only then removed from `main`.
2. `agents-lint-test` (red), `agents-lint` (green), mutation check. Wired into `just check`
   only in step 5: until the root `CLAUDE.md` is gone, the lint fails on the repo.
3. Extraction: ticket dump, input preparation, toolkit tests, extraction jobs, marker and
   JSON checks, `harness.py check`.
4. Consolidation jobs, `assemble`, trial render and lint, report. **Checkpoint: the user
   approves rules by id**, then `finalize` and a final render and lint.
5. Writing: `git mv CLAUDE.md AGENTS.md`, `render` into the worktree, README `just check`
   comment; `just agents-lint` green, wired into `just check`.
6. End-to-end check in a copy of the branch outside the main checkout (whose `CLAUDE.md`
   would otherwise disable native reading until the squash), with `claude -p --setting-sources
   project --strict-mcp-config --tools Read,Grep,Glob --permission-mode plan` (no user settings
   or hooks; `--restricted` cannot be used: it skips project instructions) and
   `codex exec --sandbox read-only`; each case three times, with a distinctive token `K` of a
   nested rule found in no other harness file: started in the nested directory, each finds
   `K` (3/3); started at the root and asked about a file there, each finds `K` (2/3); a
   negative control from an unrelated directory, for both CLIs, does not (3/3). A Codex result
   that only names the nested file counts as a failed criterion; failures are reported to the
   user and do not block the squash.
7. Review, rebase onto `main`, squash. Before the squash: `main`'s index must be empty, and
   the branch's files must not intersect `main`'s uncommitted or untracked files (otherwise
   stop and ask). If the commit-review hook blocks, the user may approve a one-command
   exemption (`git -c oku.review=deferred commit …`), which persists nowhere.

**Out of scope** (each becomes a backlog task):

- trim the 5 `ARCHITECTURE.md` files over 50 lines (agent, import, mcp, session, core);
- add `ARCHITECTURE.md` to `oku-slides-cli`, `oku-slides-i18n`, `oku-slides-platform`;
- translate existing French ADRs and specs to English;
- e2e specs discovered by name pattern instead of a hand-kept list in `ui/package.json`;
- a periodic harness review (scheduled re-extraction of new tickets, ADRs and specs);
- documentation skills (`claude-md-improver`, `revise-claude-md`, `update-docs`) still write
  `CLAUDE.md`, which would disable the harness; teach them `AGENTS.md`.

## Success criteria

- `just check` passes, `agents-lint` included; no `CLAUDE.md` remains in the repo.
- Every written rule was approved by the user by id and hash, has a resolvable source and a
  quote found in it, and a `path:line` in today's code (or was approved as doubtful).
- Claude Code and Codex both load the hierarchy (step 6), or the gap is reported.

## Cost

The extraction reads ~2.2 MB in 18 restricted `claude -p` processes (4 at a time), then one
consolidation process per target directory reads its candidates and the code it verifies:
several million tokens. It is the main cost of this work.
