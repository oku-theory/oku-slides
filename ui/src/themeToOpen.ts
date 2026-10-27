// The theme row's "Open" in the deck's packs panel: the interface opens the theme's tab itself.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "./api";
import { HOST_PACKS } from "./agent/surfaces";

/**
 * The theme id an action asks to open: `open_theme` on the packs panel, carrying a theme id;
 * null for any other action, which goes on to the agent.
 */
export function themeToOpen(s: Surface, a: A2uiClientAction): string | null {
  if (s.id !== HOST_PACKS || a.name !== "open_theme") return null;
  const theme = a.context?.theme;
  return typeof theme === "string" && theme !== "" ? theme : null;
}
