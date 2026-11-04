// Hands-free on the page side: report what the window shows, ask listening back at mount.
//
// Mounted once per workspace window (`Workspace.tsx`). Only that window reports its active
// tab, the one it holds, and the host checks it again (`ReportFrom`). The agent window never
// reports a context; it only asks listening back (`useListenAtMount`). No utterance text
// reaches the page: the host routes each one itself.

import { useEffect } from "react";
import type { AgentUi } from "./api";
import { agentApi, errorText } from "./api";
import { debounced, REAL_TIMERS, type Timers } from "./debounce";
import { voiceContext, type VoiceContextStore } from "./voiceContextStore";
import { stt } from "./voiceApi";

/** Wait after the last change before a context goes (ms): a burst of changes sends one. */
export const CONTEXT_DEBOUNCE_MS = 150;

/** Sends one context report of `tab` to the host. */
export type ReportContext = (tab: string, page: string, ui: AgentUi, seq: number, hasFocus: boolean, window: string) => Promise<void>;

/** What the hook uses beyond the page: injectable for tests. */
export interface VoiceContextDeps {
  store?: VoiceContextStore;
  timers?: Timers;
  report?: ReportContext;
}

const sendContext: ReportContext = (tab, page, ui, seq, hasFocus, window) => agentApi(tab).context(page, ui, seq, hasFocus, window);

/**
 * Asks hands-free listening back at mount when the switch is on, the user resumed it in this
 * page and the host is idle; it never stops listening, even at unmount (StrictMode remounts).
 */
export function useListenAtMount(store: VoiceContextStore = voiceContext): void {
  // The resume flag is per page: this fires only on a remount after a resume in the same page lifetime.
  useEffect(() => {
    if (!store.resumed()) return;
    let live = true;
    stt
      .status()
      .then((s) => (live && s.config.hands_free && s.state === "idle" ? stt.listen([]) : undefined))
      .catch((e) => console.warn("hands-free listening", errorText(e)));
    return () => {
      live = false;
    };
  }, [store]);
}

/**
 * Reports what window `window` shows of its active tab `tab`: at once, then after each change
 * of selection, editing, slide or focus, debounced by [`CONTEXT_DEBOUNCE_MS`].
 */
export function useVoiceContext(tab: string | null, window: string | null, deps: VoiceContextDeps = {}): void {
  const { store = voiceContext, timers = REAL_TIMERS, report = sendContext } = deps;
  useListenAtMount(store);
  useEffect(() => {
    if (!tab || !window) return;
    let last: string | null = null;
    const send = debounced(
      (ui: AgentUi, hasFocus: boolean) =>
        void report(tab, store.page, ui, store.nextSeq(), hasFocus, window).catch((e) => console.warn("page context", errorText(e))),
      CONTEXT_DEBOUNCE_MS,
      timers,
    );
    const check = () => {
      const ctx = store.read(tab);
      if (!ctx) return;
      const key = JSON.stringify(ctx);
      if (key === last) return;
      last = key;
      send(ctx.ui, ctx.hasFocus);
    };
    check();
    const unsubscribe = store.subscribe(check);
    return () => {
      unsubscribe();
      send.cancel();
    };
  }, [tab, window, store, timers, report]);
}

/** `App`'s side: records what tab `tab` shows and whether the page has the focus. */
export function useContextWriter(tab: string, ui: AgentUi, store: VoiceContextStore = voiceContext): void {
  useEffect(() => {
    store.write(tab, { ui, hasFocus: document.hasFocus() });
  }, [tab, ui, store]);
  useEffect(() => {
    const onFocus = () => {
      const ctx = store.read(tab);
      if (ctx) store.write(tab, { ...ctx, hasFocus: document.hasFocus() });
    };
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onFocus);
    };
  }, [tab, store]);
}
