// Règles des outils de slide de la barre d'outils : ajout d'éléments, suppression.

import type { SlideKind } from "./api";

/** Éléments qu'on ajoute depuis la barre d'outils, dans l'ordre des boutons. */
export const ADD_KINDS = ["text", "card", "shape", "chart"] as const;
export type AddKind = (typeof ADD_KINDS)[number];

/** Un élément s'ajoute sur une slide de contenu ; sur une vide, tout sauf le texte. */
export function canAdd(kind: AddKind, slide: SlideKind | undefined): boolean {
  return slide === "content" || (slide === "blank" && kind !== "text");
}

/** Un deck (ou un jeu de modèles) garde toujours au moins une slide. */
export function canRemoveSlide(count: number): boolean {
  return count >= 2;
}
