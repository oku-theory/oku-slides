# ADR-0001: Workspace, one session per tab, lazy v2 resume

- **Status:** accepted
- **Date:** 2026-09-26
- **Tasks:** t-142 (tabs and windows), t-158 (lazy restore), t-156 (this ADR)
- **Spec:** [onglets-fenetres-design.md](../superpowers/specs/2026-09-26-onglets-fenetres-design.md)

## Context

Until t-142, the app opened only one presentation at a time: a single `Session`,
resumed at launch from `<data>/resume/` (v1 format: `state.json`, `deck/`,
`templates/`). The user wants several presentations open, in tabs spread over
several windows (several screens), in a single instance of the app. Each tab keeps a
heavy state (Typst rendering, SVG cache, history, templates mode), and two windows must
be able to render at the same time. Everything must come back on relaunch, without the
launch costing the rendering of every tab.

## Decision

**One `Session` per tab, held by a `Workspace`.**

- `Workspace` (crate `oku-slides-session`) holds the windows (`w-<n>`), their
  ordered tabs (`t-<n>`), the active tab of each, the focused window,
  and one session per tab behind an `Arc<Mutex<Slot>>`.
- `Slot`: `Ready(Session)`, `Dormant` (tab resumed, not yet read back) or
  `Closed` (closed before being read back, never woken again).
- **Lock order:** the workspace, then a session, never the reverse. A tab request
  locks only its session. No waiting on a session under the workspace lock
  (`try_lock` only). A new session is built outside the lock.
  A poisoned lock is recovered.
- Identifiers are never reused (counters in `workspace.json`), validated
  on read-back (`t-`/`w-` and 1 to 18 digits): they become paths
  and Tauri window labels.

**v2 resume:**

```
resume/
  workspace.json   { version: 2, next_tab, next_window, focused,
                     windows: [{ label, tabs, active }], kept, info: { t-<n>: TabInfo } }
  tabs/t-<n>/      the v1 shape: state.json, deck/, templates/
```

- One writer per file: each session writes `tabs/<id>/`, only the
  `Workspace` writes `workspace.json`.
- **Lazy restore (t-158):** no session is read back at launch. The tab bar comes
  from `info`; a session is read back on the first request to its tab. Since the
  info can lag behind (abrupt stop between two writes), any destructive decision
  (closing without asking, replacing a blank tab) reads the tab back first; a
  failed read-back means the cautious decision.
- **No v1 migration:** the old `resume/state.json` is ignored, never
  converted. `<data>/resume/` is deleted by hand once.

**Out of scope:** the same deck in two windows, recoverable closed tabs, migration of the
old format.

## Consequences

**Positive**

- Two windows render in parallel: each tab has its own lock.
- `Session` barely changes: resume, history and templates mode stay
  those of a single tab, in its own folder.
- Launch no longer costs the rendering of every tab; a corrupted tab
  is only discovered when used, and costs only itself (set aside, notice).
- A tampered entry in `workspace.json` costs only its tab.

**Negative**

- Two sources for the state of a dormant tab (`info` and its folder):
  the tab must be read back before any destructive decision, and the tests
  must cover lagging info.
- The lock order is a rule to uphold by hand, which the compiler does
  not check (documented in the crate's `ARCHITECTURE.md`).
- `workspace.json` is rewritten on every change of a tab's title or state
  (written under the workspace lock).
- `Resync` (the interface subscribes to a displayed tab) wakes the session of
  that tab to read back its conversations: a tab is no longer read back on its
  first prompt but on its first display.

**Neutral**

- v1 users lose their resume once (folder ignored,
  not deleted).
- An unreadable tab discovered late stays in the tab bar, blank, instead of
  being removed at launch.

## Rejected alternatives

- **A single session that switches deck on each tab:** only one
  render at a time, history and cache to save and reload on every
  tab switch; slow, and two windows would block each other.
- **One process (or instance) per window:** no single instance,
  dragging a tab between windows becomes a transfer between processes,
  and "open with…" does not know which instance to target.
- **A single `Mutex<Workspace>` wrapping the sessions:** simple, but any
  render blocks every window.
- **Migrate the v1 format:** compatibility code kept for a single
  pass, against the project's "no shim" rule (MVP).
- **Read everything back at launch (initial t-142):** simple and without duplicated
  state, but launch costs the rendering of every open tab.
