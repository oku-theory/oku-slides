// Minuteries à la main pour les tests : rien ne part avant `run()`.

import type { Timers } from "../debounce";

/** Minuteries à la main : `run()` déclenche ce qui attend, `delays` garde les délais demandés. */
export function manualTimers(): Timers & { run: () => void; delays: number[] } {
  const waiting = new Map<number, () => void>();
  const delays: number[] = [];
  let n = 0;
  return {
    delays,
    set: (fn, ms) => {
      delays.push(ms);
      waiting.set(++n, fn);
      return n;
    },
    clear: (h) => void waiting.delete(h as number),
    run: () => {
      const now = [...waiting.values()];
      waiting.clear();
      now.forEach((fn) => fn());
    },
  };
}
