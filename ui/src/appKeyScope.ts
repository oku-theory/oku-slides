// Portée des raccourcis d'App : tout, les seuls raccourcis d'affichage, ou rien.

import { targetOwnsKeys } from "./agent/keys";
import { viewToggle } from "./useKeptUi";

/**
 * Ce qui retient les touches d'App.
 *
 * Fenêtre de l'agent, fenêtre de l'app (modèles, import), édition d'un texte.
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
 * d'affichage, qu'aucun éditeur ne lie ; une fenêtre de l'agent ou de l'app (choix d'un modèle, import) les garde toutes.
 */
export function appKeyScope(holds: KeyHolds, e: ScopedKey): "all" | "display" | "none" {
  if (holds.modal || holds.dialog) return "none";
  const target = e.target instanceof HTMLElement ? e.target : null;
  if (holds.editing || targetOwnsKeys(target, e)) return viewToggle(e) ? "display" : "none";
  return "all";
}
