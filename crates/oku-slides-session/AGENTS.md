# crates/oku-slides-session — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Keep host-only components (Prompt, Timeline, Receipt, SlideThumb) refused in the agent render tool. (spec:2026-10-03-agent-a2ui-design)
- Require render to be a strict tree: unique ids, no sharing, cycles or orphans, depth 6 max, 40 components max. (spec:2026-10-03-agent-a2ui-design)
- Reject agent texts containing bidirectional control or invisible characters. (spec:2026-10-03-agent-a2ui-design)
- Lock the workspace before a session, never the reverse; only try_lock a session under the workspace lock. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Never read files, render or build a session under the workspace lock; recover poisoned locks via into_inner. (ADR-0001)
- Never reuse tab or window ids; accept only valid unique t-N and w-N ids from untrusted workspace.json. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Let only the Workspace write workspace.json; each session writes only its own tab folder. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Reread dormant tabs before a destructive decision such as close; if unreadable, take the cautious path. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Render the new deck before committing it to the replica so a failed render leaves no trace in Loro history. (spec:2026-09-27-sources-loro-design)
- Mirror every new Request or WorkspaceRequest variant in the TypeScript unions of ui/src/api.ts. (plan:2026-09-27-sources-loro-fusion)
- Write resume and workspace files via Storage::write_atomic; log failures with tracing::warn, never fail the request. (spec:2026-09-26-resume-state-design)
- Write resume files as deck.loro, then document.json, then state.json; a Ui request writes only state.json. (spec:2026-09-26-resume-state-design)
- Treat an unknown resume state version as unreadable: log it and start from a blank deck. (spec:2026-09-26-resume-state-design)
- In history mode transitions, put moved assets and fonts back before propagating any error. (plan:2026-09-27-sources-loro-historique)
- Validate SetToken and UnsetToken ops before applying any, Color and Palette ones outside templates mode too. (plan:2026-10-05-themes-packs-editeur)
- Resolve and render deck widgets only from the pack version embedded in SourceBundle.packs, never the library. (plan:2026-10-02-creation-widgets)
- Never let a received deck change a pack's current library version; add received versions silently. (spec:2026-10-01-packs-portables-design)
- Write a pack version's widget files before its pack.json in the template library. (plan:2026-10-02-creation-widgets)
- In deck mode, embed and render a widget version before writing the library; on failure change nothing. (spec:2026-10-02-creation-widgets-design)
- End the deck-mode widget edit session at the first non-widget op and at begin_turn and end_turn. (spec:2026-10-02-creation-widgets-design)
- Reject widget sources containing a NUL byte both when reading packs and when writing a widget. (spec:2026-10-02-creation-widgets-design)
- Reference receipt slides by id and hash; never embed the slide SVG in agent receipts. (spec:2026-10-03-agent-a2ui-design)
- Address host settings fields by the key carried in the action, never by position. (spec:2026-10-03-widgets-agent-a2ui-design)
- In templates mode, make each widget write one templates-doc undo step so an agent turn is one undo. (plan:2026-10-04-widget-agent-annulable-modeles)
- Keep save_templates silent, without creating an undo step. (plan:2026-10-04-widget-agent-annulable-modeles)
- Never let undo or redo change the library current version; realign_templates never calls set_current. (plan:2026-10-04-widget-agent-annulable-modeles)
- Recognize workspace windows only through is_window_label; do not redefine the w-N pattern. (plan:2026-10-09-agent-detachable)
- Define KEPT_TURNS only once, in the oku-slides-session agent module, and reuse it elsewhere. (spec:2026-10-08-sessions-par-perspective-design)
- Move a widget conversation when the widget is renamed, also on undo or redo, never overwriting an existing one. (plan:2026-10-08-sessions-par-perspective)
- Refuse keep_thread and keep_agent_session for unknown Widget scopes so an invented scope writes nothing. (plan:2026-10-08-sessions-par-perspective)
- Serialize AgentScope as kind-tagged JSON (slides, atelier with pack, widget with pack and name). (plan:2026-10-08-sessions-par-perspective)
- On save, count only NotFound as an unchanged file; treat any other read error as disk replaced. (spec:2026-09-27-sources-loro-design)
- Re-render once on a Stale handle; if a fallback to base widgets fails, log it and return the original render error. (spec:2026-09-29-compilation-bornee-design, t-344)
- Reach a tab session from the agent only via Workspace::with_session, which drops the workspace lock first. (spec:2026-09-27-agent-onglets-design)
- Never access tab sessions after Workspace::settle at app shutdown. (t-251)
- Build design variants on a throwaway deck clone and fail the whole call if any variant is invalid. (spec:2026-09-26-agent-vocal-design)

## Pitfalls

- Guard session edits with self.ensure_editable(); there is no read_only() or in_history() helper. (plan:2026-10-05-parametres-a-la-selection)
- Call TabSession::with only off the async runtime, for example in spawn_blocking, because it blocks. (plan:2026-09-27-agent-onglets)

## Settled decisions

- Make the agent open tool use a new tab beside its own, or activate the tab already holding the file. (t-254)
- Never make allow_always the primary permission option; primary is allow_once or none. (t-222)
- Give each tab session its own lock; never wrap all sessions in one Mutex owned by the Workspace. (ADR-0001)
- Undo only through the Loro UndoManager, rebuilt empty on open or resume; never persist it or bring back History. (ADR-0002)
- Never change the built-in theme and pack UUIDs or reintroduce slugs; decks reference them by id. (plan:2026-10-05-themes-packs-editeur)
- Track agent provenance via set_widget_by_agent in the session; never add an origin flag to requests. (spec:2026-10-03-widgets-agent-a2ui-design)
- Build settings forms only through forms::params and params_with; never add a second A2UI builder. (spec:2026-10-03-widgets-agent-a2ui-design)
- Put new impl Session code, agent code included in src/agent, in its own module; add only match arms to lib.rs. (plan:2026-10-08-widget-tableau)
- Expose UI-actionable states such as disk_replaced as snapshot flags, never as session errors. (spec:2026-09-27-sources-loro-design)
- Model help as Mode::Help, exclusive of templates and history, and add no request whitelist to it. (plan:2026-10-09-tickets-debloques-t598, t-489)
