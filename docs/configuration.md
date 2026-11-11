# Configuring Oku Slides

Settings come in three layers, the last one wins:

1. the defaults, embedded and commented
   ([`config.default.toml`](../crates/oku-slides-platform/src/config.default.toml));
2. `config.toml` in the app data folder, optional;
3. environment variables `OKU_SLIDES_<SECTION>__<KEY>`, with a double underscore to
   go down one level.

## The configuration file

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
| `[usage]` | `store_in_deck`: record agent usage in each deck (default `true`); `false` also drops it at the next save |
| `[local_agent]` | Built-in fallback agent: engine, context, temperature, reply length |
| `[updates]` | `check_automatically`, `check_interval_hours` |

Every key, with its default and a comment, is in
[`config.default.toml`](../crates/oku-slides-platform/src/config.default.toml).

The app writes to `config.toml` itself when you change the language, the scheme or the
agent, and when the agent changes an app setting for you (applied at next launch); it
sets one key at a time and keeps your comments. The choices of the dictation box (engine,
Whisper model, language, auto stop, API address, model, key variable, Azure API version and
headers) are not in `config.toml`: they go to `config.json` in the models folder.
Edit that file with the app closed: the app reads it at launch and rewrites it whole on
the next change in the box. A file that does not parse is kept aside as
`config.json.invalid-<seconds>[-<n>]` (with a warning in the log) and the defaults apply.

## Environment variables

| Variable | Effect |
| --- | --- |
| `OKU_SLIDES_PATHS__DATA` | App data folder |
| `OKU_SLIDES_PATHS__DECKS` | Where new decks are saved automatically (default `Documents/Oku Slides`) |
| `OKU_SLIDES_PATHS__MODELS` | Dictation models folder |
| `OKU_SLIDES_PATHS__FONTS` | Extra fonts (TTF/OTF) |
| `OKU_SLIDES_STT__APPLE` | `false`: Whisper only |
| `OKU_SLIDES_STT__FAKE_MIC` | WAV file replayed instead of the microphone; refused at load without the `e2e-fakes` feature |
| `OKU_SLIDES_STT__VAD__PAUSE` | Pause that ends a sentence (s), like any `[stt.vad]` key |
| `OKU_SLIDES_SERVE__PORT` | Dev server port |
| `OKU_SLIDES_I18N__LOCALE` | Interface language (`en`, `fr`, `de`, `ja`…); default: the system's |
| `OKU_SLIDES_APPEARANCE__SCHEME` | `light` or `dark`; default: the system's |
| `OKU_SLIDES_AUTHOR__NAME` | Name recorded in the history of decks you edit |
| `OKU_SLIDES_USAGE__STORE_IN_DECK` | `false`: no agent usage recorded in decks, and none kept at the next save |
| `OKU_SLIDES_UPDATES__CHECK_AUTOMATICALLY` | `false`: no automatic update check, only "Check for Updates…" |
| `OKU_SLIDES_UPDATES__CHECK_INTERVAL_HOURS` | hours between automatic checks (default 6) |
| `OKU_SLIDES_DEBUG_NETWORK` | `1`: `debug` logs of outgoing HTTP calls and TLS (dictation API, downloads, updater) on stderr, release builds included; secret headers and URL paths stay masked. Launch the app from a terminal to see them |
| `OKU_SLIDES_DEBUG_ACP` | `1`: every ACP message exchanged with the agent, both ways (`→` sent, `←` received), as `debug` logs on stderr, release builds included; the MCP tools' `Authorization` header stays masked, but prompts, pasted images and deck content are logged as is. Launch the app from a terminal to see them |

The interface ships in 20 languages (`crates/oku-slides-i18n/src/locales`), English
being the reference.
