// Portée des raccourcis d'App : tout, les seuls raccourcis d'affichage, ou rien.

import { targetOwnsKeys } from "./agent/keys";
import { viewToggle } from "./useKeptUi";

/**
 * Ce qui retient les touches d'App.
 *
 * Fenêtre de l'agent, fenêtre de l'app (layouts, import), édition d'un texte.
 */
export interface KeyHolds {
  modal: boolean;
  dialog: boolean;
  editing: boolean;
}

/** Touche telle qu'App la lit. */
export type ScopedKey = Parameters<typeof viewToggle>[0] & { target: EventTarget | null };

/**
 * Ce qu'App peut faire d'une touche : `all`, `display` (⌘' et ⌘⇧G seulement) ou `none`.
 * Une saisie de texte (éditeur CodeMirror compris) garde ses touches, sauf ces deux raccourcis
 * d'affichage, qu'aucun éditeur ne lie ; une fenêtre de l'agent ou de l'app (choix d'un layout, import) les garde toutes.
 */
export function appKeyScope(holds: KeyHolds, e: ScopedKey): "all" | "display" | "none" {
  if (holds.modal || holds.dialog) return "none";
  const target = e.target instanceof HTMLElement ? e.target : null;
  if (holds.editing || targetOwnsKeys(target, e)) return viewToggle(e) ? "display" : "none";
  return "all";
}

/** A key as `themeTabIgnores` reads it. */
export type ThemeTabKey = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "preventDefault">;

/**
 * True for a key a theme tab ignores: ⌘S and ⌘⇧S (the draft saves itself, no copy) and the
 * slide and element keys (⌥↑ ⌥↓ move, Delete, Backspace, Enter); the specimens stay browsable.
 * The save keys are also kept from the browser, whose "Save as" would open otherwise.
 */
export function themeTabIgnores(e: ThemeTabKey): boolean {
  const mod = e.metaKey || e.ctrlKey;
  if (mod && e.key.toLowerCase() === "s") {
    e.preventDefault();
    return true;
  }
  if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) return true;
  return !mod && (e.key === "Delete" || e.key === "Backspace" || e.key === "Enter");
}
