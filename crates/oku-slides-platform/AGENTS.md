# crates/oku-slides-platform — agent rules

## Invariants

- Change AgentSettings presets and selection only through new, select and insert_preset; the active agent stays valid. (t-1202)
- Fail config loading when agent.selected names no preset; never silently fall back to the first agent. (spec:2026-10-10-agent-panneau-configuration-design)
- Never follow symlinks in Storage::delete_dir, and read untrusted files with read_limited and a size cap. (t-163)
- Give each agent preset exactly one of command or builtin, and fail config loading otherwise. (t-1020, spec:2026-10-18-agent-local-repli-design)
- Download model files only through models::download, which checks pinned SHA-256 and writes .complete last. (plan:2026-10-18-agent-local-repli-lot3)
- Build a shipped binary's Platform only with Platform::load; with_settings renders InProcess, unbounded and unsandboxed. (spec:2026-10-17-render-worker-sandbox-design, plan:2026-09-29-compilation-bornee)

## Pitfalls

- Use a unique temp file per Storage::write_atomic call, since two tabs may write in parallel. (plan:2026-09-26-onglets-fenetres)

## Settled decisions

- Pin shipped ACP agent presets to exact package versions in config.default.toml. (plan:2026-10-10-agent-panneau-configuration)
- Declare agents as a named table agent.presets.ID, not an array, so config.toml merges by key. (spec:2026-10-10-agent-panneau-configuration-design)
- Add settings to the config layer with an embedded default, overridable by config.toml and env vars, not hardcoded. (t-11)
- Never put a UI locale in the default settings; the system language applies unless the user forces one. (t-45)
