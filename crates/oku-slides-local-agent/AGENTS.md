# crates/oku-slides-local-agent — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Never write to stdout in this crate; stdout is the ACP pipe, logs go to stderr. (spec:2026-10-18-agent-local-repli-design)
- Map every MCP tool to a tool family; the completeness test fails otherwise. (spec:2026-10-18-agent-local-repli-design)
- Accept the fake engine only in debug builds. (plan:2026-10-18-agent-local-repli-lot1)
- End every turn with the report tool, a localized failure report if needed; only a cancel skips it. (plan:2026-10-18-agent-local-repli-lot1)

## Testing

- Test turns with the scripted fake engine and a fake tool port, never a real model. (plan:2026-10-18-agent-local-repli-lot1)
