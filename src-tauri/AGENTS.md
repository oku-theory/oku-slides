# src-tauri — agent rules

## Invariants

- Exit only via state::shutdown, or for an update state::settle then request_restart; run settle off the main thread, except the RunEvent::Exit re-call. (spec:2026-09-26-onglets-fenetres-design, spec:2026-10-18-auto-update-design)
- Reach every exit but the empty view and RunEvent::Exit through quit::exit or quit::guarded, which wait for each window's quit_ready, keyed by the caller's label, off the main thread and before closing is set. (t-1377)
- Never grant webviews core:window:allow-destroy; only the host closes windows, and new windows stay w-n. (plan:2026-09-26-onglets-fenetres)
- In windows sync, never destroy the agent window, and reconcile it before focusing the active workspace window. (plan:2026-10-09-agent-detachable)
- Never let the agent window change the focused workspace window, and never treat it as a tab drop target. (spec:2026-10-09-agent-detachable-design)
- Pass every request and workspace command from the agent window through check_agent_request or check_agent_workspace before acting, in the app and serve alike; widen their allowlist only for what its page sends. (t-1469)
- Relay the agent link only through the bounded agent_link command; never grant the agent window core:default or an event emit permission, as build.rs checks. (t-1469)
- Make the agent menu checkbox toggle the detached state read from the workspace, not the checkbox state. (plan:2026-10-09-agent-detachable)
- Throttle saves of window state and ignore windows that are already closed. (t-169)
- Install an update only after state::settle in the same blocking job, then request_restart; never restart from the webview. (spec:2026-10-18-auto-update-design)
- Let the agent quit or install only through AppPort on the stack's after-turn thread, via quit::exit and updates::install, never inside a tool call; the quit tool asks AppPort::can_quit before confirming, and refuses while any tab is unsaved, telling a count, never titles. (t-999, t-1000, plan:2026-11-01-lot-m-hands-free)
- Grant no updater or opener permission in any capability; grant app commands per window in capabilities, the agent window never update, quit or tab drag ones, as build.rs checks against generate_handler! and its COMMANDS. (spec:2026-10-18-auto-update-design, t-980)
- Keep plugins.updater.requireSignedVersion true; every updater signature must carry version:<v> in its trusted comment. (t-1210)

## Pitfalls

- Never block the Tauri main thread; run work through commands::blocking or spawn_blocking, or use try_lock. (plan:2026-09-26-onglets-fenetres)
- Register the single-instance plugin first, since the second instance stops inside its setup. (plan:2026-09-26-onglets-fenetres)
- Load tauri-plugin-single-instance on Linux only when a D-Bus session bus exists; it panics otherwise. (spec:2026-09-26-onglets-fenetres-design)
- Read the tab drop cursor on the host via tao, never from webview screenX or screenY. (spec:2026-09-26-onglets-fenetres-design, t-165)
- Expect no cursor or window positions on Wayland; a tab dropped there must open a new window. (t-160, t-179)
- Structure-changing workspace requests return the synced view, not their reply; query via read-only requests. (plan:2026-09-27-sources-loro-fusion)
- The updater installs ring as the global rustls provider on first check; keep every rustls user on ring. (spec:2026-10-18-auto-update-design)
- Never change plugins.updater.pubkey outside the ADR-0014 procedures; installed apps reject bundles signed by another key. (ADR-0014, spec:2026-10-18-auto-update-design)
- Keep the update mirror first in updater endpoints; its host is also in MIRROR_DOWNLOAD_PREFIX and update-mirror.yml. (spec:2026-10-25-update-mirror-design, t-1209)
- Repair a failed mirror job with a manual Update mirror run; apps read the mirror first, so a stale one hides releases. (spec:2026-10-25-update-mirror-design, t-1209)
- When adding a release platform, add its updater key to the release.yml matrix, the publish job's count and PLATFORMS in release-gate.sh, or no release goes latest. (spec:2026-10-18-auto-update-design, t-973)

## Testing

- Keep window open or close decisions in pure functions tested without Tauri. (spec:2026-10-09-agent-detachable-design)
- Keep testable logic in the session Workspace, since just test excludes the src-tauri app crate. (plan:2026-09-26-onglets-fenetres)

## Settled decisions

- Set bundle.createUpdaterArtifacts only in release.yml via --config, never in tauri.conf.json, so just build needs no signing key. (spec:2026-10-18-auto-update-design, t-973)
- Sign update bundles only in release.yml's sign job, with scripts from main, never the tag; build jobs hold no signing secret. (ADR-0014, t-977)
