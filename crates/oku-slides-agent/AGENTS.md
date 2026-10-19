# crates/oku-slides-agent — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Check a prompt for an answer first, then for a shortcut word, before any agent launch; a shortcut never interrupts a turn. (spec:2026-10-19-raccourci-theme-design, t-1071)
- Refuse agent and agent.* keys in agent settings writes; only the user chooses the agent. (spec:2026-10-10-agent-panneau-configuration-design)
- Never quote user data in localized agent command errors; keep the details in Display for logs. (plan:2026-10-10-agent-panneau-configuration)
- Route a resumed session only after its load reply; ignore its replay updates until its first prompt. (spec:2026-10-08-sessions-par-perspective-design)
- Never advertise fs or terminal capabilities to the ACP agent; it acts only through MCP tools. (spec:2026-09-26-agent-vocal-design)
- Reject set_draft when the draft exceeds 256 KiB of UTF-8 or its origin exceeds 64 bytes. (spec:2026-10-09-agent-detachable-design)
- Never let follow or set_draft start a tab agent; without one, refuse set_draft with a localized error. (spec:2026-10-09-agent-detachable-design)
- Open the turn as soon as the prompt arrives, before any step that can fail, so errors show in it. (plan:2026-10-03-agent-a2ui)
- Route all conversation writes through the per-tab keeper queue; the actor only holds live ACP state. (spec:2026-10-08-sessions-par-perspective-design)
- When the agent process dies mid-turn, show a localized error and still close the turn undo group. (spec:2026-09-26-agent-vocal-design)
- Do not switch agents if writing agent.selected to config.toml fails; shown and saved choice must match. (spec:2026-10-10-agent-panneau-configuration-design)
- Reuse an agent connection only if it is open and belongs to the current launch epoch. (spec:2026-10-10-agent-panneau-configuration-design)
- Persist only a sha256 fingerprint of the agent command and args, never the command in clear. (spec:2026-10-08-sessions-par-perspective-design)
- Ignore late agent commands for a closed tab; tab ids are never reused. (spec:2026-09-27-agent-onglets-design)
- A prompt during a turn cancels and replaces only that tab's turn; never touch other tabs. (spec:2026-09-27-agent-onglets-design)
- Kill the agent and report it stopped if its handshake is not done within READY_WITHIN (120 s). (spec:2026-09-27-agent-onglets-design)
- Call Stack after_change only for requests that change the deck. (t-249, t-252)
- Keep started and closed tabs under one stack table lock so a closed tab never starts again. (t-253)
- Send the import instructions only in a Tune turn, hidden before the context; the thread shows only its short text. (t-692)
- Decide whether the local agent can run only through oku_slides_platform::local_agent::local_engine, in the host and the child alike. (plan:2026-10-18-agent-local-repli-lot3)
- Start a Tune turn only from the host through Stack::tune; never deserialize AgentCommand::Tune from the UI. (t-692)

## Pitfalls

- Hold only a weak hub reference in relay tasks, or the hub and the Stack are never dropped. (plan:2026-09-26-agent-vocal)
- Keep the ACP session known when a request fails on a closed connection; the keeper resumes it later. (plan:2026-10-08-sessions-par-perspective)
- When the MCP relay lags, resend only visible_surfaces, the thread being viewed. (plan:2026-10-08-sessions-par-perspective)
- Switch agents inside the shared Link, never replace it; dropping its last clone stops the agent. (spec:2026-10-10-agent-panneau-configuration-design)
- Keep is_cancelled after ToolServer start and the relay biased selects; only next is test-pinned. (t-815)
- Do not rely on widget_touched being replayed after relay lag; only Changed and Surfaces are replayed. (plan:2026-10-03-widgets-agent-a2ui)

## Testing

- In stack tests, never sample relayed events right after handle; wait for an event and continue from it. (plan:2026-10-09-agent-detachable)
- Test ACP and multi-tab flows against the scripted fake agent in src/bin/fake_agent.rs, not a real agent. (spec:2026-09-27-agent-onglets-design)
- Point models.hub_url of every test stack at a refused local port; no test may reach the real model hub. (plan:2026-10-18-agent-local-repli-lot3)

## Settled decisions

- Do not resume a session recorded under another agent fingerprint; open a new session instead. (plan:2026-10-10-agent-panneau-configuration)
- Show the memory-lost card only when a resume was attempted and refused, not when none was tried. (plan:2026-10-09-tickets-debloques-t598)
- Show free agent text only as one-line turn narration, never as an answer, except in the fallback receipt. (spec:2026-10-03-agent-a2ui-design)
- Let users only pick configured agent presets; never add or edit agent commands from the UI. (spec:2026-10-10-agent-panneau-configuration-design)
- Let the UI compute and send the agent scope; never derive it from the session mode on the host. (spec:2026-10-08-sessions-par-perspective-design)
- Keep one ACP session and thread per scope (Slides, Workshop, Widget name), restored on reopen. (t-598)
- Write agent instructions in English only; pass the interface language in each turn context. (t-774, t-771)
- Pick the deck language as declared params.lang, else inferred from slides, else the user's language. (t-780)
- Hold every import tuning bound in session code that refuses with TuneError; import.md only explains the bounds and never enforces them. (ADR-0009, t-924)
