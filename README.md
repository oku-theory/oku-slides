# Oku Slides

`echo "an idea" > deck`: what you say becomes slides. Oku Slides creates and edits
presentations by voice. The document is a PDF (`.oku-slides.pdf`) that carries its
own sources: any viewer displays it, and Oku Slides reopens it to keep editing.

In the code: crates `oku-slides-*`, the `oku-slides` tool, the `oku-slides` Typst
library.

Implementation plan: [Oku Slides — implementation plan](https://claude.ai/code/artifact/fca9973c-7aff-454e-b40b-9a900dedaf57).
Design decisions: [`docs/adr/`](docs/adr/README.md). All documentation is written in
English; older French documents are translated when next edited. Code comments and tool
output are in French.

## Repository layout

| Path | Role |
| --- | --- |
| `crates/oku-slides-core` | Deck model, reversible operations, history, validation, Loro replica, source bundle |
| `crates/oku-slides-render` | Embedded Typst: one SVG per slide, PDF, element boxes |
| `crates/oku-slides-render-worker` | Typst rendering in a worker process, bounded in time and memory |
| `crates/oku-slides-chart` | Charts and tables: ECharts option subset, SVG via charts-rs, table model, data operations |
| `crates/oku-slides-pdf` | Source blob inside the PDF (`/PieceInfo`): write, read, strip |
| `crates/oku-slides-import` | PPTX import: `SourceDeck` intermediate representation, reader, retrofit and high-fidelity projectors, font resolution, report and content coverage |
| `crates/oku-slides-session` | Editing session (open deck, history, rendering, file), shared by the app and the dev server |
| `crates/oku-slides-platform` | Foundation: logging, settings, storage, languages; bounded accept loop for local HTTP servers |
| `crates/oku-slides-i18n` | Fluent messages (20 languages, English as the reference) and language selection |
| `crates/oku-slides-stt` | Dictation: microphone, voice detection, Apple's engine (macOS 26+) or local Whisper (CTranslate2) |
| `crates/oku-slides-a2ui` | A2UI v0.9 surfaces (component catalog), shared by the panel and the agent |
| `crates/oku-slides-mcp` | Agent MCP tools (local HTTP, token): reading, `apply`, surfaces, confirmations |
| `crates/oku-slides-agent` | Voice agent: ACP client (agent as a subprocess), turns, stack for the app and the server; fake agent for tests |
| `crates/oku-slides-cli` | `oku-slides` tool: render, save, reopen, PPTX import, benchmarks, dev server |
| `src-tauri` | Tauri 2 desktop app |
| `ui` | React + TypeScript interface (Papier design system) |
| `assets/fonts` | Bundled fonts (Newsreader, Instrument Sans, Inter; Noto Sans/Serif for non-Latin scripts; Carlito, Caladea, Liberation as metric-compatible substitutes for imports), in Git LFS |
| `typst/oku-slides` | Typst slide library (structure, default tokens, built-in `papier` and `pro` themes as JSON) |
| `templates/` | Built-in template sets (`essentiel`, `grilles`) |
| `fixtures/` | Sample decks, agent scripts, import, dictation and widget samples |
| `docs/aide/` | In-app help: one JSON deck per language (fr, en), compiled into the app, opened read-only (⌘?) |
| `docs/adr/` | Architecture decision records |

## Command-line tool

```sh
just oku-slides check fixtures/roadmap.json                         # validate a deck
just oku-slides render fixtures/roadmap.json --out out --theme pro  # SVG + layout.json; theme by name
just save fixtures/roadmap.json deck.oku-slides.pdf                 # save as .oku-slides.pdf
just oku-slides open deck.oku-slides.pdf --out extracted/           # deck.json + assets
just oku-slides strip deck.oku-slides.pdf copy.pdf                  # copy without sources
just oku-slides import deck.pptx --pack Essentiel --theme Pro -o deck.oku-slides.pdf --report r.json  # PPTX → deck (retrofit)
just oku-slides import deck.pptx --mode hifi -o deck.oku-slides.pdf --report r.json                    # PPTX → deck (faithful copy)
just bench                                                          # render and save latencies (release build)
just oku-slides schema                                              # JSON schema of operations
```

`--fonts <dir>` adds fonts (TTF/OTF); without it, Typst uses its built-in fonts
(Libertinus Serif, New Computer Modern, DejaVu Sans Mono).

`import` maps the content of a `.pptx` onto an existing pack (`--pack`: id, pack
folder or name) and theme (`--theme`: id or name; default: the default theme):
the style comes from the target theme. Titles are inferred when the slides have no
placeholders, drawn cards become `card` elements, repeated headers and footers are
left to the theme. `--report` writes the JSON report: template chosen per slide,
losses, fallbacks, widget requests, reader warnings and content coverage.
`--mode` defaults to `retrofit`.

`--mode hifi` reproduces the presentation on a theme and a pack generated for it
(one slide widget per PowerPoint layout, master and layout decoration drawn in
Typst, placeholders as editable fields), embedded in the deck only: the library is
left untouched, and `--pack` and `--theme` are refused. Pages stay 1920 × 1080; a
source of another ratio is scaled and centered. Fonts come from the presentation,
then from the app (`--fonts`) and the system (unless `--no-system-fonts`), then from
a metric-compatible substitute (Calibri → Carlito, Cambria → Caladea, Arial,
Times New Roman, Courier New → Liberation); presentation and system faces are
embedded in the deck. The report adds the font decisions.

## Running the app

Everything runs from the repository root with [just](https://just.systems).
Prerequisites: Rust, [Bun](https://bun.sh), just, cmake (for CTranslate2), the Tauri
CLI (`cargo install tauri-cli --version "^2"`), on a Mac Xcode with the macOS 26 SDK
(required: the Swift bridge to Apple's dictation engine is always built), and [Git LFS](https://git-lfs.com) for the fonts in
`assets/fonts` (`git lfs install` before cloning, otherwise `git lfs pull`).
The `typst` CLI is optional (`just typst`). `just doctor` checks everything.

```sh
just dev      # desktop app in development (installs the interface dependencies)
just build    # .app / .dmg bundle
just web      # interface in the browser with hot reload (localhost:5173)
just serve    # built interface in the browser (127.0.0.1:4870)
just test     # Rust and interface tests
just e2e      # end-to-end tests (Playwright) against a fresh server, port 4872
just check    # format, clippy, TypeScript types, LFS fonts, AGENTS.md harness (needs a git clone)
just          # every command
```

`just e2e 4999` picks another port when one is already taken; `just e2e` keeps its data
in a temporary folder, removed at the end. When running the app or the Playwright
scripts directly, point `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a
temporary directory to keep your own data untouched.

## Using the app

### The workspace

- **Pile** (left): slide thumbnails. Click to show, drag to reorder (or ⌥↑ / ⌥↓),
  `+` adds a slide from a template.
- **Sheet** (middle): the slide exactly as it prints; elements are selected, moved
  and edited right on it.
- **Thread** (right): the conversation with the agent and the dictation draft. The
  menu at the top right holds everything rarely used.

### Editing by hand

| Gesture | Effect |
| --- | --- |
| Click | Select an element or a field (title, kicker…) |
| Drag, handles | Move or resize a free element, cell by cell |
| Double-click or Enter | Edit the text (one line per paragraph, `- ` for a bullet) |
| Arrows (Shift: 10 pt) | Move the selected free element by one cell (by 1 pt if it is placed by frame, off the grid) |
| Delete | Delete the selected element |
| ⌘C, ⌘X, ⌘V | Copy, cut, paste an element; pasted text becomes a text element |
| ⌘Z, ⌘⇧Z | Undo, redo (one gesture = one undo) |
| ⌘S, ⌘⇧S, ⌘O | Save now, save a copy, open |
| ⌘T, ⌘W, ⌘⇧N | New tab, close tab, new window |
| ⌃Tab, ⌃⇧Tab | Next, previous tab |
| ⌘⇧G | Show the ghosts |
| ⌘K | Type an instruction for the agent |
| ⌘1…⌘9 | Go to tab n (⌘9: the last one) |
| ⌘? | App help (read-only deck in French or English, otherwise English; `docs/aide/`) |

The bottom bar adds a text, a card, a shape, a chart, a table or a widget from the
deck's template set.

### Charts, tables, widgets

- **Charts**: a subset of ECharts options rendered to SVG (`oku-slides-chart`); the
  data is edited in a grid in the side panel.
- **Tables**: the same data grid; header row, first column as header, striped rows
  and per-column alignment (automatic puts numbers at the end). A chart converts to
  a table and back.
- **Widgets**: template sets can ship Typst widgets. In template mode, the Widgets
  section of the pile opens their code next to a live preview. Rendering runs in a
  separate worker process, bounded in time and memory (`[render]` settings); past
  the limit, the deck falls back to the base widgets.

### Grid, templates, themes

- **Grid**: every slide has a grid (12 × 6 by default), set in its template (template mode,
  template settings); slides created from the template inherit it.
  Snapping is always on: an element covers a block of cells, moves cell by cell
  (mouse or arrows), and each corner snaps to the nodes. ⌘' (or menu ▸ Show the grid)
  shows the nodes; during a drag they show anyway. The geometry is computed
  identically by `ui/src/grid.ts` (editing) and `grid-frame` in `typst/oku-slides/src/zone.typ`
  (rendering).
- **Template sets**: “New slide” templates are grouped into sets (built-in
  `templates/<id>/pack.json`: Essentiel, Grilles; user sets in the app data folder).
  A set does not depend on the theme: a deck picks its set and its theme
  independently.
- **Editing templates** (menu ▸ Workshop): each template becomes a slide edited like
  any other (grid, elements, texts), with its name and description. A built-in set is
  saved as a copy; the deck is set aside and comes back with “Done”.
- **Themes**: a theme is a set of design tokens (colors, fonts, sizes, margins,
  components), versioned like template sets. Papier and Pro are built in
  (`typst/oku-slides/themes/*.json`); user themes go to the `themes/` folder of the
  app data. The menu picks the deck's theme; the deck embeds its version, and a
  received file keeps its own (“File theme”). In template mode, the Theme tab sets
  tokens one by one (filter, state of each token, ↺): the sheet follows every change,
  and “Done” saves the draft, as a copy (“Papier (copy)”) for a built-in theme.

### Files, tabs, history

- **Automatic saving**: every change goes to the file; a new deck goes to
  `Documents/Oku Slides` (`[paths] decks`).
- **Tabs and windows**: one deck per tab, several windows; everything comes back at
  next launch.
- **History** (menu ▸ History): a slider walks every version stored in the file;
  Restore makes one current again.
- **Merge** (menu ▸ Merge with a PDF…): two copies of the same deck, edited
  separately, merge without conflicts. Deck sources are a Loro CRDT (see
  [ADR 0002](docs/adr/0002-sources-du-deck-en-crdt-loro.md) and
  [ADR 0005](docs/adr/0005-elements-d-une-slide-en-arbre-loro.md)).

### Dictation

Everything happens on the machine. On macOS 26+, Apple's speech engine is the default when it knows the language
(streaming, language installed by the system; `[stt] apple = false` turns it off).
Otherwise, Whisper runs on CTranslate2 (faster-whisper's engine, through
[ct2rs](https://crates.io/crates/ct2rs)), int8-quantized weights, Accelerate on Mac.
On the first press of the mic, the chosen Whisper model downloads once, then
listening starts on its own.

| Model | Size | Use |
| --- | --- | --- |
| Apple (macOS 26+) | — | Default when it knows the language; streaming |
| Large v3 Turbo | 818 MB | Default otherwise; fast and accurate, multilingual |
| Large v3 | 3.1 GB | Most accurate, multilingual, slower |
| Base | 148 MB | Lightweight, for small machines |

- **Voice detection** ([earshot](https://crates.io/crates/earshot)): only spoken
  passages are transcribed; listening stops by itself after a silence (1.5 s by default, adjustable,
  can be turned off), or after 10 s if nothing is said.
- **Sentence by sentence**: at each pause, the sentence that just ended is
  transcribed at final quality, with the text dictated so far and the deck's
  vocabulary as context. The preview only re-transcribes the current sentence. On
  stop, only the last sentence remains to process.
- **Language**: forced, or detected on the first sentence and then kept.

Models go to the app data folder
(`~/Library/Application Support/me.dartiguenave.oku-slides/models` on Mac);
`OKU_SLIDES_PATHS__MODELS` overrides it (see [Settings](#settings)).

| Gesture | Effect |
| --- | --- |
| ⌘D or the mic | Start, then stop dictation |
| ⌘⇧D | Dictate into the draft, even while editing a text |
| Esc | Cancel listening or the download, stop the agent's turn |

While editing a text, ⌘D writes at the cursor. Elsewhere, dictation goes into a
**draft** in the right panel: you review it, fix it or extend it (each dictation is
appended), then choose:

- **Insert** (⌘⇧↩) writes it as is into the selected area (a field is replaced, a
  block is extended), otherwise into a new block;
- **Send to the agent** (⌘↩) turns it into an instruction (see [Agent](#agent)).

A pause never decides for you. Each insertion undoes with ⌘Z.

```sh
just transcribe voice.wav turbo fr                # split into sentences, transcribe, measure
OKU_SLIDES_STT__FAKE_MIC=voice.wav just serve     # simulate the mic in the interface
just test-apple                                   # Apple's engine for real (macOS 26, French installed)
```

### Agent

An agent edits the presentation on instruction. The instruction is dictated like any
text, or typed (⌘K), into the [draft](#dictation); **Send to the agent** (⌘↩) hands it over
[ACP](https://agentclientprotocol.com) to an agent launched by Oku Slides. Oku Slides
gives it no file or terminal access (ACP `fs` and `terminal` capabilities off): it
works through Oku Slides' tools, served over MCP on `127.0.0.1` with a token per tab.
The agent process itself runs with your user rights; any tool of its own it wants to
use goes through a permission request you answer. Its instructions are in English; the interface and
deck languages are passed in its context, and it answers in yours.

- **Everything undoes**: one agent turn is a single ⌘Z entry.
- **What cannot be undone** (saving, opening, deleting a template set, changing a
  setting) waits for your confirmation, like the permissions the agent requests: a
  click, never Enter, and never “always allow” by default.
- **Decisions**: the agent asks its questions in a card under the element (choices,
  rewordings: `1`–`4`, Enter, `E` to edit, Esc), proposes variants rendered in a
  drawer under the sheet, and live settings in a sticky note. A question can be
  answered by voice, through the draft.
- **Conversation**: one per tab, restored with the tab at next launch; tabs work at the
  same time. “New conversation” starts from scratch. ⌘⇧D during a turn stops it.
- **Detached window**: ⧉ at the top of the thread (or Window ▸ Agent in a Separate Window, on macOS) opens
  the agent in its own window, for a second screen. It follows the active tab of the
  front window.
- **Choosing the agent**: the settings menu of the dictation bar lists, at its bottom, the agents of the
  configuration (`[agent.presets]`); picking one saves it and switches at once (a turn in
  progress stops). Add an agent in `config.toml`; `OKU_SLIDES_AGENT__SELECTED` imposes the
  choice. The agent itself cannot change it.

| Setting | Default |
| --- | --- |
| `[agent] selected` / `OKU_SLIDES_AGENT__SELECTED` | `claude` |
| `[agent.presets.<id>]` `name`, `command`, `args` | Claude, Codex, Gemini (`npx`, pinned versions) |

Any ACP agent that accepts MCP servers over HTTP works.

```sh
just agent-smoke fixtures/roadmap.json "Make the title of slide 2 punchier"   # real agent (network, account)
```

## Settings

Settings come in three layers, the last one wins:

1. the defaults, embedded and commented
   ([`config.default.toml`](crates/oku-slides-platform/src/config.default.toml));
2. `config.toml` in the app data folder, optional;
3. environment variables `OKU_SLIDES_<SECTION>__<KEY>`, with a double underscore to
   go down one level.

| Variable | Effect |
| --- | --- |
| `OKU_SLIDES_PATHS__DATA` | App data folder |
| `OKU_SLIDES_PATHS__DECKS` | Where new decks are saved automatically (default `Documents/Oku Slides`) |
| `OKU_SLIDES_PATHS__MODELS` | Dictation models folder |
| `OKU_SLIDES_PATHS__FONTS` | Extra fonts (TTF/OTF) |
| `OKU_SLIDES_STT__APPLE` | `false`: Whisper only |
| `OKU_SLIDES_STT__FAKE_MIC` | WAV file replayed instead of the microphone |
| `OKU_SLIDES_STT__VAD__PAUSE` | Pause that ends a sentence (s), like any `[stt.vad]` key |
| `OKU_SLIDES_SERVE__PORT` | Dev server port |
| `OKU_SLIDES_I18N__LOCALE` | Interface language (`en`, `fr`, `de`, `ja`…); default: the system's |
| `OKU_SLIDES_APPEARANCE__SCHEME` | `light` or `dark`; default: the system's |
| `OKU_SLIDES_AUTHOR__NAME` | Name recorded in the history of decks you edit |

The interface ships in 20 languages (`crates/oku-slides-i18n/src/locales`), English
being the reference.
