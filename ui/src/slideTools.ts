// Règles des outils de slide de la barre d'outils : ajout d'éléments, suppression.

import type { AddWidget } from "./api";

/** Éléments qu'on ajoute depuis la barre d'outils, dans l'ordre des boutons. */
export const ADD_KINDS = ["text", "card", "shape", "chart", "table"] as const;
export type AddKind = (typeof ADD_KINDS)[number];

/** Widgets du pack du menu « Widget » : sans ceux qui ont déjà leur bouton. */
export function menuWidgets(list: AddWidget[]): AddWidget[] {
  return list.filter((w) => !(ADD_KINDS as readonly string[]).includes(w.name));
}

/**
 * Un élément s'ajoute (ou se colle) sur une slide de contenu ; sur une vide,
 * tout sauf le texte.
 */
export function canAdd(kind: string, slide: string | undefined): boolean {
  return slide === "content" || (slide === "blank" && kind !== "text");
}

/** Un deck (ou un jeu de modèles) garde toujours au moins une slide. */
export function canRemoveSlide(count: number): boolean {
  return count >= 2;
}
