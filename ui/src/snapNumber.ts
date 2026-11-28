// A number per snapshot, so that a surface can be reread on each new one (an edit, an undo, the
// agent, a query landing), even when no slide's render changes.

import type { Snapshot } from "./api";

const snapNumbers = new WeakMap<Snapshot, number>();
let lastNumber = 0;

/** The number of `snap`, the same for the same snapshot; 0 without one. */
export const snapNumber = (snap: Snapshot | null): number => {
  if (!snap) return 0;
  const known = snapNumbers.get(snap);
  if (known !== undefined) return known;
  snapNumbers.set(snap, ++lastNumber);
  return lastNumber;
};
