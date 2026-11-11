# Using Oku Slides

The workspace, hand editing, dictation and the agent. Settings are in
[configuration.md](configuration.md).

## The workspace

- **Pile** (left): slide thumbnails. Click to show, drag to reorder (or ⌥↑ / ⌥↓),
  `+` adds a slide from a layout.
- **Sheet** (middle): the slide exactly as it prints; elements are selected, moved
  and edited right on it.
- **Thread** (right): the conversation with the agent and the dictation draft. The
  menu only presents, opens a file and shows help: everything else, app settings
  included, is asked of the agent, so the app runs without keyboard or mouse.

## Editing by hand

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
| ⌘⇧E | Show or hide the inspector |
| ⌘K | Type an instruction for the agent |
| ⌘1…⌘9 | Go to tab n (⌘9: the last one) |
| ⌘? | App help (read-only deck in French or English, otherwise English; `docs/aide/`) |

The bottom bar adds a text, a card, a shape, a chart, a table or a widget from the
deck's widget packs.

## Charts, tables, widgets

- **Charts**: a subset of ECharts options rendered to SVG (`oku-slides-chart`); the
  data is edited in a grid in the data pane under the stage (the inspector's Data button).
- **Tables**: the same data grid; header row, first column as header, striped rows
  and per-column alignment (automatic puts numbers at the end). A chart converts to
  a table and back.
- **Widgets**: packs ship Typst widgets: `base` (text, card, image, shape, chart, table,
  stack) and layout packs such as `essentiel`, whose widgets are whole slides. In pack
  mode, the Widgets section of the pile opens their code next to a live preview. Rendering runs in a
  separate worker process, bounded in time and memory (`[render]` settings); past
  the limit, the faulty pack is disabled.

## Grid, packs, themes

- **Grid**: every slide has a grid (12 × 6 by default), set in its layout (pack mode,
  the slide's settings); slides created from the layout inherit it.
  Snapping is always on: an element covers a block of cells, moves cell by cell
  (mouse or arrows), and each corner snaps to the nodes. The nodes show only
  during a drag, with the targeted cell. The geometry is computed
  identically by `ui/src/grid.ts` (editing) and `grid-frame` in `typst/oku-slides/src/zone.typ`
  (rendering).
- **Layout packs**: “New slide” lists the layouts of the layout packs the deck pins
  (built-in `typst/oku-slides/packs/essentiel/layouts.json`: title, contents, chapter,
  content, end, blank; user packs in the `layouts/` folder of the app data). A new slide
  starts from the layout's example values. A pack does not depend on the theme: a deck
  picks its packs and its theme independently.
- **Editing a pack** (ask the agent): the pack opens in its own tab, beside a preview of the deck, as a
  working deck, one slide per layout, seeded from its example; each edit on a slide is
  written to the layout's example at once (one undo step), and saving only saves the
  pack's name. Slides cannot be added, removed or moved there. A built-in pack is saved as a copy; a deck-only pack's tab is linked to its deck; “Done” closes the tab.
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

## Files, tabs, history

- **Automatic saving**: every change goes to the file; a new deck goes to
  `Documents/Oku Slides` (`[paths] decks`).
- **Tabs and windows**: one deck per tab, several windows; everything comes back at
  next launch.
- **History** (ask the agent): a slider walks every version stored in the file;
  Restore makes one current again.
- **Merge** (ask the agent to merge the other PDF): two copies of the same deck, edited
  separately, merge without conflicts. Deck sources are a Loro CRDT (see
  [ADR 0002](adr/0002-sources-du-deck-en-crdt-loro.md) and
  [ADR 0005](adr/0005-elements-d-une-slide-en-arbre-loro.md)).

## Dictation

Everything happens on the machine, unless you choose the API engine
([OpenAI or Azure OpenAI](#dictation-through-azure-openai-in-a-company)). On macOS 26+, Apple's speech engine is the default when it knows the language
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
`OKU_SLIDES_PATHS__MODELS` overrides it (see [Settings](configuration.md)).

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
OKU_SLIDES_STT__FAKE_MIC=voice.wav cargo run -p oku-slides-cli --features e2e-fakes -- --fonts assets/fonts serve fixtures/roadmap.json  # simulate the mic (e2e-fakes builds only)
just test-apple                                   # Apple's engine for real (macOS 26, French installed)
```

### Dictation through Azure OpenAI in a company

The **API** engine sends each sentence to OpenAI or to an Azure OpenAI transcription
deployment instead of transcribing on the machine. A typical company setup: a
`gpt-4o-transcribe` (or `whisper`) deployment on an Azure resource, reached through a
gateway that wants its own headers, behind a proxy that inspects TLS.

1. **Put the secrets in environment variables.** Oku Slides never stores a key: it stores
   the *name* of a variable (capitals, digits and `_`, such as `AZURE_OPENAI_API_KEY`) and
   reads it when dictation starts. An app opened from the Finder does not see your shell's
   variables, so set them for the session, then restart the app:

   ```sh
   launchctl setenv AZURE_OPENAI_API_KEY "…"
   launchctl setenv GATEWAY_TOKEN "…"       # only if your gateway needs one
   ```

   Or start the app from a terminal where they are exported. On Linux and Windows, set
   them in the environment the app is launched from.
2. **Fill the dictation settings** (⚙ of the dictation bar, engine **API**):

   | Field | Value |
   | --- | --- |
   | API address | The resource address, `https://<resource>.openai.azure.com`, or your gateway's address in front of it (no `/openai/deployments/…` path) |
   | Model | The **deployment name** (letters, digits, `.`, `_`, `-`), not the model name |
   | Key variable | `AZURE_OPENAI_API_KEY`; its value goes in the `api-key` header |
   | Azure API version | The `api-version` your deployment supports, such as `2025-03-01-preview` |
   | Extra headers | `Name: value; Name: $VARIABLE`, such as `X-Tenant: acme; Ocp-Apim-Subscription-Key: $GATEWAY_TOKEN` |

   Each request goes to
   `<address>/openai/deployments/<deployment>/audio/transcriptions?api-version=<version>`.
   With an empty version, the OpenAI API is called instead (`<address>/audio/transcriptions`,
   key as `Authorization: Bearer`): use it for OpenAI itself or for an OpenAI-compatible
   endpoint, such as Azure's `https://<resource>.openai.azure.com/openai/v1`.
3. **Headers.** A `$VARIABLE` value is read from the environment at each start, like the
   key, and never stored; a literal is stored in clear in `config.json`, so put tokens in
   variables. At most 8 headers, values in printable ASCII without `;`. `Authorization`,
   `api-key`, `Host`, `Content-Type`, `Proxy-Authorization` and the other framing headers
   are reserved: the key always goes through the key variable. Only you can change the
   address, key variable, version and headers; the agent may only switch to the API once
   you have set them, after asking.
4. **Proxy and certificates.** Requests use `HTTPS_PROXY` / `NO_PROXY` (and, in the app,
   the system proxy settings), except to this computer. Certificates are checked against the
   system's trust store, so a company root CA installed in the Keychain (or the Linux and
   Windows stores) is trusted, TLS-inspecting proxies included. Redirects are not followed:
   give the final address.
5. **Check it.** The settings box shows **Key found** or the first variable that is not set.
   A refused request shows its HTTP status, with the service's message except for 401 and
   403, and every secret masked. Each sentence gets one attempt (no retry) within
   `[stt] api_timeout_secs` (30 s). To see the calls, start the app from a terminal with
   `OKU_SLIDES_DEBUG_NETWORK=1`: secret headers stay masked.

The same settings in `config.json` (models folder, app closed):

```json
{
  "version": 1,
  "model": "api",
  "api": {
    "url": "https://acme-ai.openai.azure.com",
    "model": "transcribe-prod",
    "key_env": "AZURE_OPENAI_API_KEY",
    "api_version": "2025-03-01-preview",
    "headers": [
      { "name": "X-Tenant", "value": { "literal": "acme" } },
      { "name": "Ocp-Apim-Subscription-Key", "value": { "env": "GATEWAY_TOKEN" } }
    ]
  }
}
```

Settings saved by version 7.0.0 or earlier with query parameters (`"query": […]`, the old way to pass
`api-version`) are not migrated: dictation reports invalid settings until you enter the
API fields again in the box.

## Agent

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
  agent requests: a click or, hands-free, a spoken answer (“yes” plus the action's verb for a
  risky one, only while the app window has the focus); never Enter, and never “always allow”.
- **Decisions**: the agent asks its questions in a card under the element (choices,
  rewordings: `1`–`4`, Enter, `E` to edit, Esc), proposes variants rendered in a
  drawer under the sheet, and live settings in a sticky note. A question can be
  answered by voice: through the draft, or hands-free by saying the answer.
- **Conversation**: one per tab, restored with the tab at next launch; tabs work at the
  same time. “New conversation” starts from scratch. ⌘⇧D during a turn stops it.
- **Detached window**: ⧉ at the top of the thread (or Window ▸ Agent in a Separate Window, on macOS) opens
  the agent in its own window, for a second screen. It follows the active tab of the
  front window.
- **Usage**: each turn's tokens, model, agent and cost (when the agent reports them) are
  recorded in the deck file itself, never marking it modified; ask the agent what the deck
  cost, or run `oku-slides usage`. `[usage] store_in_deck = false` records nothing and saves
  decks without it.
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

