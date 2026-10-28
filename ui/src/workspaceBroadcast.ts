// A view the host broadcasts: shown, then the active tab's snapshot read again.

import type { WorkspaceView } from "./workspaceApi";

/** What showing a broadcast needs from the workspace window. */
export interface BroadcastDeps {
  /** False once the window unmounted. */
  alive: { current: boolean };
  /** Shows the view, unless a newer one is shown. */
  setView: (view: WorkspaceView) => void;
  /** Makes the active App read its snapshot again. */
  bump: () => void;
}

/**
 * Shows a broadcast view, then always makes the active App re-read its snapshot.
 *
 * Always, not only when the active tab's TabInfo changed: a notice set on the active tab
 * changes no info, and an owner deck changed by its deck-pack tab's forward must redraw.
 */
export function showBroadcast(view: WorkspaceView, deps: BroadcastDeps): void {
  if (!deps.alive.current) return;
  deps.setView(view);
  deps.bump();
}
