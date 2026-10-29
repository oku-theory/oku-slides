// The "Open" buttons of the deck's packs panel: the interface opens the tab itself.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "./api";
import { HOST_PACKS } from "./agent/surfaces";

/** What a packs panel action asks to open: a theme or a pack, by its id. */
export interface PanelOpen {
  kind: "theme" | "pack";
  id: string;
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const KEY = { open_theme: ["theme", "theme"], open_pack: ["pack", "pack"] } as const;

/**
 * The tab an action asks to open: `open_theme` or `open_pack` on the packs panel, carrying a UUID;
 * null for any other action, which goes on to the agent.
 */
export function panelOpen(s: Surface, a: A2uiClientAction): PanelOpen | null {
  if (s.id !== HOST_PACKS || !Object.hasOwn(KEY, a.name)) return null;
  const [kind, key] = KEY[a.name as keyof typeof KEY];
  const context = a.context;
  if (typeof context !== "object" || context === null || !Object.hasOwn(context, key)) return null;
  const id = (context as Record<string, unknown>)[key];
  return typeof id === "string" && UUID.test(id) ? { kind, id } : null;
}
