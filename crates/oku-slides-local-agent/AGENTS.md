# crates/oku-slides-local-agent — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Never write to stdout in this crate; stdout is the ACP pipe, logs go to stderr. (spec:2026-10-18-agent-local-repli-design)
- Map every MCP tool to a tool family; the completeness test fails otherwise. (spec:2026-10-18-agent-local-repli-design)
- Accept the fake engine only in debug builds. (plan:2026-10-18-agent-local-repli-lot1)
- End every turn with the report tool, a localized failure report if needed; only a cancel skips it. (plan:2026-10-18-agent-local-repli-lot1)
- Choose the engine only through local_engine; a forced engine that is unavailable fails with its reason. (spec:2026-10-18-agent-local-repli-design)
- Keep every llama.cpp call in gguf.rs, on its one worker thread; never share a LlamaContext. (plan:2026-10-18-agent-local-repli-lot3)

## Pitfalls

- Drop the GgufEngine before the process exits; Metal aborts at exit while a model is still loaded. (plan:2026-10-18-agent-local-repli-lot3)

## Testing

- Test turns with the scripted fake engine and a fake tool port, never a real model. (plan:2026-10-18-agent-local-repli-lot1)
- Initialise llama.cpp only in the ignored manual test of gguf_tests.rs, never in another test. (plan:2026-10-18-agent-local-repli-lot3)
