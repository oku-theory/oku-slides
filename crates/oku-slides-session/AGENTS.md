# crates/oku-slides-session — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Keep agent-form Stepper bounds JS safe integers: NotSafeInteger in Rust, .int().safe() in zod. (t-1204)
- Require render to be a strict tree (unique ids, no sharing, cycles or orphans, depth 6, 40 components max); reject agent texts with bidirectional control or invisible characters. (spec:2026-10-03-agent-a2ui-design)
- Lock the workspace before a session, never the reverse, only try_lock a session under it, never hold two sessions (CarryAssets takes one, then the other), and never read files, render or build a session under it. (ADR-0001, spec:2026-09-26-onglets-fenetres-design, t-1178)
- Never reuse tab or window ids, accept only valid unique t-N and w-N ids from workspace.json, which only the Workspace writes, saving each entry's document with its legacy keys; a session writes only its tab folder. (ADR-0001, spec:2026-09-26-onglets-fenetres-design, plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs)
- Reread dormant tabs before a destructive decision such as close; if unreadable, take the cautious path. (ADR-0001, spec:2026-09-26-onglets-fenetres-design)
- Render the new deck before committing it to the replica so a failed render leaves no trace in Loro history. (spec:2026-09-27-sources-loro-design)
- Mirror every new Request variant in ui/src/api.ts and every WorkspaceRequest variant in ui/src/workspaceApi.ts. (plan:2026-09-27-sources-loro-fusion, t-936)
- Write resume and workspace files via Storage::write_atomic as deck.loro, document.json, then state.json (a Ui request only state.json), logging failures with tracing::warn, never failing the request; an unknown state version is unreadable: log, start blank. (spec:2026-09-26-resume-state-design)
- Lease each library pack or theme a tab edits with PackLease::try_take or acquire, keyed by its own library's lease_scope or theme_scope (opening lock, workspace, session, DRAFTS, then PACK_TABS; never adopt under DRAFTS; the agent gate never blocks), refuse every other write (edit_theme included), make-current, delete or second editor of it meanwhile, keep an editor without the lease read-only, and pass the lease to the pack-mode library writers; write pack and theme states at EDITOR_STATE_VERSION (5), reading pack states of versions 3 and 4 as legacy. (plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs, plan:2026-10-26-onglets-pack-theme-lot4-theme-tabs)
- In history mode transitions, put moved assets and fonts back before propagating any error. (plan:2026-09-27-sources-loro-historique)
- Validate SetToken and UnsetToken ops before applying any against the deck's composed schema (global plus pinned packs' tokens; in a theme tab those of its specimen deck): all of them in a theme tab, only Color and Palette ones elsewhere, pack mode refusing token ops; strict checks accept a component.<ns> subtree of a pack neither pins as is; save an edited built-in theme as a child extending it. (plan:2026-10-05-themes-packs-editeur, plan:2026-10-22-packs-lot4-themes, plan:2026-10-26-onglets-pack-theme-lot4-theme-tabs)
- Pin a deck theme with its whole theme_closure, versions embedded; repair a merge-broken closure from them, else the library; render only with theme_layer, never set_theme(None). (plan:2026-10-22-packs-lot4-themes, t-1098)
- Resolve and render deck widgets only from the pack version embedded in SourceBundle.packs, never the library; check each pin with check_pack_id, in the session and ProbedWidgets alike. (plan:2026-10-02-creation-widgets, t-1008)
- Never let a received deck change a pack's current library version: add its versions via receive_version, and make the current fallback prefer locally created versions whatever a received created says. (spec:2026-10-01-packs-portables-design, t-355)
- Save a pack or theme draft only through draft_write then save_draft under one draft_lock, never holding it twice on a thread, and never while another editor holds that pack's or theme's lease; freeze with adopt every library version a deck in deck mode pins (the session version only on close_pack, the widget draft only when the deck-mode widget session ends), and a pack or theme tab's base, from the library's bytes, when it opens or resumes unsaved; a preview deck never adopts, is installed only through install_document and is never saved to a file; never replace a built-in, received or adopted version. (spec:2026-10-24-onglets-pack-theme-design, plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs, t-1152, plan:2026-10-26-onglets-pack-theme-lot4-theme-tabs)
- Pin base and essentiel where slides or elements are produced (new deck, import, help, pack mode; only those two in a theme tab's specimen deck), before probing, never on open; build a first replica before the deck's packs resolve with builtin_registry, a slot without packs with WidgetRegistry::default(). (plan:2026-10-18-packs-lot2-base, plan:2026-10-19-packs-lot3-essentiel, plan:2026-10-26-onglets-pack-theme-lot4-theme-tabs)
- Write a pack version's widget files before its widgets.json in the pack library; refuse a version whose namespace differs from its pack's earlier ones (NamespaceChanged); a built-in namespace (base, essentiel) stays its built-in pack's across kinds, a user copy holding it is read by id only, never an owner, a lookup result or a new version; reject widget sources containing a NUL byte, reading or writing. (plan:2026-10-02-creation-widgets, plan:2026-10-18-packs-lot1-widget-packs, spec:2026-10-02-creation-widgets-design, t-1008, t-1068)
- In deck mode, send a widget write without a namespace to deck (element) or deck-layouts (slide) by its probed category, refuse a named pack of the other kind; embed and render before writing the library; on failure change nothing; end the widget edit session at the first non-widget op, begin_turn and end_turn. (spec:2026-10-02-creation-widgets-design, t-1016, plan:2026-10-19-packs-lot3-essentiel)
- Reference receipt slides by id and hash; never embed the slide SVG in agent receipts. (spec:2026-10-03-agent-a2ui-design)
- Address host settings fields by the action's key, never by position; apply each host-data DataEdit as one op, its patch computed in data_edit. (spec:2026-10-03-widgets-agent-a2ui-design, t-1122)
- In pack mode, refuse any op on a widget preview slide (PackModePreview), and add_slide, remove_slide, move_slide and set_kind on a slide before any op runs (PackModeSlides); make each widget write one working-deck undo step, so a turn is one undo; keep save_pack silent; never let undo or redo change the library current version (realign_pack never calls set_current). (plan:2026-10-04-widget-agent-annulable-modeles, plan:2026-10-19-packs-lot3-essentiel)
- Recognize workspace windows only through is_window_label; do not redefine the w-N pattern. (plan:2026-10-09-agent-detachable)
- Define KEPT_TURNS only once, in the oku-slides-session agent module, and reuse it elsewhere. (spec:2026-10-08-sessions-par-perspective-design)
- On save, count only NotFound as an unchanged file; treat any other read error as disk replaced. (spec:2026-09-27-sources-loro-design)
- Re-render once on a Stale handle; on a bounded failure disable only the faulty pack and its dependents, never core widgets for the whole deck; if that fails, log it and return the original error. (spec:2026-09-29-compilation-bornee-design, t-344, t-1015, plan:2026-10-18-packs-lot1-widget-packs)
- Bisect only packs not exempted by is_never_blamed (shipped built-ins, any base), keeping exempt ones in the control render and never disabling them for a bounded failure, even when a looping base leaves the deck unrenderable until the element goes; a registry that fails to assemble disables no pack and returns the error, in ProbedWidgets too when the set holds a trusted pack; a trusted pack with a broken widget returns TrustedPackBroken. (plan:2026-10-18-packs-lot1-widget-packs, t-1008, plan:2026-10-18-packs-lot2-base, t-1057, t-1055, t-1053)
- Reach a tab session from the agent only via Workspace::with_session, which drops the workspace lock first, and never after Workspace::settle at app shutdown. (spec:2026-09-27-agent-onglets-design, t-251)
- Build design variants on a throwaway deck clone and fail the whole call if any variant is invalid. (spec:2026-09-26-agent-vocal-design)
- Count import tuning tries on the scored raster's fingerprint, never page_hash; draw the global budget only for an unchanged slide model; key the deck state by theme and pack content, never versions. (t-692)
- Implement the PowerPoint import only in oku_slides_session::import (the CLI and app only call it); check the pptx size against the [import] max_input_mib setting before reading it. (ADR-0009, t-692, t-908, plan:2026-10-10-import-pptx-lot2)
- Apply the whole import in one transaction and history step through the normal validations; fail a retrofit whose target theme tokens cannot be read, never using defaults. (plan:2026-10-10-import-pptx-lot2, spec:2026-10-09-import-pptx-design, t-817)
- Keep ImportState in memory only: never resumed, cleared by load_document once the new document is set. (t-692)
- Choose the import witness only in ImportState::choose_witness, never replacing a missing soffice by office2pdf, and never run it under the session lock; callers run the ReferenceJob from import_reference_job first, in a blocking task. (t-692, t-905)
- Retrofit only onto layout packs, by id or name in the app (never a folder), their profiles in the given order; an import makes no child pack. (t-692, t-1016, plan:2026-10-19-packs-lot3-essentiel)
- Bump the library generation file on every library write, and re-hash a version's bytes on load; never trust the in-memory index for content. (t-355)
- Shorten agent text shown as is only with agent::bounds::clip, which drops invisible and bidi controls and counts the ellipsis; clean deck, manifest and request strings for titles, dialogs and logs only with agent::bounds::label. (t-842, plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs)

## Pitfalls

- Guard session edits with self.ensure_editable(), which also refuses theme tabs; theme-tab paths use ensure_document_editable or ensure_theme_tab. (plan:2026-10-05-parametres-a-la-selection, plan:2026-10-26-onglets-pack-theme-lot4-theme-tabs)

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
