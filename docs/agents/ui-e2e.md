# UI tests and e2e

Read before writing or running a UI test or an e2e spec.

- Only `*.render.test.tsx` (happy-dom) run in a second Bun process; every other `*.test.ts(x)` runs in the first, with no DOM (importing `@a2ui/react` needs one).
- e2e specs are `ui/e2e/NNN-name.cjs`, run in number order against one shared server (later specs rely on earlier ones); helpers go in `e2e/lib/`.
- Specs read French labels (system locale, not forced); `E2E_AGENT_LOG` is the fake agent's log.
- Use a fresh `OKU_SLIDES_PATHS__DATA` and `__DECKS` per run (see the root `AGENTS.md` Gotchas); `just e2e` does it itself.
