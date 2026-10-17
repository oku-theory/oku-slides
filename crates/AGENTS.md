# crates — agent rules

## Invariants

- Compute a theme's effective tokens only through oku_slides_core::tokens::effective_tokens, never a local copy. (plan:2026-10-10-import-pptx-lot2)
- Create Renderer or Registries directly only in worker serve.rs, the CLI and render tests; else use RenderWorker. (plan:2026-09-29-compilation-bornee)
- Verify packs only via PackFiles::matches or PackVersion::verifies; adopt the default pack only via adopt_default_pack. (plan:2026-10-02-creation-widgets)
- Refuse widget files over max_widget_kib or beyond max_widgets per version (counting _commun), and never execute them. (plan:2026-10-02-creation-widgets)
- Never mutate sources.deck directly; change the deck only through SourceBundle apply or apply_silent. (plan:2026-09-27-sources-loro-fondation)
- Return MCP tool and agent-form errors localized via Fluent in the user's language, never as raw English text. (t-403, t-650)
- Localize hub, MCP tool and agent texts via LiveLocalizer::current at each use, never a launch snapshot. (t-793)
- Keep dataset ids and columns aligned with source on every core write; rereads only repair merged states. (ADR-0004)
- Never make oku-slides-core depend on oku-slides-import; import may depend only on core and i18n. (plan:2026-10-09-import-pptx-lot1)
- Never write a deck-only pack or theme (PackScope::Deck) to the library; its new versions stay deck-only. (t-692)

## Pitfalls

- Keep agent.selected among agent.presets when tests edit settings by hand, or active() panics. (plan:2026-10-10-agent-panneau-configuration)
- Do not add create_dir_all or exists checks around storage; write_atomic, list_files and delete handle absence. (plan:2026-10-08-sessions-par-perspective)

## Testing

- Make probe budget tests deterministic: use probe_budget_ms = 0 and assert behaviour, never measured elapsed time. (t-571, t-576)

## Settled decisions

- Serve HTTP through the bounded accept loop in oku-slides-platform listen, never bare axum::serve. (t-637)
- Treat a version 1 sources envelope in a PDF as absent and never convert it. (ADR-0002)
- Resolve a language with oku_slides_i18n::resolve instead of building a full Localizer. (t-490)
