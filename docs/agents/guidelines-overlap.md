# Guidelines overlap registry

Rules removed from the root `AGENTS.md` because the oku-theory plugins already inject them at session start
(`rust/hooks/session-start.sh`, `common/hooks/session-start.sh`). Kept here so their sources stay traceable.
A rule is "partial" when only its generic part moved here and the repo-specific delta stays in `AGENTS.md`.

| Rule (removed text) | Covered by (hook line) | Sources |
|---|---|---|
| No std::fs in production code, go through Storage (OpenDAL); fine in tests | rust Storage: "Use OpenDAL for all file operations; no direct `std::fs` calls" (the tests exception is not stated) | plan:2026-09-26-onglets-fenetres |
| Aim for 500-line files, never exceed 1000 | rust Files row and common File Organization: "Target: 500 lines per file. Hard limit: 1000 lines" | plan:2026-10-10-import-pptx-lot2 |
| Never use unwrap or expect outside tests (partial: the silent-fallback half stays) | rust Safety: "No `.unwrap()` or `.expect()` in production code" | spec:2026-09-26-agent-vocal-design, plan:2026-10-10-deck-en-arbre |
| Errors with Snafu, backtrace in every variant, opaque public types (partial: Fluent messages stay) | rust Error Handling: "Always include `backtrace: Backtrace` in every error variant", "Opaque errors for library APIs" | spec:2026-09-27-widgets-typst-design |
| Dependency versions in workspace.dependencies, `.workspace = true` in crates (partial: thiserror/chrono ban stays) | rust Deps row: "All versions in workspace `Cargo.toml`; subcrates use `.workspace = true`" | plan:2026-09-26-agent-vocal, t-5 |
| Use `expect` with a reason instead of `allow` (partial: dead_code rule stays) | rust Code Style: "`#[expect(..., reason = \"...\")]` instead of `#[allow]`" | plan:2026-10-03-widgets-agent-a2ui |
| Document every pub item and new module, first sentence under 15 words (partial: the hook covers the 15-word rule and public modules, not every pub item) | rust and common Code Style: "Doc first sentence under 15 words", "All public modules must have `//!` documentation" | plan:2026-10-10-arbre-deck-robustesse |
| Name tests unit_scenario_result, one behavior per test, rstest for table cases | common Test Naming: "<unit>_<scenario>_<expected_result>"; rust Testing: parametrized tests with `rstest` | plan:2026-10-09-agent-detachable, plan:2026-10-09-elements-en-arbre |
| Conventional Commits in English, `type(scope): description (t-N)`, `!` for breaking | common Commits: "Conventional Commits (required)", "Task Reference (required)", breaking `!` | (no source in the removed Workflow line) |
