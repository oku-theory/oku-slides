# oku-slides-apple-model

The Swift bridge to Apple's on-device language model (Foundation Models, macOS 26+ with
Apple Intelligence on), for the local fallback agent. A leaf crate: `oku-slides-platform`
asks it for availability (`local_engine`), `oku-slides-local-agent` for replies; one crate
links the bridge.

## Modules

- `lib.rs`: the safe API. `availability() -> AppleModel` (`Ready`, `DeviceNotEligible`,
  `IntelligenceOff`, `ModelNotReady`, `MacOsTooOld`, `Unknown`); `respond(&Request,
  &CancellationToken) -> Result<String, AppleModelError>`, blocking: instructions, prompt,
  the JSON Schema of the reply, temperature, reply token cap; it returns the generated JSON.
  `AppleModelError` (Snafu, opaque, English, logs only) has a `kind()`: `ContextOverflow`,
  `Refused` (guardrail or refusal), `Failed`, `Cancelled`.
- `bridge.rs`: the only `unsafe` code. The C ABI and its code tables (`AVAILABILITY`,
  `ERRORS`, kept equal to `OkuModel.swift`, checked by `bridge_tests`). `respond` boxes a
  `Sender` as the callback context, waits on its channel and polls the token every 25 ms; on
  cancel it calls `oku_model_cancel` and returns at once (the late event finds no receiver).
  Off macOS: `MacOsTooOld`, and `respond` fails.
- `swift/` (package `OkuModel`, built by `build.rs` through `swift-rs`, deployment target
  12.0): `oku_model_availability`, `oku_model_respond` (a fresh `LanguageModelSession` with the
  instructions, the schema decoded into `GenerationSchema`, one guided `respond`; one event
  `cb(ctx, kind, code, text)` then `free(ctx)`), `oku_model_cancel`, `oku_model_release`.

## Linking

`FoundationModels` exists from macOS 26 only. Every Swift use sits behind
`#available(macOS 26.0, *)`, so the compiler weak-references its symbols and the linker loads
the framework with `LC_LOAD_WEAK_DYLIB`: the app still starts on macOS 12. No explicit
`-weak_framework`: a dependency's `rustc-link-arg` never reaches the binaries. The test
`oku-slides-local-agent/tests/weak_link.rs` checks the load command and `minos 12.0` with `otool -l`.

## Dependencies

`snafu`, `tokio-util` (the cancel token); `swift-rs` (build, macOS only).
