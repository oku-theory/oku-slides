// Answering the host's quit request (t-1377): this window's close guards, then quit_ready.
//
// The host emits `quit-requested { ms }` to every window and exits once each answered or its own
// bound passed. A workspace window runs every registered close guard (runAllCloseGuards), the
// detached agent window flushes its draft; quitAnswerer bounds either by the host's `ms`.

import { listenTauri } from "./hostCall";

/**
 * Builds the answer to a quit request: `work(ms)`, then `ready()`, whatever `work` did.
 *
 * The work is bounded by `ms` here, so a hanging guard or flush never holds the answer back. A
 * request arriving while one runs joins it (the answer goes once); a later one runs again, so a
 * stray `quit-requested` (the host ignores answers outside a quit) cannot disarm the real quit.
 * Never rejects: a failed work or answer is logged, the host's own bound ends the quit anyway.
 */
export function quitAnswerer(work: (ms: number) => Promise<void>, ready: () => Promise<void>): (ms: number) => Promise<void> {
  let running: Promise<void> | null = null;
  return (ms) => {
    running ??= (async () => {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const deadline = new Promise<"timeout">((resolve) => (timer = setTimeout(() => resolve("timeout"), ms)));
      try {
        const done = await Promise.race([work(ms).then(() => "done" as const), deadline]);
        if (done === "timeout") console.warn(`quit: close guards still pending after ${ms} ms`);
      } catch (e) {
        console.warn("quit: close guards", String(e));
      } finally {
        clearTimeout(timer);
      }
      try {
        await ready();
      } catch (e) {
        console.warn("quit: answer", String(e));
      }
    })().finally(() => (running = null));
    return running;
  };
}

/** Listens for this window's quit requests and answers each with `work`; the effect's cleanup. */
export function listenQuit(work: (ms: number) => Promise<void>): () => void {
  return listenTauri(async () => {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const { invoke } = await import("@tauri-apps/api/core");
    const answer = quitAnswerer(work, () => invoke<void>("quit_ready"));
    return getCurrentWebviewWindow().listen<{ ms: number }>("quit-requested", (e) => void answer(e.payload.ms));
  });
}
