// Règles des outils de slide de la barre d'outils : ajout d'éléments, suppression.

/** Les slots libres des layouts épinglés (`Snapshot.free_slots`) : kinds acceptés, `null` pour tous. */
export type FreeSlots = Record<string, string[] | null>;

/** La slide de kind `slideKind` a un slot libre : on peut y ajouter un élément. */
export function acceptsElements(slideKind: string | undefined, freeSlots: FreeSlots): boolean {
  return slideKind !== undefined && Object.hasOwn(freeSlots, slideKind);
}

/** Un élément de kind `kind` s'ajoute (ou se colle) dans le premier slot libre de la slide. */
export function canAdd(slideKind: string | undefined, kind: string, freeSlots: FreeSlots): boolean {
  if (slideKind === undefined || !acceptsElements(slideKind, freeSlots)) return false;
  const accepts = freeSlots[slideKind];
  return accepts === null || accepts.includes(kind);
}

/** Un deck (ou un pack de layouts) garde toujours au moins une slide. */
export function canRemoveSlide(count: number): boolean {
  return count >= 2;
}
