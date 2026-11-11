# Developing Oku Slides

How the repository is laid out, the `oku-slides` command-line tool, and how to build and
test the app. For using the app, see [usage.md](usage.md); for settings, [configuration.md](configuration.md).

Design decisions: [`adr/`](adr/README.md). Contributor and agent rules: [`AGENTS.md`](../AGENTS.md).

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
| `crates/oku-slides-stt` | Speech: dictation (microphone, voice detection, Apple's engine on macOS 26+, local Whisper or an API) and OS voice (macOS, Windows, Speech Dispatcher) |
| `crates/oku-slides-a2ui` | A2UI v0.9 surfaces (component catalog), shared by the panel and the agent |
| `crates/oku-slides-mcp` | Agent MCP tools (local HTTP, token): reading, `apply`, surfaces, confirmations |
| `crates/oku-slides-agent` | Voice agent: ACP client (agent as a subprocess), turns, stack for the app and the server; fake agent for tests |
| `crates/oku-slides-local-agent` | Built-in fallback ACP agent (`--local-agent`): routing by tool families, bounded MCP tool loop, final `report`; Apple Foundation Models or Gguf (llama.cpp) engine, no account or network |
| `crates/oku-slides-apple-model` | Swift bridge to Apple's on-device language model (Foundation Models, macOS 26+, weak-linked): availability and one guided reply |
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
just oku-slides usage deck.oku-slides.pdf --by model --since 2026-10-01  # agent tokens and cost recorded in decks
```

`--fonts <dir>` adds fonts (TTF/OTF); without it, Typst uses its built-in fonts
(Libertinus Serif, New Computer Modern, DejaVu Sans Mono).

`usage` sums the agent usage recorded in each deck (`--by day|month|model|agent|tab|deck`, UTC
days, `--since` and `--until` inclusive, `--json` for the raw report); a turn found in several
decks (a copy, a merge) counts once. Costs are the agent's own, never summed across currencies.

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
region, measured with office2pdf 0.10.0 on the alto, libreoffice and kitchen fixtures on
2026-11-01). `[import] witness = "soffice"` (or `--reference soffice`) uses LibreOffice
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
Hi-fi packs belong to the deck: their own tab, linked to the deck, edits them from the
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
just web      # interface in the browser with hot reload (localhost:5174)
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

