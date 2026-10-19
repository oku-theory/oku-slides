# src-tauri — agent rules

## Invariants

- Exit only via state::shutdown, or for an update state::settle then request_restart; run settle off the main thread, except the RunEvent::Exit re-call. (spec:2026-09-26-onglets-fenetres-design, spec:2026-10-18-auto-update-design)
- Never grant webviews core:window:allow-destroy; only the host closes windows, and new windows stay w-n. (plan:2026-09-26-onglets-fenetres)
- In windows sync, never destroy the agent window, and reconcile it before focusing the active workspace window. (plan:2026-10-09-agent-detachable)
- Never let the agent window change the focused workspace window, and never treat it as a tab drop target. (spec:2026-10-09-agent-detachable-design)
- Make the agent menu checkbox toggle the detached state read from the workspace, not the checkbox state. (plan:2026-10-09-agent-detachable)
- Throttle saves of window state and ignore windows that are already closed. (t-169)
- Install an update only after state::settle in the same blocking job, then request_restart; never restart from the webview. (spec:2026-10-18-auto-update-design)
- Grant no updater or opener permission in any capability, and refuse update commands from the agent window. (spec:2026-10-18-auto-update-design)

## Pitfalls

- Never block the Tauri main thread; run work through commands::blocking or spawn_blocking, or use try_lock. (plan:2026-09-26-onglets-fenetres)
- Register the single-instance plugin first, since the second instance stops inside its setup. (plan:2026-09-26-onglets-fenetres)
- Load tauri-plugin-single-instance on Linux only when a D-Bus session bus exists; it panics otherwise. (spec:2026-09-26-onglets-fenetres-design)
- Read the tab drop cursor on the host via tao, never from webview screenX or screenY. (spec:2026-09-26-onglets-fenetres-design, t-165)
- Expect no cursor or window positions on Wayland; a tab dropped there must open a new window. (t-160, t-179)
- Structure-changing workspace requests return the synced view, not their reply; query via read-only requests. (plan:2026-09-27-sources-loro-fusion)
- The updater installs ring as the global rustls provider on first check; keep every rustls user on ring. (spec:2026-10-18-auto-update-design)
- Never change plugins.updater.pubkey without the key rotation plan (t-978); installed apps reject bundles signed by another key. (spec:2026-10-18-auto-update-design)
- When adding a release platform, add its updater key to the release.yml matrix and to the publish gate's list, or no release goes latest. (spec:2026-10-18-auto-update-design, t-973)

## Testing

- Keep window open or close decisions in pure functions tested without Tauri. (spec:2026-10-09-agent-detachable-design)
- Keep testable logic in the session Workspace, since just test excludes the src-tauri app crate. (plan:2026-09-26-onglets-fenetres)

## Settled decisions

- Set bundle.createUpdaterArtifacts only in release.yml via --config, never in tauri.conf.json, so just build needs no signing key. (spec:2026-10-18-auto-update-design, t-973)
