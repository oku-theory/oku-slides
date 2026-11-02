# Agent working directory and received packs validation

Tasks t-1413 (an agent working directory isolated from the deck folder) and t-1414 (a
capability manifest and the user's validation before a received pack is installed). Both
are the user's hardening priorities of 2026-10-31 (1 and 4). t-1414 generalizes the install
dialog of t-1267 (`docs/superpowers/specs/2026-10-31-agent-guides-design.md`, decision 10),
now on `main` (`guide_install.rs`, `GuidesInstallDialog.tsx`).

Status: **approved by the user on 2026-11-01, every recommendation kept.** Each open question carries a
recommendation (**R**); the plan follows the recommendations the user keeps.

## Part 1 — t-1413: the agent's working directory

### Today

- `TabAgent::setup` (`crates/oku-slides-agent/src/tab_agent.rs` ~336) passes
  `Session::document_dir()` (the deck's folder) as the `cwd` of `session/new`,
  `session/resume` and `session/load`; an unsaved deck falls back to `Launch.fallback_cwd`,
  which `AgentChoice::launch_of` sets to **the data directory itself**.
- The subprocess is spawned without `current_dir` (`link.rs` ~419): it inherits the app's.
- The client advertises no `fs` and no `terminal` (`link.rs` ~638) and every permission
  request reaches the user (`link.rs::permission`). But an external agent (Claude, Codex,
  Gemini) still has its **own** tools (read, write, shell) that act on disk, held back only
  by its own permission prompts.
- No preset in `crates/oku-slides-platform/src/config.default.toml` passes a flag that skips
  permission requests (`claude-agent-acp@0.81.2`, `codex-acp@2.2.2`,
  `gemini-cli@0.63.0 --acp`, `local`): checked. The app never sets an ACP session mode.

### Why the cwd matters more than "relative paths"

1. **Project config is read from the cwd.** Coding agents load instructions and settings
   from their working directory and its parents: `CLAUDE.md`, `.claude/settings.json`
   (permission allow-lists, `defaultMode`, **hooks that run shell commands**), `AGENTS.md`,
   `.codex/`, `GEMINI.md`, `.gemini/settings.json`. A deck opened from Downloads or a shared
   or unzipped folder puts whatever sits next to it in charge of the agent: prompt
   injection at best, permission bypass or code execution at worst. *To verify in the plan:*
   which of these `claude-agent-acp` (its `settingSources`) and the two others actually load
   in ACP mode; the recommendation holds either way.
2. **The fallback is the data directory.** An unsaved deck's agent starts in the folder that
   holds `config.toml` (the agent preset's `command`: rewrite it and the next launch runs
   anything) and `guide-approvals.json` (approve a hostile guide). One permission click on a
   plausible-looking write is enough.
3. Relative paths in prompts ("the photo next to the deck") resolve in the deck folder.

An empty, app-owned directory removes 1 and 2 and costs 3. It does **not** confine the
agent: its tools still accept absolute paths. That is the later macOS sandbox step
(out of scope, see Follow-ups).

### Decisions to make

**Q1. Where does each tab's working directory live?**
- (a) Inside the tab's resume directory: `<data>/…/tabs/<tab>/work/`.
- (b) A separate root, `<data>/agent-work/<tab>/`, with its own sweep of orphans.
- (c) A temporary directory (`tempfile`), new per launch.

**R: (a).** Stable across restarts (a must: Claude Code files its sessions by cwd, so a
changing cwd breaks `session/resume`), created with `Storage::create_dir_all`, and deleted
for free when the tab's directory is (`workspace_tabs.rs`, `workspace_resume.rs` already
`delete_dir` it). The directory holds nothing at creation; its parent holds the tab's
resume files, but no file an agent loads as config (the app never writes `CLAUDE.md`,
`AGENTS.md` or a dot-folder there; a test asserts the tab's directory tree holds none). (c)
breaks resume; (b) duplicates a lifecycle that exists.

**Q2. Which API gives it?**
**R:** `Session::agent_work_dir(&self) -> Result<PathBuf>` replaces `document_dir()` (which
goes, it has no other caller): it creates the directory through the session's `Storage`
and returns it; called in `spawn_blocking` from `setup` as today. A failure fails the turn
with a localized error (`AgentError`), never a fallback to another folder (invariant: no
silent fallback). `Launch.fallback_cwd` is deleted with every assignment
(`choice.rs`, `config.rs`, test `common/mod.rs`).

**Q3. Is it ever emptied?**
- (a) Never while the tab lives. (b) Emptied when a new conversation starts in the tab.

**R: (b).** A new thread is a new agent memory: leftover scratch files from an earlier one
are stale input. Kept across resume, where the agent may refer to its own files.

**Q4. The subprocess's own working directory.**
**R:** spawn it with `current_dir` set to an empty app-owned directory,
`<data>/agent-process/` (created through Storage at launch), not the app's inherited cwd
(the repository in `just dev`, `/` in the bundle). Some agents read config from the process
cwd at startup, before any session.

**Q5. Permission modes.**
The presets skip nothing, but an agent's own user config can (`~/.claude/settings.json`
`defaultMode: "bypassPermissions"`, `~/.codex/config.toml` `approval_policy = "never"`,
Gemini's YOLO setting), and `claude-agent-acp` reports session modes (`default`,
`acceptEdits`, `bypassPermissions`, `plan`).
- (a) Do nothing; say it in the help.
- (b) After `session/new` or resume, if the agent reports session modes and the current one
  is not its default-asking one, ask for `default` with `session/set_mode`; log a refusal.
- (c) Refuse to prompt while a bypass mode is current.

**R: (a) now, plus a rule test** that no preset in `config.default.toml` carries a known
bypass flag (`--dangerously-skip-permissions`, `--yolo`, `--full-auto`,
`--ask-for-approval never`, `bypassPermissions`), so a future preset edit cannot slip one
in. (b) relies on mode ids that each agent names its own way; it belongs with the sandbox
follow-up, which stops relying on the agent's prompts at all.

**Q6. What the user loses, and where it is said.**
- Relative paths and files next to the deck: the agent no longer sees them. Images go
  through paste (`paste_image`) or the deck's insert-image flow, as today for every
  unsaved deck.
- A `CLAUDE.md` / `AGENTS.md` the user keeps beside their decks no longer applies. The
  agent's user-level config (`~/.claude/CLAUDE.md` and the like) still does.
- One-time: conversations started before the change resume in a new agent session (cwd
  changed: `Reopened::Refused`, the existing `memory_lost` path), keeping the app-side
  transcript.

**R:** one paragraph in the agent card of `docs/aide/en.json` and `fr.json`: the agent works
in a private empty folder, reaches the deck only through Oku Slides' tools, does not see
files next to the deck (paste images instead), and its own app settings (permission modes)
still apply: keep it asking.

### Testing (part 1)

- Session: `agent_work_dir` creates `tabs/<tab>/work`, is stable across two calls, emptied
  by a new conversation, gone with the closed tab; a Storage failure is an error.
- Agent (fake agent, `E2E_AGENT_LOG`): `session/new`, `session/resume` and `session/load`
  receive the tab's work dir for a saved deck and an unsaved one, never the deck folder or
  the data directory; the child's process cwd is `agent-process`.
- Platform rule test: no preset carries a bypass flag (rstest over the flag list).
- Regression `crates/oku-slides-agent/tests/regressions/issue_1413_cwd_outside_deck.rs`: a
  deck saved next to a `CLAUDE.md` and a `.claude/settings.json`; the session's cwd holds
  neither.

## Part 2 — t-1414: validate received packs and themes before install

### Today

- At open (`deck_file.rs::open`) and merge (`merge.rs`), `Session::receive_packs`
  (`pack_receive.rs` ~73) adds every embedded **library-scope pack** version the library
  lacks, silently, marked `received`. Deck-only packs never enter the library.
- **Themes are never installed**: a deck's themes stay in its sources. (The ticket assumed
  otherwise; Q9 decides.)
- t-1267's dialog (`Snapshot.guides_to_approve`, `GuidesReview`, `ApproveGuides`,
  `LaterGuides`, `GuidesInstallDialog`) lists pinned versions whose non-empty guides are
  `received` and not approved; "Later" sets them aside for the tab, in memory.
- A pack version holds a manifest (`PackManifest`: name, namespace, requires, description,
  tokens, profiles, examples, styles, order, guides) and Typst sources (`widgets()`); no
  assets and no fonts. A theme version holds `ThemeManifest` (extends, token groups, guides).
- The render is closed: `world.rs` serves only the deck, the theme, in-memory assets,
  charts, data and sources; any other path and every `@preview/...` import is `NotFound`.
  Typst runs in the bounded render worker. No test pins this down.

### Decisions to make

**Q7. What does "installed" mean, and what does "Later" leave?**
**R:** a received version enters the user's library only on validation. Until then it lives
in the deck's sources only: the deck renders, edits and saves exactly as now (the render
reads embedded versions), the pack is just absent from the library, the picker of other
decks and update counts. `receive_packs` becomes `Session::received_to_review` (pure
reading) plus `Session::install_received(&[VersionRef])` (the old body, per version).
"Later" keeps today's guide rule: set aside for this tab, in memory; a reopened deck asks
again. Packs du deck shows an "Add to my library" action on such a version, so "Later" is
never a dead end.

**Q8. One dialog for install and guides?**
**R: yes, one dialog, one validation.** It lists every version the open deck holds that is
either (i) a library-scope version the library lacks, or (ii) held but with `received`
guides not approved (today's case). The primary button installs every (i) and approves the
guides of all listed (the user read them in the dialog); its label is "Add to my library"
when any (i) is listed, "Approve guides" otherwise. Renames, breaking as MVP allows:
`guides_to_approve` → `received_to_review`, `GuidesReview` → `ReceivedReview`,
`GuidesInstallDialog` → `ReceivedDialog`, `LaterGuides` → `LaterReceived`, new
`Request::AcceptReceived { versions }`; `ApproveGuides` stays for the Guides section and
Packs du deck buttons. All-or-nothing: a deck comes from one sender; per-version
checkboxes are left out (say if you want them).

**Q9. Themes: install them too?**
- (a) Yes: a received theme version, on validation, enters the theme library
  (`ThemeLibrary::receive_version`, the generic `Library` path, marked received), so it
  shows in the theme picker of other decks.
- (b) No: themes stay in the deck; the dialog lists them only for their guides, as now.

**R: (a).** One model for everything a deck brings, as the ticket asks ("covers received
themes too"), and the user gets the theme they were sent. A theme's parents (`extends`) are
listed and installed with it.

**Q10. What is the capability manifest?**
**R:** a pure function in core, `oku_slides_core::Capabilities::of(&PackFiles)`, from the
version alone (no library, no Typst), in a new `capabilities.rs`:

| Field | Packs | Themes |
|---|---|---|
| kind, name, namespace, description (through `bounds::label` in session) | ✓ | ✓ |
| items: count and names (widgets or layouts, `_commun` excluded) | ✓ | — |
| requires (packs) / extends (themes) | ✓ | ✓ |
| component tokens: count / token groups: names (`color`, `font`, …) | ✓ | ✓ |
| font families its tokens name (they come from the app's fonts, never the version) | ✓ | ✓ |
| layout profiles used by import: count | ✓ | — |
| Typst code size (KiB) | ✓ | — |
| guides (as the agent reads them, t-1267) | ✓ | ✓ |

The session adds what depends on the user: `relation` (new pack, new version of a pack you
have, version you already have), `origin`, and an **install blocker** when
`receive_version` would refuse (namespace taken by another of your packs, reserved, changed
namespace): such a version is shown as "stays in this deck: <reason>", never installed,
and does not block the others. The dialog shows each manifest as plain text, as the guides
are.

"Anything that can reach outside the render" (ticket): today nothing can, which the
regression test of Q12 proves. **R:** no Typst source scan now (it would need parsing
untrusted Typst outside the worker and could only hint); the dialog states the guarantee in
one line ("Packs and themes cannot read your files or the network"). A future capability
that breaks this adds its own manifest field and dialog line.

**Q11. Other paths that install.**
**R:** the CLI (`oku-slides open/render/import/serve`) and any headless path never install:
received versions stay in the deck. A pack tab opened on a not-yet-installed received
version (from Packs du deck) first shows the same dialog for that version; saving it is
an install. Versions received before this change stay in the library (no migration): they
were installed under the old rule, and their guides still need approval as today.

**Q12. The offline-render regression test.**
**R:** `crates/oku-slides-render/tests/regressions/issue_1414_render_stays_offline.rs`, an
rstest table of widget sources that try to leave: `#import "@preview/cetz:0.3.4"`,
`#read("/etc/hosts")`, `#image("/System/Library/Desktop Pictures/…")`,
`#read("../../../../etc/passwd")`, `#plugin("/tmp/x.wasm")`, `#read("https://example.com")`;
each render fails with a `NotFound` file or package error naming the path, in bounded time.
Plus a dependency check in `just check`: `cargo tree -p oku-slides-render-worker -e normal`
holds no HTTP client (`ureq`, `reqwest`, `hyper`) and no `typst-kit` `downloads` feature.

### Testing (part 2)

- Core: `Capabilities::of` on a widgets pack, a layouts pack and a theme (rstest), names
  and counts exact, `_commun` excluded.
- Session: opening a deck that brings a new library pack leaves the library unchanged and
  lists it; `AcceptReceived` installs it (marked received) and approves its guides;
  `LaterReceived` empties the list for the tab, a reopened tab lists it again, the deck still
  renders; a blocked namespace is listed with its blocker and never installed; a merge
  bringing a version lists it; a theme and its parents install on validation; built-in,
  local and deck-only versions are never listed for install.
- Regression `crates/oku-slides-session/tests/regressions/issue_1414_silent_install.rs`: a
  received pack is not in the library after open.
- UI render test of `ReceivedDialog` (button label switch, blocker line, plain-text
  manifest); e2e: open a deck with a received pack, see the dialog, "Plus tard" leaves the
  library without it, "Ajouter à ma bibliothèque" from Packs du deck installs it.
- Rules: `crates/oku-slides-session/AGENTS.md` gains "a received version enters the
  library only through `install_received` after the user's validation"; the render crate's
  `AGENTS.md`, "keep the world closed: no file outside memory, no package".

## Deviations recorded after the review round (2026-11-02, approved by the user)

- **Q11, pack tab:** a pack tab opened on a received version not yet installed is refused
  (`PackNotInstalled`) instead of showing the dialog; Packs du deck offers "Add to my
  library" for it and hides Open. One way in, and a tab never shows a half-installed pack.
- **Q10, `origin`:** dropped from `ReceivedReview`: every listed version is a received one,
  so the field would always say the same thing.
- **Q12:** typst-kit's network and package *features* are banned rather than the crate (the
  worker uses its font loading); the `..` case expects Typst's "would escape the project
  root" error, raised before the world is asked. Cases that could not fail were replaced
  by real files on disk.
- **Help:** only the agent's folder gets a help card (`b-agent-dossier`); the received
  dialog explains itself in place.
- **Blocked versions:** answering the dialog sets aside every listed version, blocked ones
  included, so it always closes; a blocked version stays in the deck.
- **Capability lists** are capped at 50 names each, with the total shown.
- **Q2, `agent_work_dir`:** returns an `AgentWorkDir` (`path`, `prepare`, `empty`) rather
  than a bare path; `prepare` also removes the agents' config names (`.claude`, `.codex`,
  `.gemini`, `.mcp.json`, `AGENTS.md`, `AGENTS.override.md`, `CLAUDE.md`,
  `CLAUDE.local.md`, `GEMINI.md`) before every new, resumed or loaded conversation, and
  `agent-process` is emptied at each launch, so nothing an agent wrote configures the next.
- **Emptying on Forget** runs in the keeper after the session lock drops, not under it.
- **`serve`:** each process works in its own random folder under `<data>/serve`, removed
  when it stops, instead of emptying `<data>/serve`: two dev servers kept wiping each
  other's tabs.

## Out of scope / follow-ups

- The agent can read other received text (labels, Typst source of slides) before the user
  validates: the dialog guards the library and the agent's guides, not the deck's content.

- macOS sandbox profile for the agent process (no writes outside its work directory,
  network only to its API): its own task once this lands.
- Forcing a non-bypass ACP session mode (Q5 b).
- Per-version choice in the dialog; persisting "Later" per deck.

## Plan outline (after approval)

1. Part 1: `agent_work_dir`, emptied on new conversation, process cwd, preset rule test,
   regression, help.
2. Part 2 core: `Capabilities::of`.
3. Part 2 session: `received_to_review`, `install_received`, `AcceptReceived` /
   `LaterReceived`, themes, blockers, pack tab gate, regression.
4. Part 2 UI: `ReceivedDialog`, Packs du deck action, Fluent en/fr, e2e.
5. Offline-render regression and dependency check.
6. Docs: help cards, `ARCHITECTURE.md` and `AGENTS.md` updates; review round; `just ci`.
