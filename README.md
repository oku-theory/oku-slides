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
| `crates/oku-slides-import` | PPTX import: `SourceDeck` intermediate representation, reader, retrofit and high-fidelity projectors, font resolution, report, content coverage and visual gap |
| `crates/oku-slides-reference` | Reference rendering of an imported presentation (`office2pdf` or LibreOffice witness, run by the render worker) and `hayro` rasterization of its pages |
| `crates/oku-slides-session` | Editing session (open deck, history, rendering, file), shared by the app and the dev server, and the PPTX import pipeline the CLI and the app share |
| `crates/oku-slides-platform` | Foundation: logging, settings, storage, languages; bounded accept loop for local HTTP servers |
| `crates/oku-slides-i18n` | Fluent messages (20 languages, English as the reference) and language selection |
| `crates/oku-slides-stt` | Dictation: microphone, voice detection, Apple's engine (macOS 26+) or local Whisper (CTranslate2) |
| `crates/oku-slides-a2ui` | A2UI v0.9 surfaces (component catalog), shared by the panel and the agent |
| `crates/oku-slides-mcp` | Agent MCP tools (local HTTP, token): reading, `apply`, surfaces, confirmations |
| `crates/oku-slides-agent` | Voice agent: ACP client (agent as a subprocess), turns, stack for the app and the server; fake agent for tests |
| `crates/oku-slides-local-agent` | Built-in fallback ACP agent (`--local-agent`): routing by tool families, bounded MCP tool loop, final `report`; scripted fake engine only (lot 1), no account or network |
| `crates/oku-slides-cli` | `oku-slides` tool: render, save, reopen, PPTX import, benchmarks, dev server |
| `src-tauri` | Tauri 2 desktop app |
| `ui` | React + TypeScript interface (Papier design system) |
| `assets/fonts` | Bundled fonts (Newsreader, Instrument Sans, Inter; Noto Sans/Serif for non-Latin scripts; Carlito, Caladea, Liberation as metric-compatible substitutes for imports), in Git LFS |
| `typst/oku-slides` | Typst slide library (structure, default tokens, built-in packs `base` (widgets) and `essentiel` (slide layouts), built-in `papier`, `pro` and `neutre` themes as JSON, each with a namespace and an ordered `extends`) |
| `fixtures/` | Sample decks, agent scripts, import, dictation and widget samples |
| `docs/aide/` | In-app help: one JSON deck per language (fr, en), compiled into the app, opened read-only (⌘?) |
| `docs/adr/` | Architecture decision records |

## Command-line tool

```sh
just oku-slides check fixtures/roadmap.json                         # validate a deck
just oku-slides render fixtures/roadmap.json --out out --theme pro  # SVG + layout.json; theme by name
just save fixtures/roadmap.json deck.oku-slides.pdf                 # save as .oku-slides.pdf
just oku-slides open deck.oku-slides.pdf --out extracted/           # deck.json, assets, fonts, packs (not embedded themes)
just oku-slides strip deck.oku-slides.pdf copy.pdf                  # copy without sources
just oku-slides import deck.pptx --pack essentiel --theme Pro -o deck.oku-slides.pdf --report r.json  # PPTX → deck (retrofit)
just oku-slides import deck.pptx --mode hifi -o deck.oku-slides.pdf --report r.json                    # PPTX → deck (faithful copy)
just bench                                                          # render and save latencies (release build)
just oku-slides schema                                              # JSON schema of operations
```

`--fonts <dir>` adds fonts (TTF/OTF); without it, Typst uses its built-in fonts
(Libertinus Serif, New Computer Modern, DejaVu Sans Mono).

`import` maps the content of a `.pptx` onto layout packs (`--pack`, repeatable: namespace, id,
pack folder or name; default `essentiel`) and a theme (`--theme`: namespace, id or name; default: the default theme):
the style comes from the target theme. Titles are inferred when the slides have no
placeholders, drawn cards become `base/card` elements, repeated headers and footers are
left to the theme. `--report` writes the JSON report: profile chosen per slide,
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

In hifi mode, `--reference auto` (the default) also renders the presentation with an
independent witness (`[import] witness`: `office2pdf` by default, in its own bounded
render process, 20 slides at a time) and writes to the report a visual gap per slide
(mean OKLab ΔE) with its three worst regions (the source items that differ most, after
a realignment of up to 3 px; regions the witness is known to draw wrong are marked
`suspect` and ranked last); `--reference office2pdf` or `soffice` overrides the setting;
a slide whose worst non-suspect region exceeds `[import] score_threshold` is flagged, and a second
output line sums it up. `--reference none` skips it. In both modes, a text field
rendered past its page becomes an `Overflow` loss; in retrofit, so does any element box
rendered below the page's bottom margin.

The default `[import] score_threshold` is 7.0 (OKLab ΔE ×100 of the worst non-suspect
region, measured with office2pdf on the alto, libreoffice and kitchen fixtures on
2026-10-16). `[import] witness = "soffice"` (or `--reference soffice`) uses LibreOffice
instead, in the app too: the first program of `[import] soffice` found. Missing, or a
script on macOS, the report says why and no slide is scored: never a fallback to
office2pdf. Its own suspect regions are Calibri Light text (it has no substitute) and
SmartArt text; measured on 2026-10-18, its titles reach 6.8, closer to the threshold.

Both modes refuse a `.pptx` larger than `[import] max_input_mib` (default 256 MiB, at
least 1), checked before the file is read so a bigger one never enters memory.

In the app, Open (⌘O) accepts a `.pptx`. A dialog asks for the mode (faithful copy,
or your layouts, with library layout packs and a theme) and offers "Fine-tune with the
agent", checked by default; it is unchecked and disabled, with the reason, when the
chosen agent cannot run (program not found, launch failed, no MCP over HTTP). The import opens a new tab, autosaved into the decks
folder like a new deck (undo removes the import whole) and, when asked, the agent
adjusts the slides the witness flags (`[import] witness`) with the tools
`import_report` and `import_diff`, in bounded tries:
`[import] tune_iterations` (default 3) per slide, `tune_stall` (default 2) tries
in a row without gain, and `tune_global` (default 3) for theme and pack changes,
which change the whole deck.
Hi-fi packs belong to the deck: the pack workshop edits them from the
version the deck embeds and saves them in the deck, never in the library. The hi-fi
import pins the Neutre theme and writes its generated tokens in the deck's `tokens`. A retrofit deck pins
the library layout packs; a widget written for it goes to the deck's own packs, as in any deck.

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
  `+` adds a slide from a layout.
- **Sheet** (middle): the slide exactly as it prints; elements are selected, moved
  and edited right on it.
- **Thread** (right): the conversation with the agent and the dictation draft. The
  menu only presents, opens a file and shows help: everything else, app settings
  included, is asked of the agent, so the app runs without keyboard or mouse.

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
| ⌘⇧G | Show the guides |
| ⌘K | Type an instruction for the agent |
| ⌘1…⌘9 | Go to tab n (⌘9: the last one) |
| ⌘? | App help (read-only deck in French or English, otherwise English; `docs/aide/`) |

The bottom bar adds a text, a card, a shape, a chart, a table or a widget from the
deck's widget packs.

### Charts, tables, widgets

- **Charts**: a subset of ECharts options rendered to SVG (`oku-slides-chart`); the
  data is edited in a grid in the side panel.
- **Tables**: the same data grid; header row, first column as header, striped rows
  and per-column alignment (automatic puts numbers at the end). A chart converts to
  a table and back.
- **Widgets**: packs ship Typst widgets: `base` (text, card, image, shape, chart, table,
  stack) and layout packs such as `essentiel`, whose widgets are whole slides. In pack
  mode, the Widgets section of the pile opens their code next to a live preview. Rendering runs in a
  separate worker process, bounded in time and memory (`[render]` settings); past
  the limit, the faulty pack is disabled.

### Grid, packs, themes

- **Grid**: every slide has a grid (12 × 6 by default), set in its layout (pack mode,
  the slide's settings); slides created from the layout inherit it.
  Snapping is always on: an element covers a block of cells, moves cell by cell
  (mouse or arrows), and each corner snaps to the nodes. ⌘' (or ask the agent)
  shows the nodes; during a drag they show anyway. The geometry is computed
  identically by `ui/src/grid.ts` (editing) and `grid-frame` in `typst/oku-slides/src/zone.typ`
  (rendering).
- **Layout packs**: “New slide” lists the layouts of the layout packs the deck pins
  (built-in `typst/oku-slides/packs/essentiel/layouts.json`: title, contents, chapter,
  content, end, blank; user packs in the `layouts/` folder of the app data). A new slide
  starts from the layout's example values. A pack does not depend on the theme: a deck
  picks its packs and its theme independently.
- **Editing a pack** (ask the agent for the workshop): the deck is set aside and the pack opens as a
  working deck, one slide per layout, seeded from its example; each edit on a slide is
  written to the layout's example at once (one undo step), and saving only saves the
  pack's name and theme. Slides cannot be added, removed or moved there. A built-in pack is saved as a copy; “Done” brings the deck back.
- **Themes**: a theme is a set of design tokens (colors, fonts, sizes, margins, components),
  versioned like packs. Papier, Pro and Neutre are built in (`typst/oku-slides/themes/*.json`);
  user themes go to the `themes/` folder of the app data. A theme has a namespace and an ordered
  `extends` list (parents first, later ones win, cycles refused). The agent picks the deck's
  theme; the deck pins it and its whole `extends` closure, and a received file keeps its own
  (“File theme”). Component tokens are owned by packs (`component.base.*`,
  `component.essentiel.*`); a deck's `tokens` override the theme. “Open” on the theme row of
  “Deck packs” opens the theme in its own tab, on Essentiel's specimen slides: it sets tokens one
  by one (filter, state of each token, ↺) and `extends`, the slides follow every change, and it
  autosaves while it holds the theme; “Done” closes it. A built-in theme saved there becomes a
  variant that extends it. User themes without a namespace are ignored, with a
  notice.

### Files, tabs, history

- **Automatic saving**: every change goes to the file; a new deck goes to
  `Documents/Oku Slides` (`[paths] decks`).
- **Tabs and windows**: one deck per tab, several windows; everything comes back at
  next launch.
- **History** (ask the agent): a slider walks every version stored in the file;
  Restore makes one current again.
- **Merge** (ask the agent to merge the other PDF): two copies of the same deck, edited
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
appended), then **send it to the agent** (⌘↩) as an instruction (see [Agent](#agent)).
The draft only talks to the agent; a pause never decides for you.

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
- **What cannot be undone** (saving, opening, deleting a pack, changing an app setting
  other than the plain dictation ones) waits for your confirmation, like the permissions the
  agent requests: a click, never Enter, and never “always allow” by default.
- **Decisions**: the agent asks its questions in a card under the element (choices,
  rewordings: `1`–`4`, Enter, `E` to edit, Esc), proposes variants rendered in a
  drawer under the sheet, and live settings in a sticky note. A question can be
  answered by voice, through the draft.
- **Conversation**: one per tab, restored with the tab at next launch; tabs work at the
  same time. “New conversation” starts from scratch. ⌘⇧D during a turn stops it.
- **Detached window**: ⧉ at the top of the thread (or Window ▸ Agent in a Separate Window, on macOS) opens
  the agent in its own window, for a second screen. It follows the active tab of the
  front window.
- **Dictation and agent settings**: the ⚙ of the dictation bar opens a box in the agent area,
  live until closed (model, language, auto stop, silence, API, and the agent from the
  configuration's `[agent.presets]`). The agent opens it on request and changes plain
  dictation settings at once; it asks before switching to the API, downloading a model or
  switching agent (a switch waits until no reply runs). The API address (https, or http for
  this computer) and the key variable (a name like `OPENAI_API_KEY`) stay yours, set in the
  box. Add an agent in `config.toml`; `OKU_SLIDES_AGENT__SELECTED` imposes the choice.

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

### The configuration file

`config.toml` lives in the app data folder:
`~/Library/Application Support/me.dartiguenave.oku-slides/config.toml` on macOS, the XDG
data folder on Linux, `APPDATA` on Windows; `OKU_SLIDES_PATHS__DATA` moves it with the rest
of the data. It is optional and only needs the keys you change: everything else keeps the
embedded default, and tables merge key by key (adding `[agent.presets.mine]` keeps Claude,
Codex, Gemini and Local). Settings are read at launch; restart the app after editing the
file. A file that does not parse, or a value of the wrong type, stops the launch with the
offending key rather than being ignored.

```toml
[paths]
decks = "/Users/julie/Slides"   # where new decks are saved (absolute: no ~)
fonts = "/Users/julie/Fonts"    # extra TTF/OTF fonts

[i18n]
locale = "en"                   # absent: the system language

[author]
name = "Julie"                  # recorded in each deck's history

[stt.vad]
pause = 1.0                     # longer pause before a sentence ends (s)

[agent]
selected = "mine"               # must name a preset, or the launch fails

[agent.presets.mine]
name = "My agent"
command = "/usr/local/bin/my-acp-agent"
args = ["--acp"]
```

| Section | What it sets |
| --- | --- |
| `[paths]` | Folders: `data`, `models`, `widgets`, `layouts`, `themes`, `decks`, `fonts` |
| `[history]` | `limit`: transactions kept for undo |
| `[resume]` | `keep_failed`: unreadable restore files kept aside |
| `[sources]` | Size caps of the sources read from a PDF; `compact_after_days` before deletions are purged |
| `[render]` | Typst worker bounds: time, memory, message size, widget files per pack |
| `[import]` | PPTX import: witness, score threshold, LibreOffice paths, agent tuning tries, input size cap |
| `[paste]` | Images pasted into the agent draft: size, pixels, count, what the agent is shown |
| `[serve]` | Dev server `host`, `port`, header timeout and connection cap (also the agent's tool server) |
| `[i18n]`, `[appearance]`, `[author]` | Interface language, light or dark scheme, name in deck history |
| `[stt]`, `[stt.vad]` | Dictation timing, Apple engine on or off, API timeout; voice detection thresholds and pauses |
| `[models]` | Model download host, retries and timeouts |
| `[agent]`, `[agent.presets.<id>]` | Active agent and the agents offered: `name`, then exactly one of `command` (with `args`) or `builtin` |
| `[local_agent]` | Built-in fallback agent: engine, context, temperature, reply length |
| `[updates]` | `check_automatically`, `check_interval_hours` |

Every key, with its default and a comment, is in
[`config.default.toml`](crates/oku-slides-platform/src/config.default.toml).

The app writes to `config.toml` itself when you change the language, the scheme or the
agent, and when the agent changes an app setting for you (applied at next launch); it
sets one key at a time and keeps your comments. The choices of the dictation box (engine,
Whisper model, language, auto stop, API address, model, key variable, query parameters and
headers) are not in `config.toml`: they go to `config.json` in the models folder.
Edit that file with the app closed: the app reads it at launch and rewrites it whole on
the next change in the box. A file that does not parse is kept aside as
`config.json.invalid-<seconds>[-<n>]` (with a warning in the log) and the defaults apply.

### Environment variables

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
| `OKU_SLIDES_UPDATES__CHECK_AUTOMATICALLY` | `false`: no automatic update check, only "Check for Updates…" |
| `OKU_SLIDES_UPDATES__CHECK_INTERVAL_HOURS` | hours between automatic checks (default 6) |
| `OKU_SLIDES_DEBUG_NETWORK` | `1`: `debug` logs of outgoing HTTP calls and TLS (dictation API, downloads, updater) on stderr, release builds included; secret headers and URL paths stay masked. Launch the app from a terminal to see them |

The interface ships in 20 languages (`crates/oku-slides-i18n/src/locales`), English
being the reference.
