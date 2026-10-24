# Dictation and agent settings as an A2UI host box, driven over MCP — Design

Task: t-1144. Plan: `docs/superpowers/plans/2026-10-23-reglages-dictee-a2ui.md`.

## Goal

The ⚙ popup of the dictation device (`Dictation.tsx`: model, language, auto stop, API
fields, download, then the agent picker) becomes a live A2UI box built by the host and shown
in the agent area, like the deck packs panel. The agent can open that box and change its
settings over MCP, the agent choice included — except the dictation API address and key
variable, which only the user sets.

## Decisions

- **One box, `host-voice`:** dictation settings, then the agent section (today's
  `picker_surface` content), then a close button. Shown live outside any turn, in the dock
  like `host-packs` (a requested box: never folded under a question); it stays until closed
  and follows every change. The ⚙ button opens it; the popup and its markup are deleted.
  Read-only views have no dock, so their ⚙ is hidden.
- **App-wide content, per-tab display:** dictation (`Stt`) and the agent choice
  (`AgentChoice`) are global, so the box content is the same in every tab; each tab's
  `SurfaceHub` shows it on demand, and a change made anywhere refreshes every hub showing it.
- **Existing catalog only:** no new `ComponentKind`, `CATALOG_ID` unchanged. Models are
  `ChoiceChips` (label: short name; hint: note, size, state — no `Badge`), unavailable models
  a `Text` line, language a `Select`, auto stop one `Select` (never, 1, 1.5, 2, 3, 5 s, plus
  the current value), API fields three `TextField`s, download/cancel `Button`s. Progress, busy
  and pending lines are always present `Text`s bound to the data model, so their changes are
  data-only patches and never rebuild the box (no lost typing).
- **The agent never chooses where dictation goes nor which secret it sends.** `api.url` and
  `api.key_env` are not tool arguments; a request touching them is refused before any
  dialog (`ApiReservedToUser`, telling the agent to open the box). No format rule on these
  fields could close the hole: any existing secret, transcription keys included, could go to
  a host the agent picks.
- **Validation on every write and at use:** the address is ASCII, ≤ 512 chars, without
  whitespace, controls, userinfo, query or fragment, host ≤ 253 chars and labels ≤ 63,
  `https` or `http` to this computer only; only its normalized form is stored, shown and
  called. The key variable matches `^[A-Z_][A-Z0-9_]{0,63}$`, the API model
  `^[A-Za-z0-9._:/-]{1,128}$`. The API client refuses invalid stored settings, follows no
  redirect and uses no proxy for this computer. Key presence is checked without copying the
  key. An old invalid stored address blocks the agent's switch to `api`, never the user's
  (who must reach the fields to correct it).
- **Confirmed means confirmed:** an agent request answered yes is applied with
  `confirmed: true`; if the state changed so that a confirmation would now be needed but was
  not given, it is refused (`Changed`).
- **User clicks apply at once, unconfirmed**, as today (an agent switch by click interrupts
  running replies, as the picker does now).
- **Agent tools (MCP):**

  | Tool | Effect | Confirmed |
  |---|---|---|
  | `show_dictation` | opens the box, returns `{shown: "host-voice"}` | no |
  | `set_dictation {model?, language?, auto_stop?, silence?, api_model?}` | partial update | only switching to `api` (danger: user-set host, URL, model, variable) or to a model to download (then downloaded) |
  | `download_dictation_model {model}` | model or Apple language | yes (name, size) |
  | `cancel_dictation_download` | stops it; refused when none runs | no |
  | `select_agent {id}` | switches the app agent once no reply runs | yes (name, command) |
  | `download_agent_model` / `cancel_agent_model_download` | local agent model | yes / no |

  Every request is checked through the port before any dialog; a refusal opens none. A
  request that changes nothing answers `{"applied": "unchanged"}`.
- **One path per setting:** `set_setting` drops its `dictation.*` keys (the `settings` read
  tool still lists them) and keeps refusing agent settings (t-786); agent switching goes
  through `select_agent`.
- **Agent switch when no reply runs:** one ACP agent serves every tab, and switching kills
  it. A `PromptGuard` held by each tab actor while it prompts (and across a queued follow-up)
  counts running prompts; "busy" means any tab. An agent's `select_agent` applies at once
  when nothing runs, else it is pending (`when_idle`), shown in the box, and applied when the
  last guard drops, re-checked under the `AppVoice` lock with an epoch so a user click wins and
  a starting prompt is never cut; then the agent actually active is persisted, never the
  requested one. Choosing the active agent never restarts it, it only clears a pending choice.
  Closing the requesting tab or quitting drops it. Under steady use a pending switch can wait
  long: the box shows it, which is accepted.
- **Trust boundary:** on loopback, `oku-slides serve` accepts UI commands without a token, so
  a local process — an agent allowed to run shell commands included — can act as the user;
  such an agent could read the environment anyway. The guarantees above hold against agents
  that only reach the app through MCP.
- **The API key never leaves the environment:** not in the surface, tool results, logs or
  confirmation text; only the variable name and "found / not set".

## Components

- `oku-slides-stt`: `api_check` (`normalized_url`, `url_host`, `is_loopback`,
  `valid_key_env`, `valid_api_model`), `Model::{id, from_id}`, `SILENCE_RANGE`,
  `configure -> bool` (false when dictation is busy), `download -> bool` (false when nothing
  started), `ApiConfig::key_present`, a hardened `ApiTranscriber`. `SttRequest::{Configure, Download}`
  are removed: the box and the port are the only writers.
- `oku-slides-session`: `voice_view.rs` (text-free `VoiceView`, `DictationPatch`,
  `VoiceEvent`, `voice_event`) and `voice_panel.rs` (`VoicePanel { build, surface,
  same_layout, data_messages }`, `model_label`, `size_text`, `seconds_text` with
  `NumberFormat`). Living here puts the box in the forms contract test.
- `oku-slides-mcp`:
  - `voice.rs`: `VoicePort { view, check, apply }`, `Origin`, `Checked` (what a confirmation
    needs: API target, model to download, agent, sizes, unchanged), `VoiceApplied`, opaque
    `VoiceError` with a `VoiceErrorKind` per refusal, Fluent-localized.
  - `hub/settings.rs`: `show_settings`, `apply_settings` (data patch when the layout is the
    same, the whole box after a refusal), `refresh_settings_with(&VoicePanel)`; never `prune`.
  - `voice_tools.rs`: the seven tools; confirmations through `SurfaceHub::confirm`.
  - `settings.rs`: `WRITABLE` loses the `dictation.*` keys.
- `oku-slides-agent`:
  - `voice_port.rs`: `AppVoice` (`Stt`, `Arc<AgentChoice>`): `VoicePort`, `tab_closed`,
    `shutdown`. `agent_switch.rs`: `PromptGuard` (unique ids) and the deferred switch.
    `stt_watch.rs`: one `Stt` watcher thread (400 ms, typed fingerprint, bumps only on
    change). `tab_agent.rs` takes the guard in the actor (`spawn_counted`).
  - `voice_refresh.rs`: follows `AppVoice::revision`, builds one panel per change for all
    tabs, emits `AgentEvent::Voice` when the dictation part changed and `Availability` when
    it changed. `table.rs` holds the tab table.
  - `stack.rs`: `AgentCommand::ShowSettings { tab }`; the old `Picker`, `SelectAgent`,
    `DownloadModel`, `CancelDownload` and `AgentEvent::Picker` go.
- `oku-slides-local-agent`: `families.rs` files the new tools under `Configure`.
- `oku-slides-i18n`: box labels move from `ui-dictation-*` and `agent-picker-*` to
  `voice-*` keys, 20 locales; keys no longer used are deleted.
- UI: `Dictation.tsx` keeps mic, level, transcript, idle tooltip and status polling; ⚙ calls
  `agent.showSettings` (hidden in read-only views, replaced by cancel while recording) and the
  status is re-read on `AgentEvent::Voice` through a module store (`useVoiceSeq`), not App
  state. `surfaces.ts` gains `HOST_VOICE` in `isRequestedBox`; `useRelocalizedBoxes`
  re-asks the box on a locale change; the quick-key guard targets the box.

## Data flow

⚙ or `show_dictation` → `hub.show_settings` → `VoicePort::view` → `VoicePanel` →
`McpEvent::Surface` → dock. Click → `SurfaceAction` → `hub.act` → `apply_settings` →
`VoicePort::apply` (`Origin::User`) → `Stt` / `AgentChoice` → revision bump →
`voice_refresh` → every hub's `refresh_settings_with` (patch or surface) and
`AgentEvent::Voice`. Agent call → `voice_tools` → `VoicePort::check` → (confirm) →
`VoicePort::apply` (`Origin::Agent`).

## Testing

- `api_check_tests.rs`: accepted and refused URL forms (normalization, userinfo, bidi,
  lookalikes, whitespace, length), variable and model names. `api_tests.rs`: invalid stored
  settings refused, no redirect followed.
- `voice_panel_tests.rs` / `voice_view_tests.rs`: one test per box state and per action;
  data-only changes keep the layout; every action parses back. The forms contract covers
  three boxes.
- `oku-slides-mcp/tests/voice.rs` (real server, `FakeVoice`): box shown, clicked, refused,
  dismissed, refreshed (patch vs surface); each tool's question, danger flag and refusal
  before any dialog; regressions `issue_1144_*` (`set_setting` without `dictation.*`, no API
  address or variable argument).
- `voice_port_tests.rs`: validation, API reserved to the user, stored target checked,
  `PromptGuard` busy and pending rules, epoch, close and shutdown. Stack tests: a click in one
  tab refreshes another; an agent's `select_agent` during a turn applies after it ends; a turn
  in one tab shows busy in another.
- UI render tests and the e2e spec `210-settings-box` (box opened, agent switched back, language
  round trip, local model, API variable typed by the user with a dedicated test secret that never
  reaches the page or the responses, box closed).
