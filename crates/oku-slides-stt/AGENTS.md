# crates/oku-slides-stt — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Gate Apple-bridge-only code with cfg(any(test, target_os = "macos")); never lift dead_code for other platforms. (t-1205)
- Keep all unsafe code of the stt crate in src/apple.rs, src/speak/apple_ffi.rs and src/duplex_ffi.rs, with a SAFETY comment on every unsafe block. (spec:2026-10-03-dictee-native-macos-design, plan:2026-11-01-lot-m-hands-free, plan:2026-11-05-lot-o-backlog)
- In the Swift bridge, call free(ctx) once after the last event; never use try! or force unwraps. (spec:2026-10-03-dictee-native-macos-design)
- Keep the C ABI and the stream::raw event and error codes identical on the Rust and Swift sides. (plan:2026-10-03-dictee-native-macos)
- Have Swift callbacks only send on an mpsc channel; only the Rust dictation thread may write Inner. (spec:2026-10-03-dictee-native-macos-design)
- Migrate old dictation config once at load, in memory only; configure always writes CONFIG_VERSION. (spec:2026-10-03-dictee-native-macos-design)
- Read dictation API secrets only from the variables named in ApiConfig; never store, log or send them to the UI. (spec:2026-10-23-dictee-api-design, t-1213)
- Store, show and call only the normalized_url form of the dictation API address; refuse invalid settings at use. (spec:2026-10-23-reglages-dictee-a2ui-design)
- Check API variables with key_present or missing_variable; read their values only in ApiTranscriber. (spec:2026-10-23-reglages-dictee-a2ui-design, t-1213)
- Show an API error message only for statuses other than 401 and 403, with every secret masked. (spec:2026-10-25-dictee-api-entetes-design)
- Call the dictation API only through async-openai with the client built in ApiTranscriber::new; no hand-made HTTP. (t-1233)
- Send dictation API requests only through api_http::send: one attempt, a bounded reply, the status kept. (t-1233)
- Set the key, base and empty org and project on every async-openai config, so no OPENAI_* variable leaks. (t-1233)
- Load config.json only through saved_config::load; keep an unparseable one aside, log no serde message. (t-1216)
- Hand each utterance out once, only to the host voice router; never put utterance text in Status. (plan:2026-11-01-lot-m-hands-free, t-996)
- Speak only through SpeechEngine; never bundle a voice, call a cloud service or pass SSML; report Unavailable, never fall back to another language; free each speech callback once. (plan:2026-11-01-lot-m-hands-free, t-1002)
- Speak only through the one process Speaker: hold a speech while the user speaks (at most SPEECH_HOLD), suspend listening once the voice took it and SPEECH_GUARD after it unless barge-in applies, end it by watchdog when DONE never comes, and hold no lock across engine calls. (plan:2026-11-01-lot-m-hands-free, t-1002, plan:2026-11-05-lot-o-backlog, t-1491)
- Apply barge-in only through the duplex voice while listening with speech.barge_in on; elsewhere stay muted, and report a failed duplex start in barge_in_unavailable, never silently. (plan:2026-11-05-lot-o-backlog, t-1491)
- Tell a speech cut by barge-in that it ended at its start, so an answer spoken over a question never predates it. (plan:2026-11-05-lot-o-backlog, t-1491)
- On Linux, send speech text only as SSIP SPEAK data, never in an argv or a log, and cut it only with CANCEL self. (plan:2026-11-05-lot-o-backlog, t-1524)
- Keep Oku's recent speech only folded in memory for Speaker::echoes, never one that asks or plays under barge-in; never log, save or show it. (plan:2026-11-05-lot-o-backlog, t-1522)

## Testing

- Set stt.apple to false in tests built on Stt::new so Whisper tests stay deterministic on macOS 26. (spec:2026-10-03-dictee-native-macos-design)
- Time utterances by sample offsets; test closing on silence with sample fixtures, never wall clock. (plan:2026-11-01-lot-m-hands-free, t-996)
- Test AVFoundation main-queue callbacks only from tests/duplex.rs's own main, which runs the main run loop. (plan:2026-11-05-lot-o-backlog, t-1526)

## Settled decisions

- Use the Apple engine only on macOS 26+ with stt.apple enabled; keep Whisper as the engine elsewhere. (plan:2026-10-03-dictee-native-macos, t-393)
- Never silently fall back from the Apple speech engine to Whisper; grey out unsupported languages instead. (spec:2026-10-03-dictee-native-macos-design)
- Never migrate the API query parameters of 7.0.0 and earlier; refuse such settings until the user writes an API field again. (t-1233)
