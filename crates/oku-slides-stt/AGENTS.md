# crates/oku-slides-stt — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Keep all unsafe code of the stt crate in apple.rs, with a SAFETY comment on every unsafe block. (spec:2026-10-03-dictee-native-macos-design)
- In the Swift bridge, call free(ctx) once after the last event; never use try! or force unwraps. (spec:2026-10-03-dictee-native-macos-design)
- Keep the C ABI and the stream::raw event and error codes identical on the Rust and Swift sides. (plan:2026-10-03-dictee-native-macos)
- Have Swift callbacks only send on an mpsc channel; only the Rust dictation thread may write Inner. (spec:2026-10-03-dictee-native-macos-design)
- Migrate old dictation config once at load, in memory only; configure always writes CONFIG_VERSION. (spec:2026-10-03-dictee-native-macos-design)

## Testing

- Set stt.apple to false in tests built on Stt::new so Whisper tests stay deterministic on macOS 26. (spec:2026-10-03-dictee-native-macos-design)

## Settled decisions

- Use the Apple engine only on macOS 26+ with stt.apple enabled; keep Whisper as the engine elsewhere. (plan:2026-10-03-dictee-native-macos, t-393)
- Never silently fall back from the Apple speech engine to Whisper; grey out unsupported languages instead. (spec:2026-10-03-dictee-native-macos-design)
