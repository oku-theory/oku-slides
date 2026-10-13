# src-tauri — agent rules

## Invariants

- Exit only via state::shutdown; call Workspace::settle on the main thread only at RunEvent::Exit. (spec:2026-09-26-onglets-fenetres-design)
- Never grant webviews core:window:allow-destroy; only the host closes windows, and new windows stay w-n. (plan:2026-09-26-onglets-fenetres)
- In windows sync, never destroy the agent window, and reconcile it before focusing the active workspace window. (plan:2026-10-09-agent-detachable)
- Never let the agent window change the focused workspace window, and never treat it as a tab drop target. (spec:2026-10-09-agent-detachable-design)
- Make the agent menu checkbox toggle the detached state read from the workspace, not the checkbox state. (plan:2026-10-09-agent-detachable)
- Throttle saves of window state and ignore windows that are already closed. (t-169)

## Pitfalls

- Never block the Tauri main thread; run work through commands::blocking or spawn_blocking, or use try_lock. (plan:2026-09-26-onglets-fenetres)
- Register the single-instance plugin first, since the second instance stops inside its setup. (plan:2026-09-26-onglets-fenetres)
- Load tauri-plugin-single-instance on Linux only when a D-Bus session bus exists; it panics otherwise. (spec:2026-09-26-onglets-fenetres-design)
- Read the tab drop cursor on the host via tao, never from webview screenX or screenY. (spec:2026-09-26-onglets-fenetres-design, t-165)
- Expect no cursor or window positions on Wayland; a tab dropped there must open a new window. (t-160, t-179)
- Structure-changing workspace requests return the synced view, not their reply; query via read-only requests. (plan:2026-09-27-sources-loro-fusion)

## Testing

- Keep window open or close decisions in pure functions tested without Tauri. (spec:2026-10-09-agent-detachable-design)
- Keep testable logic in the session Workspace, since just test excludes the src-tauri app crate. (plan:2026-09-26-onglets-fenetres)
