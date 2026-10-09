// La fenêtre de l'agent : laquelle elle est, et quel onglet elle suit.

import { inTauri, type WorkspaceView } from "../api";
import { AGENT_LABEL } from "./link";

/** L'onglet actif de la fenêtre au premier plan ; null si elle est inconnue. */
export function agentTarget(view: WorkspaceView): string | null {
  return view.windows.find((w) => w.label === view.focused)?.active ?? null;
}

/** Cette page est-elle la fenêtre de l'agent ? Synchrone : label Tauri, ou `?window=agent`. */
export function isAgentWindow(): boolean {
  if (!inTauri) return new URLSearchParams(window.location.search).get("window") === AGENT_LABEL;
  const internals = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } }).__TAURI_INTERNALS__;
  return internals?.metadata?.currentWindow?.label === AGENT_LABEL;
}
