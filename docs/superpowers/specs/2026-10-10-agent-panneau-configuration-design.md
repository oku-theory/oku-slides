# Agent configuration panel

Task t-775. Today the app launches **a single ACP agent**, whose command
and arguments are fixed in `[agent]` of `config.default.toml`
(`npx -y @agentclientprotocol/claude-agent-acp@0.81.2`). They can be
overridden by `config.toml` or `OKU_SLIDES_AGENT__*`, but no interface
lets one change agents. This spec adds a **list of agents defined
by the configuration** and a **panel in the agent area** to choose
the active agent.

Builds on `2026-10-09-agent-detachable-design.md` (the detached window
also renders `AgentPanel`) and on the lifecycle of `oku-slides-agent`
(`ARCHITECTURE.md`: one agent for the app, stop → `agent-stopped`, relaunch on the
next prompt).

## Decisions (chosen on 2026-10-10)

1. **Choose an agent from a list; a single active one for the whole app.** No
   agent per tab or per perspective, no setting of the agent's model or
   modes (out of scope).
2. **The list comes from the configuration** (`config.default.toml`, overridable
   by `config.toml`). The panel **only chooses**: it neither adds nor
   edits agents. Launching an arbitrary subprocess stays a decision
   made in an explicit file.
3. **The panel lives in the agent area**: a button in the header
   of `AgentPanel` opens a card. No "Settings…" window (its own
   spec, later).
4. **Immediate switch.** The current agent is stopped; an in-progress turn
   fails as for any stop (`agent-stopped`); the next prompt of
   each tab launches the new agent. The card warns, before the click,
   that a turn will be interrupted.
5. **Shipped agents: Claude, Codex, Gemini**, each pinned to a version:
   - `@agentclientprotocol/claude-agent-acp@0.81.2` (unchanged; the version
     bump is another work item);
   - `@agentclientprotocol/codex-acp@2.2.2`: `@zed-industries/codex-acp` is
     deprecated ("replaced by @agentclientprotocol/codex-acp", seen on npm on
     2026-10-10);
   - `@google/gemini-cli@0.63.0 --acp`: `--experimental-acp` is deprecated in
     favor of `--acp`.
   Each agent is checked (handshake, `agent_capabilities.mcp.http`)
   during implementation; an agent that fails is removed from the list.
   Checked on 2026-10-10: with an ineligible Google account (free tier,
   `IneligibleTierError`, `UNSUPPORTED_CLIENT`), Gemini CLI never answers
   `initialize`; the first prompt then fails after the startup timeout, and
   changing agents interrupts that launch.
6. **The agent does not choose its successor**: `set_setting` refuses `agent.*`.

## Configuration

`config.default.toml`, the `[agent]` block becomes:

```toml
[agent]
# Active agent: a key of [agent.presets]; chosen in the agent panel.
selected = "claude"

[agent.presets.claude]
name = "Claude"
command = "npx"
args = ["-y", "@agentclientprotocol/claude-agent-acp@0.81.2"]

[agent.presets.codex]
name = "Codex"
command = "npx"
args = ["-y", "@agentclientprotocol/codex-acp@2.2.2"]

[agent.presets.gemini]
name = "Gemini"
command = "npx"
args = ["-y", "@google/gemini-cli@0.63.0", "--acp"]
```

A **named table** (`[agent.presets.<id>]`) rather than an array: the `config`
crate merges by key, so a `config.toml` that declares
`[agent.presets.mistral]` adds an agent without erasing the other three, and
can replace one by reusing its key.

### Types (`oku-slides-platform`)

- `AgentSettings { selected: String, presets: BTreeMap<String, AgentPreset> }`
  (stable display order: that of the keys).
- `AgentPreset { name: String, command: String, args: Vec<String> }`. The name
  is a proper name coming from the configuration, not a Fluent string.
- `AgentPreset::new(name, command, args)`: the struct is
  `#[non_exhaustive]` like the other settings, the tests of other crates
  go through this constructor.
- `AgentSettings::active() -> &AgentPreset`. Validation happens **at
  load time** (`Settings::from_sources`): a `selected` that names no
  preset is a `ConfigError` (English message `agent.selected = "<id>" names
  no [agent.presets] entry (known: …)`), which surfaces like any settings
  error (`PlatformError::Settings`): the `Localizer` does not exist yet at
  that point. No silent fallback to the first agent.
- `AgentSettings::locked: bool` (`#[serde(skip)]`): true if
  `OKU_SLIDES_AGENT__SELECTED` is defined, set by `Settings::load`.

### Removals (no shim, ADR-015)

`agent.command`, `agent.args` and the splitting of `OKU_SLIDES_AGENT__ARGS` on
spaces (`list_separator(" ")` / `with_list_parse_key("agent.args")` in
`config.rs`) disappear. All consumers are updated:

- `Stack::start` (launch from `AgentSettings::active()`);
- `crates/oku-slides-agent/tests/common` and `examples/smoke.rs`;
- `crates/oku-slides-platform/src/config_tests.rs` (test of the
  `AGENT__COMMAND`/`ARGS` variables, replaced by those of `selected`);
- the `e2e` recipe of the `justfile`: instead of
  `OKU_SLIDES_AGENT__COMMAND`/`ARGS`, it writes into its isolated data
  folder (`$tmp/data/config.toml`) two presets of the fake agent, `e2e` and
  `e2e-bis` (scenarios `fixtures/agent/e2e.json` and a second scenario
  `e2e-bis.json`), with `selected = "e2e"`; the comment of
  `ui/e2e/agent.cjs` follows;
- the settings table of `README.md` (`[agent] selected` and
  `[agent.presets.<id>]`).

`OKU_SLIDES_AGENT__SELECTED` works naturally and **wins** over
`config.toml`. In that case, the choice made in the panel would be written but
ignored at the next launch: the panel shows as **locked** (see
below). The lock is detected by the presence of the variable at startup
(`AgentSettings::locked`).

The `agent-not-found` message ("[agent] command setting") is updated in
the 20 languages: it points to `[agent.presets]`.

## Switch

### `Link::switch(launch: Launch)` (new, public, synchronous)

`Link` is cloned in each tab; dropping the last clone stops the agent.
So the `Link` of `Stack` is not replaced: the switch happens **inside** the
shared `Link`.

Synchronous: `Stack::handle` is too, and a prompt following the choice must
see the new agent without racing with a task launched separately.

1. Under a standard lock (`Inner::launch: StdMutex<(Launch, époque)>`),
   replaces the launch and increments the **epoch** (`u64`). `agent_id()` and
   `fallback_cwd()` (returned as `PathBuf`) read the current launch:
   the fingerprint changes, the watchdog does not attempt to resume a session of
   the other agent (new session, without a "lost memory" card).
2. Stops the live or launching connection: each `Live` carries its epoch and a `stop`
   token, a **child** of the `shutdown` token; `Inner::current_stop` keeps that
   of the live or launching connection (set before the
   handshake). Cancelling it kills the subprocess of that
   generation only; `shutdown` cancels them all and stays final.
3. `connect` reuses a connection only if it is not closed **and** from
   the current epoch; a connection from another epoch is stopped (`stop`)
   then replaced. An in-progress launch is interrupted by the switch, and
   the next attempt launches the new agent.
   The routes of the old generation no longer receive anything.

In-progress turns go through the **existing stop path**: connection
closed → `agent-stopped` in the thread, undo groups closed,
questions released. Nothing new on the tabs side.

### `Stack::select_agent(id: &str) -> Result<(), String>` (new)

1. `id` missing from the presets (stale surface): localized error, nothing
   changes. Environment lock: localized refusal.
2. Already the active agent: nothing.
3. Writes `agent.selected` into `config.toml` through `write_value`
   (`settings_port.rs`). **Write refused: no switch**, the displayed choice
   and the saved choice never diverge.
4. `link.switch(Launch { … })` with the chosen preset, `fallback_cwd` and
   `READY_WITHIN` unchanged.
5. Pushes the updated panel surface to the views of the tab that chose.

`Stack` keeps the active id in a `Mutex<String>`, initialized from
`settings.agent.selected`: `Platform.settings` is frozen at load time.

### `set_setting` refuses `agent.*`

`AppSettings::write` refuses any `agent` or `agent.*` key with the message
`agent settings are reserved to the user`, returned to the agent like the other
`write` refusals (`dictation unavailable`…). `read` does not expose them.

## Surface and interface

Tab surfaces go through the tab's `SurfaceHub`; the agent choice
applies to the app, but the UI only reads `{tab, event}` envelopes
(`ui/src/agent/events.ts`). The panel is therefore **delivered to the tab that
asks for it**, on the `"agent"` channel, like the draft (`AgentEvent::Draft`,
emitted directly by `emit`, ignored by the turns reducer). The views of
that tab (main window and detached window) receive it; nothing
changes on the hosts or subscription side.

### Construction (`oku-slides-agent`, new module `picker`)

`picker_surface(presets, active, busy, locked, l) -> Surface` is pure.
A2UI surface with id `agent-picker`:

- localized title ("Agent");
- `ChoiceChips`: one chip per preset (label: the name; `hint`: the
  command and its arguments), value `/choice` = the active one; `select` action
  with `{id: /choice}`;
- `busy` (an open turn, `hub.turn()`, in a tab): warning
  line « Changer d'agent interrompra le tour en cours » (Changing agents will interrupt the turn in progress);
- `locked`: chips **without action** and line « Choix imposé par
  l'environnement (OKU_SLIDES_AGENT__SELECTED) » (Choice set by the environment).

New event `AgentEvent::Picker { surface }` (`AgentEvent` loses `Eq`,
`Surface` is only `PartialEq`). Two new commands:

- `AgentCommand::Picker { tab }`: builds the surface (`busy` evaluated at that
  instant) and emits it to `tab`. Sent by the UI when the card opens:
  the state shown is always the one at the moment it is opened.
- `AgentCommand::SelectAgent { tab, id }`: `Stack::select_agent(id)`, then
  emits the updated surface to `tab`. Distinct from `SurfaceAction`, reserved for
  tab surfaces.

Like any command, they are ignored for a closed tab; they do not
start the tab.

### Display (UI)

- `AgentThread` receives `onPicker`: a button (icon `settings`, label
  `ui-agent-picker`) in `agent-thread-head`, always present when
  `onPicker` is given (the header is then never `vacant`).
- `AgentPanel` holds `picker: Surface | null` (last `AgentEvent::Picker`)
  and `pickerOpen`. The button opens the card and sends `picker`; a second
  click, Escape, or an accepted choice closes it.
- New component `ui/src/agent/AgentPicker.tsx`: renders the surface through
  `SurfaceView` (lazily loaded, like `AgentThread`), under the header.
  The `select` action goes out through `agent.selectAgent(id)`; a refusal is shown through
  `say(…, true)` (banner).
- The detached window renders `AgentPanel`: same button, same surface.
  `oku-slides serve` too (browser), through the same SSE relay.
- No confirmation modal: the warning is visible before the click.

### i18n

Title, warning, lock, button label and errors shown to the
user (unknown `id`, write refused, lock) go through Fluent in
all existing languages. Agent names come from the
configuration.

## Errors

| Case | Behavior |
|---|---|
| Unknown `selected` at load time | `ConfigError` (key, known agents): the app does not start, as for any invalid configuration |
| `SelectAgent` with an unknown `id` | localized error in the UI (banner), no switch |
| Write to `config.toml` refused | localized error, no switch, surface unchanged |
| Choice set by the environment | list disabled; a command received anyway is refused |
| New agent not found or without HTTP MCP | existing error in the thread on the first prompt (`agent-not-found`, `agent-no-http`, `ready_within` timeout); one can go back to the previous agent |
| `set_setting("agent.…")` by the agent | refused (`agent settings are reserved to the user`) |

## Tests

Test-first at each step.

- **platform**: the three default presets loaded; a `config.toml` that
  adds a preset keeps the others; unknown `selected` → error;
  `OKU_SLIDES_AGENT__SELECTED` wins over `config.toml`.
- **link**: `switch` stops the live generation and relaunching stays
  possible; the next `connect` launches the new command; `agent_id`
  changes; `shutdown` after `switch` stays final; a switch during a
  launch relaunches with the new agent.
- **stack** (fake agent, two presets with different scenarios): switch outside a
  turn, the next prompt talks to the second agent; switch during a turn, the
  turn fails with `agent-stopped` then the next prompt talks to the second one;
  write refused → no switch; no resume and no "lost
  memory" card from one agent to the other; environment lock → refusal.
- **picker**: `insta` snapshots of the surface (normal, `busy`, `locked`).
- **settings_port**: refusal of `agent.*`.
- **UI**: `AgentThread`/`AgentPanel` render tests: the button sends
  `picker`, the card shows the received surface, a choice sends `select_agent`
  then closes, Escape closes.
- **e2e**: open the panel, choose the second fake agent, check its
  answer; free port and isolated data folder.
- **Preset verification** (manual): each of the three pinned agents
  is launched, its handshake done and `mcp.http` observed; an agent that
  fails is removed from `config.default.toml` before the merge.

## Out of scope

Agent per tab or per perspective; adding or editing agents from the
panel; agent-specific settings (model, modes, instructions); general
"Settings…" window; version bump of `claude-agent-acp`.
