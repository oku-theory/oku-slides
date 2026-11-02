# crates/oku-slides-apple-model — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Keep all unsafe code of this crate in bridge.rs, with a SAFETY comment on every unsafe block. (plan:2026-11-01-lot-l-local-agent)
- In the Swift bridge, guard every FoundationModels use with #available(macOS 26.0, *), send one event then free(ctx) once, and never use try! or force unwraps. (plan:2026-11-01-lot-l-local-agent)
- Keep the C ABI and its availability and error codes identical in bridge.rs and OkuModel.swift; bridge_tests reads the Swift constants. (plan:2026-11-01-lot-l-local-agent)
- Keep prompt, schema and reply logic out of the bridge; it only decodes the schema JSON and runs one guided reply. (plan:2026-11-01-lot-l-local-agent)

## Pitfalls

- Keep FoundationModels weak-linked through #available alone; local-agent's weak_link test checks LC_LOAD_WEAK_DYLIB, since a build.rs link-arg never reaches binaries. (plan:2026-11-01-lot-l-local-agent, t-1462)

## Testing

- Call Apple's real model only in the ignored manual tests; other tests use the code tables and never the system model. (plan:2026-11-01-lot-l-local-agent)
