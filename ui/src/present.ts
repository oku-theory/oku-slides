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

/**
 * What the host asks of the presentation (`McpEvent::Present`, mirrored from the mcp crate):
 * start it, or steer the one running.
 */
export type PresentCommand =
  | { action: "start"; mode: PresentMode }
  | { action: "show"; slide: string }
  | { action: "next" }
  | { action: "previous" }
  | { action: "first" }
  | { action: "last" }
  | { action: "exit" };

/** A command that steers a running presentation. */
export type SlideCommand = Exclude<PresentCommand, { action: "start" }>;

/** Subscribes to the steering commands; returns the unsubscribe. */
export type SubscribeSlideCommands = (l: (c: SlideCommand) => void) => () => void;

/**
 * What `c` does from slide `index` of `slides`: `stay` at an edge, and for a slide id missing
 * from the snapshot (the host checked it against a newer deck, or an older one).
 */
export function commandAction(c: SlideCommand, index: number, slides: readonly { id: string }[]): PresentAction {
  const last = slides.length - 1;
  const to = (i: number): PresentAction => (i === index || i < 0 || i > last ? { kind: "stay" } : { kind: "show", index: i });
  switch (c.action) {
    case "exit":
      return { kind: "exit" };
    case "next":
      return to(index + 1);
    case "previous":
      return to(index - 1);
    case "first":
      return to(0);
    case "last":
      return to(last);
    case "show":
      return to(slides.findIndex((s) => s.id === c.slide));
  }
}

/** Les touches de `slideTarget`, Espace compris. */
const NAV_KEYS = new Set(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "PageUp", "PageDown", " ", "Home", "End"]);

/** True during a presentation: Escape belongs to it, no modal or question takes it. */
export const presenting = (): boolean => document.querySelector(".presenter") !== null;
