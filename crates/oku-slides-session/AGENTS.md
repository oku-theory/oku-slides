# crates/oku-slides-session — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Require render to be a strict tree (unique ids, no sharing, cycles or orphans, depth 6, 40 components max); reject agent texts with bidirectional control or invisible characters. (spec:2026-10-03-agent-a2ui-design)
- Lock the workspace before a session, never the reverse, only try_lock a session under it, and never read files, render or build a session under it. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Never reuse tab or window ids, accept only valid unique t-N and w-N ids from workspace.json, which only the Workspace writes; a session writes only its tab folder. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Reread dormant tabs before a destructive decision such as close; if unreadable, take the cautious path. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Render the new deck before committing it to the replica so a failed render leaves no trace in Loro history. (spec:2026-09-27-sources-loro-design)
- Mirror every new Request variant in ui/src/api.ts and every WorkspaceRequest variant in ui/src/workspaceApi.ts. (plan:2026-09-27-sources-loro-fusion, t-936)
- Write resume and workspace files via Storage::write_atomic, logging failures with tracing::warn, never failing the request. (spec:2026-09-26-resume-state-design)
- Write resume files as deck.loro, document.json, then state.json (a Ui request only state.json); an unknown state version is unreadable: log, start blank. (spec:2026-09-26-resume-state-design)
- In history mode transitions, put moved assets and fonts back before propagating any error. (plan:2026-09-27-sources-loro-historique)
- Validate SetToken and UnsetToken ops before applying any, Color and Palette ones outside pack mode too. (plan:2026-10-05-themes-packs-editeur)
- Resolve and render deck widgets only from the pack version embedded in SourceBundle.packs, never the library; check each pin with check_pack_id, in the session and ProbedWidgets alike. (plan:2026-10-02-creation-widgets, t-1008)
- Never let a received deck change a pack's current library version: add its versions via receive_version, and make the current fallback prefer locally created versions whatever a received created says. (spec:2026-10-01-packs-portables-design, t-355)
- Pin base and essentiel where slides or elements are produced (new deck, import, help, pack mode), before probing, never on open; build a first replica before the deck's packs resolve with builtin_registry, a slot without packs with WidgetRegistry::default(). (plan:2026-10-18-packs-lot2-base, plan:2026-10-19-packs-lot3-essentiel)
- Write a pack version's widget files before its widgets.json in the pack library; refuse a version whose namespace differs from its pack's earlier ones (NamespaceChanged); a built-in namespace (base, essentiel) stays its built-in pack's across kinds, a user copy holding it is read by id only, never an owner, a lookup result or a new version. (plan:2026-10-02-creation-widgets, plan:2026-10-18-packs-lot1-widget-packs, t-1008, t-1068)
- In deck mode, send a widget write without a namespace to deck (element) or deck-layouts (slide) by its probed category, refuse a named pack of the other kind; embed and render before writing the library; on failure change nothing. (spec:2026-10-02-creation-widgets-design, t-1016, plan:2026-10-19-packs-lot3-essentiel)
- End the deck-mode widget edit session at the first non-widget op and at begin_turn and end_turn. (spec:2026-10-02-creation-widgets-design)
- Reject widget sources containing a NUL byte both when reading packs and when writing a widget. (spec:2026-10-02-creation-widgets-design)
- Reference receipt slides by id and hash; never embed the slide SVG in agent receipts. (spec:2026-10-03-agent-a2ui-design)
- Address host settings fields by the key carried in the action, never by position. (spec:2026-10-03-widgets-agent-a2ui-design)
- In pack mode, refuse add_slide, remove_slide, move_slide and set_kind on a slide before any op runs (PackModeSlides); make each widget write one working-deck undo step, so a turn is one undo; keep save_pack silent; never let undo or redo change the library current version (realign_pack never calls set_current). (plan:2026-10-04-widget-agent-annulable-modeles, plan:2026-10-19-packs-lot3-essentiel)
- Recognize workspace windows only through is_window_label; do not redefine the w-N pattern. (plan:2026-10-09-agent-detachable)
- Define KEPT_TURNS only once, in the oku-slides-session agent module, and reuse it elsewhere. (spec:2026-10-08-sessions-par-perspective-design)
- Move a widget conversation on rename, undo or redo, never overwriting one; refuse keep_thread and keep_agent_session for unknown Widget scopes. (plan:2026-10-08-sessions-par-perspective)
- On save, count only NotFound as an unchanged file; treat any other read error as disk replaced. (spec:2026-09-27-sources-loro-design)
- Re-render once on a Stale handle; on a bounded failure disable only the faulty pack and its dependents, never core widgets for the whole deck; if that fails, log it and return the original error. (spec:2026-09-29-compilation-bornee-design, t-344, t-1015, plan:2026-10-18-packs-lot1-widget-packs)
- Bisect only untrusted packs, keeping the shipped built-ins in the control render, never disable a trusted pack; a registry that fails to assemble disables no pack and returns the error, in ProbedWidgets too when the set holds a trusted pack. (plan:2026-10-18-packs-lot1-widget-packs, t-1008, plan:2026-10-18-packs-lot2-base, t-1057)
- Reach a tab session from the agent only via Workspace::with_session, which drops the workspace lock first, and never after Workspace::settle at app shutdown. (spec:2026-09-27-agent-onglets-design, t-251)
- Build design variants on a throwaway deck clone and fail the whole call if any variant is invalid. (spec:2026-09-26-agent-vocal-design)
- Count import tuning tries on the scored raster's fingerprint, never page_hash; draw the global budget only for an unchanged slide model; key the deck state by theme and pack content, never versions. (t-692)
- Implement the PowerPoint import only in oku_slides_session::import (the CLI and app only call it); check the pptx size against the [import] max_input_mib setting before reading it. (ADR-0009, t-692, t-908, plan:2026-10-10-import-pptx-lot2)
- Apply the whole import in one transaction and history step through the normal validations; fail a retrofit whose target theme tokens cannot be read, never using defaults. (plan:2026-10-10-import-pptx-lot2, spec:2026-10-09-import-pptx-design, t-817)
- Keep ImportState in memory only: never resumed, cleared by load_document once the new document is set. (t-692)
- Choose the import witness only in ImportState::choose_witness, never replacing a missing soffice by office2pdf, and never run it under the session lock; callers run the ReferenceJob from import_reference_job first, in a blocking task. (t-692, t-905)
- Retrofit only onto layout packs, by id or name in the app (never a folder), their profiles in the given order; an import makes no child pack. (t-692, t-1016, plan:2026-10-19-packs-lot3-essentiel)
- Bump the library generation file on every library write, and re-hash a version's bytes on load; never trust the in-memory index for content. (t-355)
- Shorten agent text shown as is only with agent::bounds::clip, which drops invisible and bidi controls and counts the ellipsis. (t-842)

## Pitfalls

- Guard session edits with self.ensure_editable(); there is no read_only() or in_history() helper. (plan:2026-10-05-parametres-a-la-selection)

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
- Model help as Mode::Help, exclusive of pack mode and history, and add no request whitelist to it. (plan:2026-10-09-tickets-debloques-t598, t-489)
- Undo an agent turn only while its steps are on top of the deck undo stack (TurnNotOnTop); never restore a pre-turn state under later edits. (t-417)
