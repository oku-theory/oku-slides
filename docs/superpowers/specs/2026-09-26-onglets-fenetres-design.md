# Single instance, tabs and windows — design

Date: 2026-09-26 · Task: t-142

## Goal

Several presentations open at the same time, in tabs, spread
across several windows to take advantage of several screens. A single instance
of the app: "open with…" while it runs opens a tab in the
existing window.

"Browser" model: each window has its tab bar; a tab (a
deck) is open in only one place; it moves from one window to another by
menu or by drag and drop.

Out of scope: the same deck shown in two windows; presentation mode
or presenter view; a custom application menu (only the default macOS menu
is rebuilt, without "Fermer la fenêtre" (Close Window)); recoverable
closed tabs; resuming the old format (v1).

## Architecture

- **One `Session` per tab.** `Session` barely changes: each
  tab keeps its Typst rendering, its SVG cache, its history and its templates
  mode.
- New module `crates/oku-slides-session/src/workspace.rs` (tests:
  `workspace_tests.rs`): `Workspace` holds the windows, their ordered
  tabs, the active tab of each and the last active window, and the
  sessions (`Arc<Mutex<Session>>` per tab). It opens, closes, reorders and
  moves tabs.
- Locks. The `Workspace` lock only covers its structure: a tab
  request locks only its session, two windows render in parallel.
  Order: the workspace then a session, never the reverse. It lives in
  four crate functions (`Workspace::apply`, `Workspace::request`,
  `Workspace::open_files`, `Workspace::settle`) that the dev server and
  the host call. No
  slow work under the workspace lock: a new session is
  built outside the lock (id reserved, session created and opened,
  then inserted), and the folders of closed tabs are deleted afterwards.
  Only exception: closing reads the real state of the sessions involved. A
  poisoned lock is recovered (`PoisonError::into_inner`): a panic does not
  block the app.
- Ids: tabs `t-<n>`, windows `w-<n>` (the Tauri label of the
  window), `n` from 1 to 18 digits. Counters kept in `workspace.json`,
  never reused.
- `Workspace::new(fonts, platform, file)`: one window and tab `t-1`,
  blank or with `file` open; nothing is written (dev server, tests).
  `Workspace::resume`: the desktop app.
- Opening a file (dialog, argument, second instance, macOS
  `RunEvent::Opened`): if it is already open, its tab is activated (and its
  window brought to the front), including when another opening of the same
  file finished during construction. Otherwise a tab is created
  in the last active window; if the active tab is blank (new deck
  never touched: not opened, saved, modified or undone — the real
  state of its session is authoritative, read without waiting: a busy session is
  not blank), it is closed once the new tab is ready, which takes its
  place. Paths are compared absolute and normalized (`.`,
  `..`); symbolic links are not followed.
- Closing the last tab of a window closes the window; closing the
  last tab of the last window ends the app (that tab, closed
  on purpose, is not kept: at the next launch, a blank tab). This
  `Close` returns `Applied` with a view without windows; the host quits the app
  on seeing it empty (`windows::sync` calls `state::shutdown`). The dev
  server, with no native window to quit, refuses it upfront (`409`,
  `session-last-window`) without closing anything: the interface would otherwise stay on
  "loading". `WorkspaceOutcome::LastWindow` only concerns
  `CloseWindow` on the only window (see below): the host quits
  keeping everything, the dev server also answers `409`.
- Quitting the app keeps everything: ⌘Q, and closing the **last** window (the only
  way to quit on Windows and Linux); no question is asked.
  The host applies this rule itself. Closing a window that is not the
  last closes its tabs.
- A single exit path, `state::shutdown`: it marks `AppState::closing`
  (`request`, `workspace` and `drop_tab` refuse everything afterwards, without
  touching the workspace any more), calls `Workspace::settle` (waits for the
  in-flight requests then saves the state, under a lock that also sets
  `closed`: an `OpenTab` afterwards is refused, and a tab whose
  construction overlaps `settle` is not inserted, its session goes to the
  trash), then `app.exit(0)`. It is called from `commands::quit`, the
  "Quitter" (Quit) menu (a macOS-only item, rebuilt on top of Tauri's default
  menu), closing the last window, and — as a safety
  net only, because `Workspace::settle` must never be invoked
  from the main thread — `RunEvent::Exit`, where no thread can
  hold these locks any more.
- `Close { tab, discard }` and `CloseWindow { window, discard }` check
  the real state of each session: a modified tab that the user did not
  choose to abandon (`discard`) is refused (`Unsaved` error). The UI
  indicator can lag behind an in-progress render.
- Shared templates: nothing to invalidate. A session's preview cache
  (`template_cache`) compares the set read back from disk on each call:
  a set saved in one tab is seen immediately in the others. A
  test guarantees it. Two tabs saving the same file at the same
  time do not interfere: `Storage::write_atomic` takes a temporary file
  specific to each write.
- `Request` loses `New`, `Open` (opening goes through the workspace)
  and `Messages` (which depends on no tab: it moves to
  `WorkspaceRequest`).
- `open_at_launch` and the keys `notice-resume-kept-unsaved` and
  `session-busy` are removed: a received file no longer overwrites anything, it
  opens a tab.
- Development server (`serve.rs`): `Workspace::new` with the given
  file, a single window, no resume.

## Resume files

Under `<data>/resume/`, via `Storage::write_atomic`:

```
resume/
  workspace.json    { version: 2, next_tab, next_window, focused,
                      windows: [{ label, tabs: [id…], active }], kept: [id…],
                      info: { id: { title, dirty, editing_templates, path,
                                    pristine } } }
  tabs/<id>/        the shape of the old resume/ (v1):
    state.json      { version, mode, ui }
    deck/ …         document.json (with `blank`), assets/, fonts/
    templates/ …
```

- Each `Session` writes only `tabs/<id>/` (one writer per folder); only
  the `Workspace` writes `workspace.json`.
- When read back, `workspace.json` is not trustworthy: only
  `t-<digits>` ids and `w-<digits>` labels are accepted,
  without duplicates; the counters are brought beyond the largest id
  read back (or seen on disk), and a counter beyond 18 digits is
  reset to that minimum. Otherwise a path (`../..`) would escape the folder, an
  invalid label would make window creation panic at every launch,
  and two tabs would share a folder.
- `kept`: tabs listed but not restored whose folder still exists
  (unreadable and impossible to set aside, window refused…). They stay
  listed from launch to launch: their folder is never deleted.
- `document.json` keeps `blank`: a blank tab stays blank on
  relaunch.
- Changing the active window writes nothing: the order is saved with the
  next change.
- Creating a tab: its folder first, `workspace.json` next.
  Closing: `workspace.json` first, the folder next. A stop between
  the two leaves at worst an orphan folder, deleted at the next launch.
- Moving a tab only touches `workspace.json`.
- Old format (v1: `resume/state.json` without `workspace.json`): not handled,
  no shim. `<data>/resume/` is erased by hand, once, before the
  first launch of this version; otherwise it stays there, ignored.

## Restoration

1. `workspace.json` absent: one window, one blank tab.
2. Unreadable or unknown version: the whole `resume/` set aside
   (`resume.failed-…`), one window, one blank tab.
3. Lazy restoration (t-158): no tab is read back at launch.
   The bar comes from the info in `workspace.json` (`info`, written with the
   structure and at each title or state change); the session of a
   tab is read back only at its first request, like the v1 resume
   (file modified elsewhere, unreadable history…). Unreadable at that
   moment: only its folder is set aside (`t-3.failed-…`), the tab
   stays, restarts from a blank deck with a fresh resume and announces it
   (`notice-tab-resume-failed`). The saved info may lag
   behind (abrupt stop between two writes): closing a sleeping tab
   without discarding, or replacing a saved blank tab with an opened
   file, reads it back first, outside the workspace lock, without
   ever waiting for a busy session. Read-back impossible: closing
   refused (question), no replacement. A tab closed before having
   been read back is never woken again (its folder is not rewritten).
   Tab listed without readable info (file from before t-158, or altered
   entry, which only costs its tab): read back at launch, removed if it
   is unreadable. Tab listed without `state.json`:
   removed (folder kept if it exists, via `kept`).
   Window left without tabs: removed. No window left: one window,
   one blank tab.
4. `tabs/t-<n>` folders that `workspace.json` does not list (neither in a
   window, nor in `kept`): deleted. Foreign names stay. Folders
   set aside too, except beyond the `resume.keep_failed` setting
   (3 by default): at launch, only the most recent remain, among the
   `resume.failed-*` and among the `tabs/t-<n>.failed-*` (t-143).
   A new tab reads nothing back: a leftover found at its number is set
   aside, the tab keeps its deck (the file just opened) and its
   resume starts fresh; only a leftover impossible to move leaves it
   without resume.
5. File(s) passed as arguments: opened next, off the main thread,
   following the opening rule above. Unreadable: logged, notice in
   the active tab.

## Host ↔ interface

- `request(tab, req)`: the existing `Request`, addressed to a tab.
- `workspace(req)` with `WorkspaceRequest`: `View`, `Messages`,
  `NewTab { window }`, `OpenTab { window, path }`, `Activate { tab }`,
  `Close { tab, discard: bool }`, `Reorder { tab, index }`,
  `MoveTab { tab, window: Option<String>, index: Option<usize> }` (`None`:
  new window; without index: at the end; same window without index:
  the tab is only activated),
  `NewWindow`, `CloseWindow { window, discard: Vec<String> }`. Response: the
  view, as JSON (`Messages`: language and Fluent resources). The active window
  is tracked by the host (`Workspace::focus`, in memory), not by a
  request. On the Tauri side, each focus event receives a number taken on the
  main thread (`AppState::focus_seq`); applying it (off the main thread)
  compares this number to `AppState::focus_applied` under the workspace
  lock and ignores an event older than the last applied one: the order
  of arrival is authoritative even if the tasks applying it finish out of
  order.
- `drop_tab(tab)` (Tauri only): tab dropped outside its window. The host
  reads the cursor itself (tao, never `screenX`/`screenY`: t-165) and
  compares it to the window rectangles in its coordinate space: points on
  macOS, physical desktop pixels on Windows and X11, common to all
  screens even with different densities (t-160). On Wayland, the cursor and
  window positions cannot be read (tao returns `(0, 0)`, which counts
  as unreadable on Linux): the drop opens a new window, placed
  by the compositor. Outside macOS, a new window is placed hidden, its
  size reapplied on the target screen, then positioned according to its real
  height (title bar included) before being shown.
- `quit` (Tauri only): quit keeping everything.
- `WorkspaceView`: `{ focused, windows: [{ label, active, tabs: [{ id,
  title, dirty, editing_templates, path, pristine }] }] }`. Title: file
  name, otherwise deck title (the deck set aside while editing
  templates).
- The host emits the `workspace` event (the view) to all windows after
  any command that changes a tab (title, `dirty`) or the structure. The
  windows follow the most recent view, in order (serialized
  updates). On each event, the active tab of each window also re-reads
  its snapshot (`nonce` passed from `Workspace.tsx` to `App`): a notice
  set without changing tab (received file that could not be opened)
  is shown immediately. A response older than another one already applied
  is discarded (`latest.ts`); its notice is shown anyway. In
  development, re-reading the view after each command does not re-trigger
  the snapshot (no loop).
- `drop_tab`: the window whose rectangle contains the pointer receives
  the tab last; on overlap, the most recently active one.
  None: a new window whose tab bar falls under the pointer.
- `Close` of a modified tab (deck, or template set being edited):
  the UI re-reads the view then asks — native dialog Enregistrer / Ne pas
  enregistrer / Annuler (Save / Don't Save / Cancel); "Enregistrer" (Save) without a path opens the save
  dialog.
- Native closing of a window (button, system ⌘W, Alt+F4): the host
  always prevents it. The last window: the host quits keeping everything.
  Another one: the host notifies the page (`close-requested`), which asks the same
  question for each of its modified tabs, then `CloseWindow`. A page
  not ready yet: the window does not close (nothing is lost). ⌘Q
  asks nothing.
- App menu on macOS: Tauri's without "Fermer la fenêtre" (Close Window), whose
  ⌘W would take precedence over the page. Titles "Édition" (Edit) and "Fenêtre" (Window) via Fluent.
- Single instance: `tauri-plugin-single-instance` (all systems: a
  second instance forwards its arguments) and `RunEvent::Opened` (macOS,
  Finder) → `OpenTab` in the last active window, brought to the front,
  off the main thread. An `Opened` arriving before startup ends waits
  in a queue (`pending.rs`, capped at 64 files per call: the excess
  is logged and dropped, against an abusive argument or an abnormally long macOS
  list), drained at the end of `setup`. Only existing
  files are kept. On Linux, `tauri-plugin-single-instance` 2.4.5
  does an `unwrap()` on the D-Bus session bus and panics if there is none
  (headless CI, container without D-Bus); the plugin is only loaded if
  `DBUS_SESSION_BUS_ADDRESS` is set or `$XDG_RUNTIME_DIR/bus` exists
  (`XDG_RUNTIME_DIR` alone does not prove a bus is running; always loaded
  elsewhere), otherwise the single instance is simply disabled (logged
  as `warn`).
- Windows created by `WebviewWindowBuilder` with their label;
  `tauri-plugin-window-state` restores size and position by label (those
  of the old `main` label are lost once). `tauri.conf.json` no longer
  declares a window; the capability covers `w-*`, without
  `allow-destroy`: only the host closes windows.
- Dev server: `POST /api` receives `{ tab, req }`, `POST /api/workspace`
  a `WorkspaceRequest`; no event (a single window: the UI re-reads the
  view after its own commands). It refuses (403) a request whose
  `Host` header is not a loopback name (`localhost`, `127.0.0.1`, `[::1]`,
  any port) or whose `Origin` is not one: a web page cannot
  drive it (CSRF, DNS rebinding), but `just web` (Vite on 5173)
  passes. A request without `Origin` is accepted on purpose (a client that
  never sends this header is not a browser that a CSRF can drive,
  and a browser always sends `Origin` on a `POST`). The body of an
  `/api*` request is capped at 64 MiB; beyond that, `413`. In the browser,
  window actions (new window, move) are hidden and the
  last tab does not close.

## Interface

- `main.tsx` mounts `Workspace.tsx`: tab bar in the title area
  (`Overlay`, to the right of the macOS window buttons) and the `App` of the active
  tab, `key={tab}`. Only the active tab is mounted: current slide and
  grid come back from the host (t-87). Opening a file always changes
  the active tab, which remounts the `App`.
- `App` receives `tab`; `tabApi(tab).*` replaces `api.*` (also in
  `TemplatePicker`, which receives the API of its tab).
- Tab: title, dot if modified, close button; context menu
  "Nouvelle fenêtre" (New Window), "Déplacer vers <fenêtre>" (Move to <window>), "Fermer" (Close),
  "Fermer les autres" (Close Others).
- Dragging within the bar: reorder. Dragging out of the window (in
  any direction): on drop, `drop_tab`. In development
  (browser), nothing. In RTL (`document.dir`), the bar is shown right to
  left (the array order does not change): the slot targeted by the pointer
  is computed mirrored (`slotAt(mids, x, rtl)`).
- Shortcuts: ⌘T new tab, ⌘W close the tab, ⌘⇧N new window,
  ⌃Tab / ⌃⇧Tab next / previous tab, ⌘1…⌘9 tab n (⌘9: the
  last one), ⌘O open (opening rule). ⌃ replaces ⌘ outside macOS. Letters
  read by character (`key`): ⌘Z stays "undo" in AZERTY. Digits by
  physical key (`code`): ⌘1 works in AZERTY. Repeats ignored;
  during text input, only ⌃Tab stays active.
- New texts via Fluent, in the 20 locales.

## Errors

- `UnknownTab { id }`, `UnknownWindow { label }` (race between a close
  and a request), `Unsaved { title }` (close refused): translated;
  the UI re-reads the `workspace` view.
- Failure to write `workspace.json`: logged (`warn`), never returned
  to the UI, like the v1 resume.

## Tests (TDD)

- `workspace_tests` (unit): new tab; workspace created
  with a file; opening an already open file (even written differently)
  activates it; blank tab replaced, modified or saved tab kept;
  close (neighbors, last tab, `Unsaved`, only templates modified);
  close a window (nothing is closed if a tab is refused);
  reorder; move (other window, new window, same window,
  unknown window); window under a point (a single one, overlap, outside
  any window, unknown window); poisoned lock recovered; request
  and close in parallel without deadlock; messages; id
  validation; construction overlapping `settle` (refused, put in the
  trash); new tab on a resume leftover (file kept).
- `tests/session.rs`: tab info (blank, modified, saved, JSON
  opened, templates); notice shown once; a template set
  saved in one session is seen in another.
- `tests/workspace.rs` (resume): full round trip (2 windows, 3
  tabs, active ones, one tab in templates mode, history and `undo`);
  corrupted `workspace.json`; one corrupted tab out of three (kept, set
  aside at its first request, with its notice); lazy restoration
  (titles without read-back, inactive tab read back in full, info written at
  each change, closing a sleeping tab that is modified or whose info
  lags behind, malformed `info` entry, file without `info`, opened file that
  replaces the restored blank tab); tab listed
  without folder; tab listed without state kept across two launches; window
  refused; altered ids; counters too low or too high;
  orphans deleted, set-aside and foreign folders kept; files
  at launch (one file twice, unreadable file and its notice).
- `storage_tests`: `resolve`, `list_dirs`, `delete_dir`, simultaneous
  atomic writes.
- Dev server (`cli.rs`): routing by tab, workspace, unknown
  tab, foreign origin, DNS rebinding, Vite origin; closing the only
  window or its last tab (409, nothing is closed); body over
  64 MiB (413).
- `src-tauri` (unit, what can be tested without a window): `main_tests`
  (launch files, presence of the D-Bus bus), `commands_tests`
  (request classification, view order), `pending_tests` (queue of
  files received at startup, cap), `windows_tests` (drop point).
- UI (`bun test`): drop slots, shortcuts (AZERTY included),
  navigation, closing (dialog choice, saving before closing),
  concurrent responses (`latest.ts`: an outdated response is not
  applied).
- e2e (Playwright, dev server): open tabs, switch, close;
  question for a modified tab.
- Manual (the rest of `src-tauri`, native windows): two screens, drag
  a tab from one window to another and outside the windows; second instance
  with a file, including an unreadable file (its notice is shown
  immediately); `open -a` on macOS; closing a modified tab;
  last window; relaunch; AZERTY keyboard; `just web`.

## Known limits

- "Enregistrer sous" (Save As) to a file already open in another tab
  is not detected: two tabs then target the same file, and the
  last one saved wins. Opening that file afterwards activates one of the
  two.
