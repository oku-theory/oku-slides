// Le mode présentation : ses raccourcis d'entrée, et les touches pendant qu'il tourne.

import { slideTarget } from "./slideKeys";

type Keys = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

/** `screen` : la fenêtre passe en plein écran ; `window` : la fenêtre garde sa taille, sans interface. */
export type PresentMode = "screen" | "window";

/** Le mode qu'ouvre la touche : ⌘⏎ plein écran, ⌘⇧⏎ dans la fenêtre ; `null` sinon. */
export function presentShortcut(e: Keys): PresentMode | null {
  if (e.key !== "Enter" || !(e.metaKey || e.ctrlKey) || e.altKey) return null;
  return e.shiftKey ? "window" : "screen";
}

/** Ce que fait une touche pendant la présentation. */
export type PresentAction = { kind: "exit" } | { kind: "show"; index: number } | { kind: "stay" };

/**
 * L'action de la touche depuis la slide `index` sur `count` ; `null` : la touche n'est pas à elle.
 *
 * Échap sort ; ← → ↑ ↓ Page↑ Page↓ Espace Début Fin naviguent (`stay` au bord : la touche reste prise).
 */
export function presentKey(e: Keys, index: number, count: number): PresentAction | null {
  if (e.metaKey || e.ctrlKey || e.altKey) return null;
  if (e.key === "Escape") return { kind: "exit" };
  if (!NAV_KEYS.has(e.key)) return null;
  const to = slideTarget(e, index, count);
  return to === null ? { kind: "stay" } : { kind: "show", index: to };
}

/** Les touches de `slideTarget`, Espace compris. */
const NAV_KEYS = new Set(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "PageUp", "PageDown", " ", "Home", "End"]);
