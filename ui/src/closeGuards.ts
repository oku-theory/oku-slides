// Close guards: the work a tab must finish before it closes (typed widget writes, the autosave).
//
// A tab's App registers its guard while mounted; the close flows run it, bounded, before reading
// whether the tab is dirty. An App that unmounted (its tab no longer shown) leaves its in-flight
// work as the entry until that work settles, so a close right after still waits for it.

import { useEffect, useRef } from "react";

/** How long a close waits for a tab's guard, in milliseconds. */
const CLOSE_GUARD_MS = 3000;

/** Each tab's guard: the mounted App's, or an unmounted App's pending work. */
const guards = new Map<string, () => Promise<void>>();

/**
 * Registers `guard` as tab `tab`'s close guard while mounted.
 *
 * The guard is read from a ref updated every render, so only a change of `tab` re-registers it.
 * A registration first joins the work already there (a previous mount's pending writes); the
 * unmount joins the in-flight work once (the editor hooks' own cleanups started it) and leaves it
 * as the entry until it settles. Every removal is identity-checked: a StrictMode remount, or a
 * stale unregister, keeps the live guard.
 */
export function useCloseGuard(tab: string, guard: () => Promise<void>): void {
  const ref = useRef(guard);
  ref.current = guard;
  useEffect(() => {
    const prior = guards.get(tab);
    // Unbounded here: runCloseGuard's deadline bounds the whole chain when a close asks.
    const priorDone = prior ? prior().catch(() => {}) : Promise.resolve();
    const live = async () => {
      await priorDone;
      await ref.current();
    };
    guards.set(tab, live);
    return () => {
      const pending = Promise.all([priorDone, ref.current()]).then(
        () => {},
        () => {},
      );
      if (guards.get(tab) !== live) return;
      const left = () => pending;
      guards.set(tab, left);
      void pending.then(() => {
        if (guards.get(tab) === left) guards.delete(tab);
      });
    };
  }, [tab]);
}

/** Runs the tab's guard for at most `ms`; "done" without one. */
export async function runCloseGuard(tab: string, ms = CLOSE_GUARD_MS): Promise<"done" | "timeout"> {
  const guard = guards.get(tab);
  if (!guard) return "done";
  let timer: ReturnType<typeof setTimeout> | undefined;
  const deadline = new Promise<"timeout">((resolve) => {
    timer = setTimeout(() => resolve("timeout"), ms);
  });
  try {
    const answer = await Promise.race([guard().then(() => "done" as const), deadline]);
    if (answer === "timeout") console.warn(`close guard of tab ${tab}: still pending after ${ms} ms`);
    return answer;
  } finally {
    clearTimeout(timer);
  }
}
