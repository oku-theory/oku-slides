// Élément neuf de la grille d'ajout : le départ que la session donne à chaque widget.

import type { Element } from "./api";
import type { ElementSeed } from "./packTypes";

/** Élément neuf du widget `kind` (`id` fourni), parti de son départ `seed`. */
export function newElement(kind: string, id: string, seed: ElementSeed): Element {
  return { id, kind, params: structuredClone(seed.params), ...(seed.style ? { style: structuredClone(seed.style) } : {}) };
}
