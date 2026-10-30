# Task 8 report (t-1275)

Status: DONE_WITH_CONCERNS

## Files (staged)
- agent: instructions/{agent,deck,pack,theme}.md, instructions.rs (`for_kind`, doc translated), instructions_tests.rs, turn.rs, tests/stack.rs, tests/turns.rs, ARCHITECTURE.md
- mcp: tools.rs (`tool_descriptions_for`, tool names in descriptions backticked), lib.rs (re-export), tools_tests.rs
- session: agent/context.rs (+tests, snapshot agent__agent_context_describes_mode_...snap), ARCHITECTURE.md

## Decisions
- Core agent.md keeps role, ask/report/render rules, interface and dictation tools, pasted images, languages. Everything deck-specific moved to deck.md (incl. show_themes, slide kinds, layouts).
- turn.rs: INSTRUCTIONS, for_kind(ctx.kind), WIDGETS unless Theme, IMPORT when tuning. The debug log with ctx.kind is kept.
- Context: new `kind: <deck|help|pack|theme>` line after `mode:` (mode line unchanged); pack tab: `pack: <name>` and `preview theme: <namespace>`; theme tab: `theme: <draft name>`. Names go through bounds::label. Done in helper `write_kind_lines` (agent_context hit clippy too_many_lines).
- Descriptions: backticked underscore tool names only (plain-word tools such as history, move, apply stay prose). `packs` description reworded ("A deck tab opens one in its own tab.") because it named open_pack for all kinds; `set_widget` names the add_pack op unbackticked (collision). `apply` description names (add_pack, remove_pack) again as ops, unbackticked, with a test.
- Languages paragraph in core left unbackticked: existing tests pin its wording and it names set_widget_trial, which a theme tab lacks.
- Existing tests updated: stack.rs/turns.rs block order, tools_tests.rs backticks, menu-tool tests re-pointed to deck/core.

## Tests
- `cargo nextest run -p oku-slides-agent -p oku-slides-mcp -p oku-slides-session --retries 2 --no-fail-fast`: 4056 passed (1 timing flake in launch.rs under load, passed on retry), run before the final context.rs refactor; after it: session context tests 33/33, tools_tests 33/33.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean. `just agents-lint`: ok.
- TDD deviation: tests and implementation were written together; the red run was not recorded separately.
