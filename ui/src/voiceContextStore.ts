// What this window's page shows, per tab, for hands-free routing: one store per window.
//
// `App` writes the `AgentUi` it builds and `document.hasFocus()`; `useVoiceContext` reads the
// active tab's entry and reports it to the host. The store also numbers the reports (one page
// id per page, a seq never reused) and keeps whether the user resumed hands-free listening in
// this page. It holds no utterance text: the page never sees any.

import type { AgentUi } from "./api";
import { randomId } from "./randomId";

/** What the page shows of one tab, and whether the page has the focus. */
export interface PageContext {
  ui: AgentUi;
  hasFocus: boolean;
}

/** One window's contexts, report numbering and resume flag. */
export interface VoiceContextStore {
  /** This page's id: a reload is a new page, whose numbering starts over on the host. */
  readonly page: string;
  /** Records what the page shows of `tab`; subscribers hear only a real change. */
  write(tab: string, ctx: PageContext): void;
  /** What the page last showed of `tab`, if anything. */
  read(tab: string): PageContext | null;
  /** Calls `l` after each change; returns the unsubscribe. */
  subscribe(l: () => void): () => void;
  /** The next report number of this page: never reused, even across remounts. */
  nextSeq(): number;
  /** The user resumed hands-free listening in this page (and has not paused it since). */
  resumed(): boolean;
  /** Records the user's resume (`true`) or pause (`false`). */
  setResumed(on: boolean): void;
}

/** A fresh store, with its own page id. */
export function createVoiceContextStore(): VoiceContextStore {
  const page = randomId("p");
  const contexts = new Map<string, PageContext>();
  const listeners = new Set<() => void>();
  let seq = 0;
  let resumed = false;
  return {
    page,
    write(tab, ctx) {
      const before = contexts.get(tab);
      if (before && JSON.stringify(before) === JSON.stringify(ctx)) return;
      contexts.set(tab, ctx);
      for (const l of listeners) l();
    },
    read: (tab) => contexts.get(tab) ?? null,
    subscribe(l) {
      listeners.add(l);
      return () => void listeners.delete(l);
    },
    nextSeq: () => ++seq,
    resumed: () => resumed,
    setResumed(on) {
      resumed = on;
    },
  };
}

/** This window's store. */
export const voiceContext: VoiceContextStore = createVoiceContextStore();
