// La fenêtre de l'agent : laquelle elle est, et quel onglet elle suit.

import { inTauri, sessionTab, type WorkspaceView } from "../api";
import { AGENT_LABEL } from "./link";

/**
 * The session tab of the focused window's active tab (a knowledge tab's owner deck tab); null
 * when that window is unknown.
 */
export function agentTarget(view: WorkspaceView): string | null {
  const active = view.windows.find((w) => w.label === view.focused)?.active;
  return active === undefined ? null : sessionTab(view, active);
}

/** Cette page est-elle la fenêtre de l'agent ? Synchrone : label Tauri, ou `?window=agent`. */
export function isAgentWindow(): boolean {
  if (!inTauri) return new URLSearchParams(window.location.search).get("window") === AGENT_LABEL;
  const internals = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } }).__TAURI_INTERNALS__;
  return internals?.metadata?.currentWindow?.label === AGENT_LABEL;
}
