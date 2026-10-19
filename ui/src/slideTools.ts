// Règles des outils de slide de la barre d'outils : ajout d'éléments, suppression.

import type { AddWidget } from "./api";
import { type BaseName, baseName } from "./baseKinds";

/** Widgets de `base` de la grille d'ajout, par nom, dans l'ordre des cartes. */
export const ADD_NAMES = ["text", "card", "shape", "chart", "table"] as const satisfies readonly BaseName[];
/** Le nom d'une carte native de la grille d'ajout. */
export type AddName = (typeof ADD_NAMES)[number];

/** Widgets du pack de la grille d'ajout : sans aucun widget de `base` (cartes natives). */
export function packOnlyWidgets(list: AddWidget[]): AddWidget[] {
  return list.filter((w) => baseName(w.name) === null);
}

/**
 * Un élément s'ajoute (ou se colle) sur une slide de contenu ; sur une vide,
 * tout sauf le texte. `name` : la carte native, ou `widget` pour tout autre élément.
 */
export function canAdd(name: AddName | "widget", slide: string | undefined): boolean {
  return slide === "content" || (slide === "blank" && name !== "text");
}

/** Un deck (ou un jeu de modèles) garde toujours au moins une slide. */
export function canRemoveSlide(count: number): boolean {
  return count >= 2;
}
