// Portée des raccourcis d'App : tout, les seuls raccourcis d'affichage, ou rien.

import { targetOwnsKeys } from "./agent/keys";
import { viewToggle } from "./useKeptUi";

/** Ce qui retient les touches d'App : fenêtre de l'agent, choix d'un modèle, édition d'un texte. */
export interface KeyHolds {
  modal: boolean;
  picking: boolean;
  editing: boolean;
}

/** Touche telle qu'App la lit. */
export type ScopedKey = Parameters<typeof viewToggle>[0] & { target: EventTarget | null };

/**
 * Ce qu'App peut faire d'une touche : `all`, `display` (⌘' et ⌘⇧G seulement) ou `none`.
 * Une saisie de texte (éditeur CodeMirror compris) garde ses touches, sauf ces deux raccourcis
 * d'affichage, qu'aucun éditeur ne lie ; une fenêtre de l'agent ou le choix d'un modèle les garde toutes.
 */
export function appKeyScope(holds: KeyHolds, e: ScopedKey): "all" | "display" | "none" {
  if (holds.modal || holds.picking) return "none";
  const target = e.target instanceof HTMLElement ? e.target : null;
  if (holds.editing || targetOwnsKeys(target, e)) return viewToggle(e) ? "display" : "none";
  return "all";
}
