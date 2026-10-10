// Une valeur passagère : montrée pour un délai, puis effacée d'elle-même.

import { useCallback, useEffect, useRef, useState } from "react";
import { REAL_TIMERS, type Timers } from "./debounce";

/** Une valeur effacée après son délai ; `show(null)` l'efface tout de suite. */
export function useTransient<T>(timers: Timers = REAL_TIMERS): [T | null, (value: T | null, ms?: number) => void] {
  const [value, setValue] = useState<T | null>(null);
  const handle = useRef<unknown>(null);
  useEffect(() => () => void (handle.current !== null && timers.clear(handle.current)), [timers]);
  const show = useCallback(
    (next: T | null, ms = 0) => {
      // Une nouvelle valeur remplace la précédente : son minuteur ne doit pas l'effacer en avance.
      if (handle.current !== null) timers.clear(handle.current);
      handle.current = null;
      setValue(next);
      if (next === null) return;
      handle.current = timers.set(() => setValue((cur) => (cur === next ? null : cur)), ms);
    },
    [timers],
  );
  return [value, show];
}
