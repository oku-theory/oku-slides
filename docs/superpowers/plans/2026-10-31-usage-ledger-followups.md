# Usage ledger follow-ups (batch D) — implementation plan

**Goal:** close the deferred review findings of the deck usage ledger (t-1296, plan
`2026-10-30-deck-usage-ledger.md`): t-1313, t-1310, t-1309, t-1307, t-1308, t-1314.

**Constraints:** root, `crates/`, session, agent, i18n and ui `AGENTS.md`; TDD; one squash commit
citing every ticket; `ARCHITECTURE.md` of session stays at 50 lines.

## Survey of the ACP adapters (2026-10-31, sources at their `main`)

| Adapter | `PromptResponse.usage` | `usage_update.cost` | Model `currentValue` |
|---|---|---|---|
| agentclientprotocol/claude-agent-acp | this turn only (reset per turn) | cumulative USD, before the reply; background work may send one after a turn settled | alias: `default`, `sonnet`, `opus`… |
| agentclientprotocol/codex-acp (TS; the Rust zed one is archived) | this prompt only | never | concrete id (`gpt-5.4`) |
| gemini-cli `--experimental-acp` | this prompt only | never | alias `auto` by default |

The ACP schema documents `usage` as "for this turn" but its `Usage` fields say "across session";
the end-turn-token-usage RFD is still a draft listing this as open.

## Task 1 — t-1313: close race loses unsaved usage

`save_pending_usages` skips a busy session; if it frees before `ensure_closable`, the close went on.
Fix: `Document.usage_write_tried` (in memory) set when a close tries the write, reset by a new record;
`Session::usage_awaits_write` (clean deck, usage unsaved, never tried) makes `ensure_closable` refuse
under the `try_lock` it already holds, like a busy session (no file I/O under the workspace lock).
A retried close writes it. Tests in `workspace_usage_tests.rs`: the race needs crate-private access
(`ensure_closable`, the slot lock), so its regression test is a unit test, not `tests/regressions`.

## Task 2 — t-1310: usage of a pack or theme tab wakes a dormant opener deck

`Sessions::record_usage(owner, record)` (default: `with`); the workspace overrides it with
`Workspace::record_usage`: an awake deck records at once (not rendered, no links), a dormant one
queues the record in `Dormant.usage`; `Dormant::wake_with_usage` drains it on wake; `Slot::flush` at
`settle` wakes a dormant deck with a queue (unrendered) so its resume keeps the records. A crash
loses a queue (memory only). Session `AGENTS.md`: `record_usage` is the one agent path besides
`with_session`.

## Task 3 — t-1307: per-turn or cumulative `PromptResponse.usage`

Per turn in all three adapters: recorded as is. Two-turn fake-agent test
(`two_turns_with_usage_record_each_reply_as_the_turn_own`); agent `AGENTS.md` pitfall on the draft
`unstable_end_turn_token_usage` feature.

## Task 4 — t-1309: a cost update after the prompt reply

Decision: no wait, no re-attribution. The cost is cumulative, so a late update is counted once, in
the turn that takes it first (the ending one if it lands before its record is built, else the next):
totals stay right, only the split between two turns moves. A wait cannot catch background work that
reports arbitrarily late, and the add-only ledger cannot amend a written record. Fake agent
`cost_after_reply`; test `cost_sent_after_the_reply_is_counted_once_across_turns`; agent
`AGENTS.md` settled decision; `usage_board.rs` doc.

## Task 5 — t-1308: `ModelOptions.current` may be an alias

Claude (`default`, `sonnet`) and Gemini (`auto`) report aliases; ACP carries no resolved id. Decision:
record the value the agent reports, as is; the "by model" report groups by it. Agent `AGENTS.md`
settled decision; `record_turn` doc.

## Task 6 — t-1314: localized cost; named Markdown tables

`usage_cost` formats the amount with `oku_slides_chart::NumberFormat` for the interface locale (fr
`0,0400`). `Markdown.tsx` tables: `role="region"` and `aria-label` `ui-agent-md-table` (20 locales).

## Task 7 — Review and gates

Multi-angle review, `just ci`, `just e2e` (Markdown tables in the agent thread), squash onto `main`.
