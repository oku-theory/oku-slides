// A fake workspace for the close flow tests: every call noted in one log, answers set per test.

import { errorText, type OpenerOffer, type Snapshot, type TabApi, type TabDocument, type TabInfo, type WindowView, type WorkspaceView } from "../api";
import { type CloseFlowInput, closeFlowDeps } from "../closeFlow";
import type { PackMode, ThemePin, ThemeTabView } from "../packTypes";
import type { Question } from "../useConfirm";

/** A snapshot with what the close flows read. */
export const snap = (s: Partial<Snapshot> = {}) => ({ dirty: false, path: null, name: "essai", editing: null, history: null, ...s }) as Snapshot;

/** A pack tab's editing view: a library pack, writable. */
export const packEditing = (o: Partial<PackMode> = {}) =>
  ({ pack: "p", name: "Acme", kind: "widgets", builtin: false, deck_only: false, read_only: false, leased: true, display_name: "Acme", ...o }) as PackMode;

const DECK: TabDocument = { kind: "deck", path: null, deck_uid: null };

/** A tab as the bar shows it; `pack` makes its cached kind a pack. */
export const tabOf = (id: string, dirty = false, pack = false): TabInfo => ({
  id,
  title: id,
  dirty,
  pristine: false,
  theme: null,
  document: pack ? { kind: "pack", pack: "p" } : DECK,
});

/** A deck-pack tab of owner `owner`, as the bar shows it. */
export const linkedOf = (id: string, owner: string, dirty = false): TabInfo => ({
  ...tabOf(id, dirty),
  document: { kind: "deck_pack", deck_tab: owner, pack: "p" },
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

/** The view a workspace command answers; the flows read only `freshView`. */
const VIEW = { generation: 1, focused: "main", agent_detached: false, import_pick: null, windows: [] } as unknown as WorkspaceView;

/** A fake workspace: `state` holds the answers, `log` every call, `input` builds deps. */
export function fake(tabs: TabInfo[]) {
  const log: string[] = [];
  /** Every question asked, in order. */
  const questions: Question[] = [];
  const guardCalls = new Map<string, number>();
  const state = {
    tabs,
    /** Windows other than `main`, for linked tabs elsewhere. */
    otherWindows: [] as WindowView[],
    /** The guard of `id` at its `n`-th call. */
    guard: (_id: string, _n: number): Promise<"done" | "timeout"> => Promise.resolve("done"),
    /** A deck tab as dirty as the bar says. */
    snapshot: (id: string): Promise<Snapshot> => Promise.resolve(snap({ dirty: !!state.tabs.find((t) => t.id === id)?.dirty, path: "/x.oku-slides.pdf" })),
    /** A deck-pack tab's save is forwarded: its owner becomes dirty. */
    savePack: (id: string): Promise<Snapshot> => {
      const doc = allTabs().find((t) => t.id === id)?.document;
      if (doc?.kind === "deck_pack") state.tabs = state.tabs.map((t) => (t.id === doc.deck_tab ? { ...t, dirty: true } : t));
      return Promise.resolve(snap());
    },
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
  /** Every tab of every window. */
  const allTabs = () => [...state.tabs, ...state.otherWindows.flatMap((w) => w.tabs)];
  /** The view read now: `main` holds `state.tabs`, then the other windows. */
  const viewOf = (): WorkspaceView => ({
    ...VIEW,
    windows: [{ label: "main", active: state.tabs[0]?.id ?? "", tabs: state.tabs }, ...state.otherWindows],
  });
  /** Drops `id` and, for an owner, its deck-pack tabs, as the host closes them together. */
  const drop = (id: string) => {
    const kept = (t: TabInfo) => t.id !== id && !(t.document.kind === "deck_pack" && t.document.deck_tab === id);
    state.tabs = state.tabs.filter(kept);
    state.otherWindows = state.otherWindows.map((w) => ({ ...w, tabs: w.tabs.filter(kept) }));
  };
  const api = (id: string) =>
    ({
      snapshot: () => (log.push(`snapshot ${id}`), state.snapshot(id)),
      savePack: () => (log.push(`savePack ${id}`), state.savePack(id)),
      saveTheme: () => (log.push(`saveTheme ${id}`), state.saveTheme(id)),
      save: () => (log.push(`save ${id}`), state.save(id)),
      historyClose: () => (log.push(`historyClose ${id}`), Promise.resolve(snap())),
      takePackVersion: (pack: string, version: string, deckUid: string) => (log.push(`take ${id} ${pack} ${version} ${deckUid}`), state.take()),
    }) as unknown as TabApi;
  const input: CloseFlowInput = {
    tabApi: api,
    confirm: async (q) => {
      questions.push(q);
      log.push(`confirm ${q.message}`);
      return state.answers.shift() ?? true;
    },
    runGuard: (id) => {
      const n = (guardCalls.get(id) ?? 0) + 1;
      guardCalls.set(id, n);
      log.push(`guard ${id}`);
      return state.guard(id, n);
    },
    closing: new Set<string>(),
    stalls: new Map<string, number>(),
    refresh: () => log.push("refresh"),
    freshView: async () => (log.push("fresh"), viewOf()),
    now: () => state.clock,
    ws: {
      close: async (id: string, discard = false) => {
        log.push(`close ${id} ${discard}`);
        if (state.refused.has(`${id} ${discard}`)) throw new Error("refusé");
        drop(id);
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
  return { log, questions, state, input, deps: () => closeFlowDeps(input), guardCalls };
}

/** A promise resolved from outside. */
export function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((ok) => {
    resolve = ok;
  });
  return { promise, resolve };
}
