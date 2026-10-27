// A fake workspace for the close flow tests: every call noted in one log, answers set per test.

import { errorText, type OpenerOffer, type Snapshot, type TabApi, type TabDocument, type TabInfo, type WorkspaceView } from "../api";
import { type CloseFlowInput, closeFlowDeps } from "../closeFlow";
import type { PackMode, ThemePin, ThemeTabView } from "../packTypes";

/** A snapshot with what the close flows read. */
export const snap = (s: Partial<Snapshot> = {}) => ({ dirty: false, path: null, name: "essai", editing: null, history: null, ...s }) as Snapshot;

/** A pack tab's editing view: host tab, writable. */
export const packEditing = (o: Partial<PackMode> = {}) =>
  ({ pack: "p", name: "Acme", kind: "widgets", builtin: false, host: "tab", read_only: false, leased: true, display_name: "Acme", ...o }) as PackMode;

const DECK: TabDocument = { kind: "deck", path: null, deck_uid: null };

/** A tab as the bar shows it; `pack` makes its cached kind a pack. */
export const tabOf = (id: string, dirty = false, pack = false): TabInfo => ({
  id,
  title: id,
  dirty,
  pristine: false,
  editing_pack: pack,
  theme: null,
  document: pack ? { kind: "pack", pack: "p" } : DECK,
});

/** A theme tab as the bar shows it: its cached kind is a theme. */
export const themeTabOf = (id: string, dirty = false): TabInfo => ({ ...tabOf(id, dirty), document: { kind: "theme", theme: "t" } });

/** A theme tab's view: writable, leased. */
export const themeTabView = (o: Partial<ThemeTabView> = {}) =>
  ({ read_only: false, leased: true, display_name: "Maison", session_version: null, ...o }) as ThemeTabView;

/** The offer of the opener `t-1` for version `v2` of pack `p`. */
export const OFFER: OpenerOffer = { tab: "t-1", title: "Roadmap", deck_uid: "d1", pack: "p", pack_name: "Acme", version: "v2" };

/** The clock's start, in milliseconds. */
export const T0 = 1_000_000;

/** The workspace view the fakes answer. */
const VIEW = { generation: 1, focused: "main", agent_detached: false, import_pick: null, windows: [] } as unknown as WorkspaceView;

/** A fake workspace: `state` holds the answers, `log` every call, `input` builds deps. */
export function fake(tabs: TabInfo[]) {
  const log: string[] = [];
  const guardCalls = new Map<string, number>();
  const state = {
    tabs,
    /** The guard of `id` at its `n`-th call. */
    guard: (_id: string, _n: number): Promise<"done" | "timeout"> => Promise.resolve("done"),
    /** A deck tab as dirty as the bar says. */
    snapshot: (id: string): Promise<Snapshot> => Promise.resolve(snap({ dirty: !!state.tabs.find((t) => t.id === id)?.dirty, path: "/x.oku-slides.pdf" })),
    savePack: (_id: string): Promise<Snapshot> => Promise.resolve(snap()),
    saveTheme: (_id: string): Promise<ThemePin | null> => Promise.resolve(null),
    /** A save cleans the tab. */
    save: (id: string): Promise<Snapshot> => {
      state.tabs = state.tabs.map((t) => (t.id === id ? { ...t, dirty: false } : t));
      return Promise.resolve(snap({ path: "/x.oku-slides.pdf" }));
    },
    take: (): Promise<Snapshot> => Promise.resolve(snap()),
    offer: null as OpenerOffer | null | Error,
    /** Confirm answers, in order; true once spent. */
    answers: [] as boolean[],
    /** `"<id> <discard>"` closes the host refuses. */
    refused: new Set<string>(),
    clock: T0,
    /** Runs inside closeWindow, before it answers. */
    duringCloseWindow: async () => {},
  };
  const api = (id: string) =>
    ({
      snapshot: () => (log.push(`snapshot ${id}`), state.snapshot(id)),
      savePack: () => (log.push(`savePack ${id}`), state.savePack(id)),
      saveTheme: () => (log.push(`saveTheme ${id}`), state.saveTheme(id)),
      save: () => (log.push(`save ${id}`), state.save(id)),
      closePack: () => (log.push(`closePack ${id}`), Promise.resolve(snap())),
      historyClose: () => (log.push(`historyClose ${id}`), Promise.resolve(snap())),
      takePackVersion: (pack: string, version: string, deckUid: string) => (log.push(`take ${id} ${pack} ${version} ${deckUid}`), state.take()),
    }) as unknown as TabApi;
  const input: CloseFlowInput = {
    tabApi: api,
    confirm: (m) => (log.push(`confirm ${m}`), state.answers.shift() ?? true),
    runGuard: (id) => {
      const n = (guardCalls.get(id) ?? 0) + 1;
      guardCalls.set(id, n);
      log.push(`guard ${id}`);
      return state.guard(id, n);
    },
    closing: new Set<string>(),
    stalls: new Map<string, number>(),
    refresh: () => log.push("refresh"),
    freshTabs: async () => (log.push("fresh"), state.tabs),
    now: () => state.clock,
    ws: {
      close: async (id: string, discard = false) => {
        log.push(`close ${id} ${discard}`);
        if (state.refused.has(`${id} ${discard}`)) throw new Error("refusé");
        state.tabs = state.tabs.filter((t) => t.id !== id);
        return VIEW;
      },
      closeWindow: async (_label: string, discard: string[]) => {
        log.push(`closeWindow ${discard.join(",")}`);
        await state.duringCloseWindow();
        return VIEW;
      },
      packOpener: async (id: string) => {
        log.push(`opener ${id}`);
        if (state.offer instanceof Error) throw state.offer;
        return state.offer;
      },
      activate: async (id: string) => (log.push(`activate ${id}`), VIEW),
    },
    label: "main",
    alive: { current: true },
    run: async (p) => {
      try {
        return await p;
      } catch (e) {
        log.push(`run error ${errorText(e)}`);
        return null;
      }
    },
    setError: (e) => log.push(`report ${e}`),
  };
  return { log, state, input, deps: () => closeFlowDeps(input), guardCalls };
}

/** A promise resolved from outside. */
export function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((ok) => {
    resolve = ok;
  });
  return { promise, resolve };
}
