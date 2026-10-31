# Token usage ledger in each deck — design

Task: t-1296. Date: 2026-10-30.
Status: implemented 2026-10-30 (t-1296). Plan: [`2026-10-30-deck-usage-ledger.md`](../plans/2026-10-30-deck-usage-ledger.md).
The user's answers: stored in the deck (not the app data dir); read through the CLI and through an
MCP tool plus a hub-built A2UI box; pack and theme tab turns go to their linked deck; a turn without
usage is recorded with unknown tokens; cost only as the agent reports it; one `usage.jsonl` entry in
the sources zip. After the plan review: a clean deck closed with unrecorded usage is rewritten
silently; one global setting `usage.store_in_deck` (default on) turns recording off.

## Problem

Nothing records what the agent spent on a deck. ACP tells the app the session's model
(`session_model.rs`) and can return per-turn token counts, but `turn.rs` only reads `stop_reason`.
The user wants ccusage-style reports: tokens, turns and cost per day, month, model or deck.

## Goals

- Every agent turn that ends with a reply adds one record to its deck: model, preset, tokens, cost.
- The record travels with the deck file and survives merges, copies and app restarts.
- One aggregation, read by `oku-slides usage` (table or JSON) and by the agent (`usage`,
  `show_usage`).

Non-goals: a price table (cost is only what the agent sends), local time zones (UTC days), a chart
in the box, a per-deck opt-out (the opt-out is global), counting turns of a pack or theme tab with
no linked deck.

## ACP sources

- `PromptResponse.usage` (`Usage`: `total_tokens`, `input_tokens`, `output_tokens`, optional
  `thought_tokens`, `cached_read_tokens`, `cached_write_tokens`), behind the schema feature
  `unstable_end_turn_token_usage`, enabled in the agent crate's `Cargo.toml` only. Optional: many agents
  leave it out.
- `SessionUpdate::UsageUpdate.cost` (`amount`, `currency`): the session's **cumulative** cost,
  optional. `used`/`size` (context occupancy) are not consumption and are not recorded.

## Record

`oku-slides-core/src/usage.rs`, one JSON object per turn:

```json
{"turn":"9f1c…","at":"2026-10-30T14:02:11Z","tab":"deck","agent":"claude","model":"claude-opus-5-5",
 "tokens":{"input":1840,"output":512,"thought":0,"cache_read":41200,"cache_write":900},
 "cost":{"amount":0.0412,"currency":"USD"}}
```

- `turn`: UUID v4 made by the app per turn, the record's identity.
- `at`: end of turn, RFC 3339 UTC (`time`).
- `tab`: `deck`, `pack` or `theme`. `agent`: preset id (1 to 64 chars).
- `model`: the session's current model (≤ 200 chars), `null` when the agent offers none. `agent`
  and `model` hold no control, format (Unicode Cf: bidi marks, zero-width, BOM) or line/paragraph
  separator character (they reach terminals and Markdown); `shown_char` in core is the one test.
- `tokens`: `null` when the reply has no `usage`; `total` is ACP's `total_tokens`; ACP's optional
  fields stay `null`, never 0.
- `cost`: `null` when no `UsageUpdate.cost` was seen. `amount` finite, between 0 and 10⁹,
  `currency` three upper-case letters.

`UsageLedger`: `BTreeMap<TurnId, UsageRecord>`, a field `usage` of `SourceBundle`, add-only. It
refuses a record failing these checks, so the app never writes a line its reader drops; capture
sanitizes the agent's values first (control characters stripped, lengths cut, a bad cost dropped).
Unknown fields are kept (`extra`) and written back, so an older app that saves a newer app's
lines keeps them whole. At most 10,000
records and 16 MiB of lines, and no line over 4 KiB, so a full ledger always reads back whole; past it new turns are not
recorded (`Inserted::Full`, one warning per session).

## Capture

In `oku-slides-agent/src/turn.rs`, when the prompt reply arrives (also for a cancelled turn):

- tokens from `response.usage`, model from `session_model`, preset id kept by the agent link next
  to the running launch (the running agent did not know it).
- cost: the last `UsageUpdate.cost` seen during the turn minus the session's previous one. A
  negative difference (new ACP session) or a changed currency makes the current value the new base,
  and the turn records the current value whole. No cost seen: `null`. A `UsageUpdate` replayed by
  `session/load` or routed to no live session is ignored, as `ConfigOptionUpdate` is; the first
  turn of a resumed session only sets the base and records `null`, so a resume never charges the
  whole history to one turn.
- A turn that fails without a reply records nothing; its tokens are lost. A cancelled turn is
  recorded. The local agent reports no usage, so its turns have `tokens: null`.
- The record goes to the tab's `TabSession`. A deck tab writes its own bundle; a pack or theme tab
  writes the deck that owns it (a deck-only pack's `TabDocument::deck_tab`) or opened it
  (`Tab.opener`, set whether the user or the deck's agent opened the tab). A pack or theme tab with
  neither is not counted. A help tab records nothing.
- The record goes to the deck's real document whatever the tab's mode: the open document in deck
  mode, the stashed deck while history is open. A pack tab's preview is only a copy of its opener's
  deck, so its turns are routed to the opener; `Tab.opener` lives in memory, so a pack or theme tab
  restored after a restart is no longer counted.
- Recording never marks the deck modified: a question to the agent does not ask to save. It
  schedules the tab's resume, which writes the ledger (`tabs/t-<n>/deck/usage.jsonl`, through
  Storage) only when the ledger changed since its last write; the file gets it at the next save. A deck reread because its file changed on disk
  keeps the unsaved records (union).
- Closing a clean deck that holds unsaved records rewrites its file silently, no dialog, if the file
  on disk has not changed since it was read: the sources zip is re-embedded into the PDF bytes on
  disk, without rendering, so the pages stay identical. A dirty deck keeps the usual dialog; otherwise the records are dropped with a warning in the log
  (the newer file wins, as on any close of a clean deck). Quitting the app writes no deck file: the
  resume keeps the records until the next save. The close never waits: a deck tab whose session is
  busy or asleep is refused as unsaved and keeps its records. A deck with history open and unsaved
  records counts as unsaved; the close flow leaves history first, then saves silently.
- A failure to record (it should not happen) is a `tracing::warn!`; the turn ends normally.

## Setting

`[usage] store_in_deck = true` in the app settings (`OKU_SLIDES_USAGE__STORE_IN_DECK`). Off: no
record is made, and a save writes no `usage.jsonl`, dropping what the deck held, so a deck can be
shared without its spending history. The help page on the agent says so.

## Zip entry

- `usage.jsonl` at the zip root, one record per line, sorted by `(at, turn)`, so saving the same
  ledger twice gives the same bytes. Absent when the ledger is empty.
- `SOURCES_VERSION` stays 13. Readers already skip unknown entries (`sources_zip.rs`), so a new
  file opens in an older app; a v13 file opens with an empty ledger. An older app that saves the
  file drops the ledger. Bumping the version would make every existing deck unreadable.
- Reading is bounded: the loop over entries skips `usage.jsonl` by name before opening it; it is
  read after every other entry, capped at 16 MiB on its own (outside `SourceLimits.total_bytes`,
  so a deck full of assets keeps its ledger); lines over 4 KiB
  are refused. A line that fails the record's checks is dropped with an `Issue`
  (`usage-line-invalid`, its line number); the same `turn` twice with different content keeps the
  record whose JSON line is shorter (byte order on a tie, so a replacement never grows the ledger) and
  reports the turn once (`usage-line-forged`); past 20 such
  issues one summary (`usage-lines-dropped`, the count) replaces the rest. An entry over its cap or
  unreadable (encrypted, unsupported method, bad CRC, I/O) is dropped whole (`usage-too-large`, `usage-unreadable`). The deck always opens.
- `SourceBundle::merge` unions both ledgers by `turn`, collisions handled as on read: commutative
  and idempotent, even on forged collisions.
- `save_copy` keeps the ledger; a new deck or an import starts empty; a history tab (`stash`) does
  not touch it.

## Aggregation

`aggregate(records, By, Range) -> UsageReport` in core `usage_report.rs`, the only one (CLI and hub
call it); `records` pairs each record with its deck's label:

- `By`: `day`, `month` (UTC), `model`, `agent`, `tab`; the CLI adds `deck` over several files,
  labelled by path without `.oku-slides.pdf` (or a JSON deck's extension), and counts a turn found in several files (a copy, a merge)
  once.
- `Range`: optional `since` / `until` dates (`YYYY-MM-DD`), inclusive, parsed in core so callers
  need no date crate; `since` after `until` is refused.
- A row: key (none for an unknown model, shown "unknown"), `turns`, `turns_without_tokens`,
  `input`, `output`, `thought`, `cache_read`, `cache_write`, `total`, and costs per currency (never summed across currencies), each marked
  partial when some turns in the row have no cost. Plus a totals row.

## Agent tools

`oku-slides-mcp`, `read_tools`, listed for deck, pack and theme tabs (not help), added to the local
agent's `families_of`:

- `usage {by, since?, until?}`: the `UsageReport` as JSON. A pack or theme tab reads its linked
  deck; none: `ToolError::UsageNoDeck`. A bad range: `ToolError::UsageBadRange`.
- `show_usage {by, since?, until?}`: the hub builds the box itself (like `hub/settings`), shown in
  the turn's thread (`show_static`): `Badge`s for total tokens, turns, turns without tokens and
  cost, then a `Markdown` GFM table of nine columns, scrolling sideways when wide (the CLI prints
  the same columns), headers from Fluent (`LiveLocalizer`). One function in the session crate (`usage_cells`) builds
  the cells for the box and the CLI. No new component: the catalog and
  `CATALOG_ID` do not change.
- `show_usage`'s own description names it for "how much did this deck cost" questions (`deck.md`
  is shared with help tabs, which do not list the tool). Every table cell is Markdown-escaped; an
  empty cell shows `–`. The UI wraps Markdown tables in a scrolling container (the table keeps its
  table semantics for screen readers).

## CLI

`oku-slides usage <deck…> [--by day|month|model|agent|tab|deck] [--since D] [--until D] [--json]`:
`--by` defaults to `day`; aligned table with Fluent headers and plain numbers, or the raw report
with `--json`. Only the ledger's `Issue`s (`issue-usage-*`) go to stderr;
paths are printed without control or format characters; dropped lines keep exit code 0, an
unreadable deck fails as in other commands.

## Errors and i18n

Snafu, a backtrace in every variant, opaque public types, Fluent messages. New `Issue`s
`usage-line-invalid`, `usage-line-forged`, `usage-too-large`, `usage-unreadable`,
`usage-lines-dropped` (core
`validate.rs`); `UsageRangeError` (core); `ToolError` `UsageNoDeck`, `UsageBadRange`; column, badge
and CLI keys `usage-…`. English is the reference; every key goes to all 20 locales in the same
change.

## Open questions (backlog)

ACP does not say whether `PromptResponse.usage` covers the turn or the session (t-1307); whether
`ModelOptions.current` can be an alias (t-1308); a cost update after the reply counts in the next
turn (t-1309); routing to a dormant opener deck wakes it (t-1310).

## Tests

- core: zip round trip, stable order; v13 file without the entry gives an empty ledger; invalid line
  dropped with its `Issue`, the others kept; forged `turn` reported, the shorter line kept (byte order on a tie); over-budget entry
  dropped, deck opens; merge commutative and idempotent (`rstest`); `aggregate` by day, month,
  model, tab, separate currencies, partial cost, `turns_without_tokens`, range bounds.
- agent: cost difference table (growing total, new session, currency change, no cost); a reply
  without `usage` gives `tokens: null`; model read from `session_model`.
- session: a linked pack tab's turn lands in its opener's deck; unlinked, nothing; recording leaves
  the deck unmodified; the ledger survives a resume, history and pack modes, and a reread from
  disk; closing a clean deck saves it silently, not over a changed file; the setting off records
  and writes nothing.
- mcp: `usage` report; `show_usage` box checked on Fluent keys in a non-English locale;
  `UsageNoDeck`.
- cli: `usage --json` over two fixture decks; `--by deck`.
- `fake_agent.rs` returns scripted `usage` and `UsageUpdate.cost`; e2e `ui/e2e/NNN-usage.cjs`
  sends a prompt, asks for `show_usage` and reads the table (French labels).

## Documentation

`ARCHITECTURE.md` of core, agent, session, mcp and cli in the commits that change them. Rules, in
the nearest `AGENTS.md`: the usage ledger is add-only (except that `store_in_deck = false` drops it
on save) and never part of `Deck` or the replica; a usage record never marks a deck modified. The
README lists the `[usage]` setting, its environment variable and the `usage` command.
