# Agent model choice over ACP — Plan

Task: t-1257 (sub-tasks t-1258 to t-1264, one per step left). Goal: pick the model of the ACP agent (Codex, Claude…)
in the "Dictation and agent" box, through ACP session config options (stable in
agent-client-protocol-schema 1.9.1: `configOptions`, `session/set_config_option`, category `model`).

## Decisions

- **Global, per agent preset:** `[agent.presets.<id>] model = "<value id>"` in `config.toml`
  (absent: the agent's default). The box writes it with `write_value("agent.presets.<id>.model")`;
  refuse an id containing `.` (`AgentModelNotSaved`), since `write_value` splits on dots.
- **Offered list from the agent:** the first select option of category `model` (or id `model`
  without category) in the `session/new|load|resume` replies, `set_config_option` replies and
  `config_option_update` notifications. Kept per launch epoch in `ModelBoard` (in `Link`).
- **Applied lazily:** before each prompt, a tab actor compares its session's current model with
  the chosen one; if offered and different, it sends `session/set_config_option`. A refusal is
  logged (warn) and the turn goes on; the box shows the model actually in use.
- **User only:** no MCP tool; `VoiceEvent::SelectAgentModel` comes from the box (`agent_model` action).
  Never restarts the agent; applies from the next message in each tab.

## Done before the handoff

- platform: `AgentPreset.model`, default config comment, config tests.
- session: `AgentModelView`, `AgentModelsView`, `AgentsView.models`, `VoiceEvent::SelectAgentModel`,
  `voice_event("agent_model")`, panel `a-model` Select / `a-model-missing` / `a-model-note`, tests.
- i18n: `voice-agent-model*` (session.ftl) and `agent-error-unknown-model`,
  `agent-error-model-not-saved` (agent.ftl), 20 locales.
- mcp: `VoiceErrorKind::{UnknownAgentModel, AgentModelNotSaved}`.
- agent: `src/session_model.rs` (`model_options`, `Offered`, `ModelBoard`).

## Steps after the handoff (all done)

1. agent `lib.rs`: `mod session_model;`, write `session_model_tests.rs` (parse ungrouped, grouped,
   category-less `model`, non-model options; board epoch and revision).
2. `link.rs`: `Inner.models: Arc<ModelBoard>`, the wanted model kept with the launch and its epoch
   under one lock (`set_wanted_model`, `wanted_model`, `offered_models` for the current epoch,
   `models_revision`); `Link::switch(launch, model)` and `Link::with_model` set it with the agent, so
   no prompt between a switch and a separate write runs the new agent without its model;
   `forget` and connection end also clear the board; `on_notification` handles
   `SessionUpdate::ConfigOptionUpdate` before the turn routing (outside turns too).
3. `tab_agent.rs`: `open`/`reopen` pass `config_options` to `ModelBoard::told`; `ensure_model`
   before `turn::run` (`SetSessionConfigOptionRequest`, reply told to the board).
4. `choice.rs`: chosen models per preset seeded from settings; `switch_to` sets the link's wanted
   model; `agents_view` fills `models` (`None` for the builtin agent); `check_model`, `choose_model`.
5. `voice_port.rs`: `check`/`apply` for `SelectAgentModel` (`unchanged` when already chosen);
   `AppVoice::new` forwards `models_revision` to `bump` so every tab's box refreshes.
6. `bin/fake_agent.rs`: scenario `models: [{id, name}]` in session replies, `session/set_config_option`
   logged and answered (error on an unknown value); stack test: choose a model in the box, next
   prompt sends `set_config_option`, a resumed session gets it too.
7. Docs: agent `ARCHITECTURE.md` (`session_model`), agent `AGENTS.md` rule (apply the chosen model
   before a prompt, only if offered; never restart the agent for it), help `docs/aide` if needed,
   then `just check` and `just test`.
