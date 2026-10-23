// Déplacement au clavier dans une grille de cartes, en sautant celles qu'on ne peut pas choisir.

/** Sens d'un déplacement dans la grille. */
export type GridKey = "left" | "right" | "up" | "down";

/**
 * Carte choisissable atteinte depuis `at` par `key`, ou `at` sinon.
 *
 * ← → vont à la carte voisine, d'une rangée à l'autre ; ↑ ↓ restent dans la colonne et ne
 * bougent pas si elle n'a plus de carte choisissable dans ce sens (rangée partielle comprise).
 * Sans carte au focus (`at` vaut -1) ou sans colonne, rien ne bouge.
 */
export function gridStep(at: number, key: GridKey, columns: number, count: number, pickable: (i: number) => boolean): number {
  if (at < 0 || columns < 1) return at;
  const step = { left: -1, right: 1, up: -columns, down: columns }[key];
  for (let i = at + step; i >= 0 && i < count; i += step) {
    if (pickable(i)) return i;
  }
  return at;
}

/** Nombre de cartes de la première rangée, d'après leurs hauteurs (au moins 1). */
export function rowLength(tops: number[]): number {
  return Math.max(1, tops.filter((top) => top === tops[0]).length);
}

/**
 * La grille vue par le clavier quand les cartes sont rangées en sections : chaque section
 * commence une rangée et sa dernière rangée est complétée par des trous (`null`). Chaque case
 * porte l'indice de sa carte dans la liste à plat, pour que ↑ ↓ suivent les colonnes à l'écran.
 */
export function sectionSlots(sizes: number[], columns: number): (number | null)[] {
  const width = Math.max(1, columns);
  const slots: (number | null)[] = [];
  let next = 0;
  for (const size of sizes) {
    for (let i = 0; i < size; i++) slots.push(next++);
    while (slots.length % width !== 0) slots.push(null);
  }
  return slots;
}
