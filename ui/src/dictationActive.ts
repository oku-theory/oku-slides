// How many push-to-talk dictations are active (recording or downloading), outside App state.

import { useLayoutEffect } from "react";

let value = 0;
const change = (by: 1 | -1) => {
  value += by;
};

/** The number of active push-to-talk dictations: above zero, Escape cancels one first. */
export function dictationActive(): number {
  return value;
}

/** Zeroes the counter (tests). */
export function resetDictationActive(): void {
  value = 0;
}

/** Counts this component as an active dictation while `on`; the cleanup undoes it, StrictMode included. */
export function useDictationActive(on: boolean): void {
  // A layout effect, so the count follows the status before the next Escape can arrive.
  useLayoutEffect(() => {
    if (!on) return;
    change(1);
    return () => change(-1);
  }, [on]);
}
