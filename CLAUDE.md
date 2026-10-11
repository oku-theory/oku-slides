# Oku Slides

Voice-driven slide editor: Rust workspace (`crates/oku-slides-*`), Tauri 2 app (`src-tauri`),
React/TS UI (`ui`, Bun), Typst library (`typst/oku-slides`). Crate map: README.md.
Before touching a crate, read its `ARCHITECTURE.md` when it has one (and keep it current).

## Language

Everything written in the repo is **French**: commits, ADRs, specs/plans, code comments,
tool output. README is the exception (English). User-facing strings go through Fluent
(`oku-slides-i18n`, English is the reference locale).

## Commands (from repo root)

```bash
just test          # nextest (excludes oku-slides-app) + UI tests
just check         # fmt --check, clippy -D warnings, tsc, fonts-lfs
just ci            # check + test + audit — the full pre-merge gate
just e2e 4999      # Playwright; default port 4872, refuses a port already taken
just web           # UI with HMR on :5173 + Rust API
just oku-slides …  # CLI, e.g. check, render, save, open, import, serve (see --help)
```

Ports in use: 4870 `just serve`, 4871 `.claude/launch.json` preview, 4872 `just e2e`,
5173 `just web`. Pick another one when parallel sessions need a server.

## Gotchas

- Every run outside `just e2e` (`just dev/web/serve`, the launch.json preview, the raw
  binary) writes into the real `~/Library/Application Support/me.dartiguenave.oku-slides`
  and `~/Documents/Oku Slides`. Set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS`
  to a **fresh** temp dir per run (also in subagent prompts): a reused dir restores the
  previous run's tabs and fails e2e specs falsely. `just e2e` handles this itself.
- The launch.json preview runs from the main checkout and serves its `ui/dist`, so it
  does not show a worktree's changes.
- UI tests: only `*.render.test.tsx` (happy-dom) run in a second Bun process; every other
  `*.test.ts(x)` runs in the first. A new e2e **spec** must be appended to the `e2e` script
  in `ui/package.json` (`http.cjs`, `stage.cjs`, `until.cjs` are helpers, not specs).
- `.cargo/config.toml` `MACOSX_DEPLOYMENT_TARGET` must equal
  `bundle.macOS.minimumSystemVersion` in `src-tauri/tauri.conf.json`, or typst rebuilds on
  every `just dev` / `just test` switch.
- Git LFS holds `assets/fonts/*.{ttf,otf}` and `fixtures/import/*.pdf` — commit pointers,
  never the binaries.

## Workflow

- Commits: `type(scope) : description (t-N)` — note the space before the colon; `!` for
  breaking. Every commit cites its Oku task(s).
- Specs: `docs/superpowers/specs/YYYY-MM-DD-<slug>-design.md`; plans: `docs/superpowers/plans/`.
- Decisions: `docs/adr/` (see its README).
