/** Appel retardé : seul le dernier d'une rafale part, `ms` après lui. */

/** Minuteries injectables (tests). */
export interface Timers {
  set(fn: () => void, ms: number): unknown;
  clear(handle: unknown): void;
}

export const REAL_TIMERS: Timers = {
  set: (fn, ms) => setTimeout(fn, ms),
  clear: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
};

/** Délai de l'aperçu d'un widget après la dernière frappe. */
export const PREVIEW_DELAY_MS = 400;

/** `fn` retardé de `ms` ; `.cancel()` oublie l'appel en attente, `.flush()` l'envoie. */
export function debounced<A extends unknown[]>(fn: (...args: A) => void, ms: number, timers: Timers = REAL_TIMERS) {
  let handle: unknown = null;
  let waiting: A | null = null;
  const call = (...args: A) => {
    if (handle !== null) timers.clear(handle);
    waiting = args;
    handle = timers.set(() => {
      handle = null;
      const now = waiting;
      waiting = null;
      if (now) fn(...now);
    }, ms);
  };
  call.cancel = () => {
    if (handle !== null) timers.clear(handle);
    handle = null;
    waiting = null;
  };
  call.flush = () => {
    const now = waiting;
    call.cancel();
    if (now) fn(...now);
  };
  return call;
}
