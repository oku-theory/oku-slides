# crates/oku-slides-local-agent — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Never write to stdout in this crate; stdout is the ACP pipe, logs go to stderr. (spec:2026-10-18-agent-local-repli-design)
- End every turn with the report tool, a localized failure report if needed; only a cancel skips it. (plan:2026-10-18-agent-local-repli-lot1)
- Choose the engine only through local_engine; a forced engine that is unavailable fails with its reason. (spec:2026-10-18-agent-local-repli-design)
- Keep every llama.cpp call in gguf.rs, on its one worker thread; never share a LlamaContext. (plan:2026-10-18-agent-local-repli-lot3)
- Build every Gguf prompt through chatml::render; gguf tokenizes special tokens, so inserted text must be neutralised there. (plan:2026-10-18-agent-local-repli-lot3, t-1047)

## Pitfalls

- Drop the GgufEngine before the process exits; Metal aborts at exit while a model is still loaded. (plan:2026-10-18-agent-local-repli-lot3)
- Change AGENT_MODEL only together with chatml.rs and reply.rs; llama.cpp skips the GGUF chat template, so Qwen3 ChatML is hand-written. (spec:2026-10-18-agent-local-repli-design)
- Load the model before serving ACP, within READY_WITHIN (120 s); a new binary's first LlamaBackend::init compiles Metal shaders (~23 s). (spec:2026-10-18-agent-local-repli-design)
- Gate every gguf or llama-cpp use behind cfg(all(target_os = "macos", target_arch = "aarch64")); other targets lack the dependency. (spec:2026-10-18-agent-local-repli-design)

## Testing

- Test turns with the scripted fake engine and a fake tool port, never a real model. (plan:2026-10-18-agent-local-repli-lot1)
- Initialise llama.cpp only in the ignored manual test of gguf_tests.rs, never in another test. (plan:2026-10-18-agent-local-repli-lot3)

## Settled decisions

- Start the local agent with the --local-agent argument, never an OKU_SLIDES_LOCAL_AGENT env var, which collides with the [local_agent] section. (spec:2026-10-18-agent-local-repli-design)
