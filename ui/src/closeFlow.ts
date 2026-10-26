// Closing tabs and windows: guards, saves, the opener offer.
//
// A tab closes through closeTabFlow (the finish button, its ×, ⌘W, "close others"), a window
// through closeWindowFlow (not on quit). Both first run the tabs' bounded close guards, so the
// tab list they read next says whether a tab is dirty; a first stall refuses the close and asks
// nothing, a second one within 30 s asks whether to close anyway. A tab's kind and read-only
// state come from its live snapshot when one is read, never from an error kind.

import { errorText, type OpenerOffer, type Snapshot, type TabApi, type TabInfo, type WorkspaceView } from "./api";
import type { workspace } from "./api";
import { type Confirm, mayClose } from "./close";
import { isolate, t } from "./i18n";

/** How long a stall counts: a second one within this many milliseconds asks to close anyway. */
const STALL_MS = 30_000;

/** What closing a tab needs from the workspace. */
interface CloseFlowDeps {
  /** Ids whose close is running; a second close of one returns false. */
  closing: Set<string>;
  /** Runs a tab's close guard (closeGuards.runCloseGuard). */
  runGuard: (tab: string) => Promise<"done" | "timeout">;
  /** The current window's tabs, read when called. */
  freshTabs: () => Promise<TabInfo[]>;
  /** The tab API of one tab. */
  tabApi: (tab: string) => TabApi;
  /** Asks a yes/no question. */
  confirm: Confirm;
  /** The deck a pack tab may update (workspace.packOpener). */
  packOpener: (tab: string) => Promise<OpenerOffer | null>;
  /** False: refused, the error already shown. */
  close: (tab: string, discard: boolean) => Promise<boolean>;
  /** Rereads the view and the active snapshot. */
  refresh: () => void;
  /** Shows an error, unless the workspace unmounted. */
  report: (error: unknown) => void;
  /** First stall time per tab id; outlives each flow call (a Workspace ref). */
  stalls: Map<string, number>;
  /** The clock (Date.now in the app; injected in tests). */
  now: () => number;
}

/** What closing a window needs: CloseFlowDeps plus tab activation. */
interface CloseWindowDeps extends CloseFlowDeps {
  /** Activates a tab before asking about it. */
  activate: (tab: string) => Promise<unknown>;
  /** Closes the window, discarding `discard`; false if refused. */
  closeWindow: (discard: string[]) => Promise<boolean>;
}

/** The workspace values the close flows are built from. */
export interface CloseFlowInput
  extends Pick<CloseFlowDeps, "tabApi" | "confirm" | "runGuard" | "closing" | "stalls" | "refresh" | "freshTabs" | "now"> {
  /** The workspace calls the flows use; tests pass a fake. */
  ws: Pick<typeof workspace, "close" | "closeWindow" | "packOpener" | "activate">;
  /** This window's label, for closeWindow. */
  label: string;
  /** False once the workspace unmounted. */
  alive: { current: boolean };
  /** Workspace.run: shows the error and answers null on failure. */
  run: (p: Promise<WorkspaceView>) => Promise<WorkspaceView | null>;
  /** Shows an error text. */
  setError: (e: string | null) => void;
}

/** Builds the deps of the close flows from the workspace's values. */
export function closeFlowDeps(input: CloseFlowInput): CloseWindowDeps {
  const { ws, label, alive, run, setError } = input;
  return {
    closing: input.closing,
    runGuard: input.runGuard,
    freshTabs: input.freshTabs,
    tabApi: input.tabApi,
    confirm: input.confirm,
    stalls: input.stalls,
    now: input.now,
    refresh: input.refresh,
    packOpener: ws.packOpener,
    close: async (id, discard) => (await run(ws.close(id, discard))) !== null,
    closeWindow: async (discard) => (await run(ws.closeWindow(label, discard))) !== null,
    report: (e) => {
      if (alive.current) setError(errorText(e));
    },
    activate: ws.activate,
  };
}

/** True when the guard settled; a timeout or a rejection is a stall. */
async function guarded(id: string, deps: CloseFlowDeps): Promise<boolean> {
  try {
    return (await deps.runGuard(id)) === "done";
  } catch {
    return false;
  }
}

/** Drops the stall entries older than STALL_MS. */
function pruneStalls(deps: CloseFlowDeps): void {
  const now = deps.now();
  for (const [id, at] of deps.stalls) if (now - at > STALL_MS) deps.stalls.delete(id);
}

/** Notes a stall of `id`; true when it is a second one within STALL_MS (its entry then goes). */
function secondStall(id: string, deps: CloseFlowDeps): boolean {
  pruneStalls(deps);
  if (deps.stalls.delete(id)) return true;
  deps.stalls.set(id, deps.now());
  return false;
}

/** Tab `id` in a fresh read of the window, or undefined once it is gone. */
async function findTab(id: string, deps: CloseFlowDeps): Promise<TabInfo | undefined> {
  return (await deps.freshTabs()).find((x) => x.id === id);
}

/**
 * A stalled close of `id`: a first stall is reported and refused; a second one asks whether to
 * close anyway, without a snapshot, save or guard on a session known to be stuck.
 */
async function stalledClose(id: string, deps: CloseFlowDeps): Promise<boolean> {
  if (!secondStall(id, deps)) {
    deps.report(t("ui-close-writes-pending"));
    return false;
  }
  const tab = await findTab(id, deps);
  if (!tab) return true;
  if (!deps.confirm(t("ui-confirm-close-writes-pending"))) return false;
  // A busy session refuses a plain close: the question already said unsaved work is lost.
  if (!tab.dirty && (await deps.close(id, false))) return true;
  return deps.close(id, true);
}

/** A pack tab, known by its live snapshot when one was read, else by its cached kind. */
function isPackTab(tab: TabInfo, snap: Snapshot | null): boolean {
  return snap ? snap.editing?.host === "tab" : tab.document?.kind === "pack";
}

/** The verdict after a failed snapshot read; "gone": the tab left the window meanwhile. */
type Unread = "keep" | "discard" | null | "gone";

/**
 * After the live snapshot of `id` could not be read: a tab still of kind pack tries savePack
 * ("keep" when it resolves), else asks to quit without saving ("discard" on yes); any other
 * tab stays (null).
 */
async function unreadVerdict(id: string, error: unknown, deps: CloseFlowDeps): Promise<Unread> {
  deps.report(error);
  // The wake attempt refreshed the cached info.
  const again = await findTab(id, deps);
  if (!again) return "gone";
  if (again.document?.kind !== "pack") return null;
  try {
    await deps.tabApi(id).savePack();
    return "keep";
  } catch {
    return deps.confirm(t("ui-confirm-quit-pack")) ? "discard" : null;
  }
}

/** Step 3: the tab's live state when dirty, then its verdict; a boolean ends the flow. */
async function liveVerdict(tab: TabInfo, deps: CloseFlowDeps): Promise<{ verdict: "keep" | "discard" | null; snap: Snapshot | null } | boolean> {
  const api = deps.tabApi(tab.id);
  let snap: Snapshot | null = null;
  if (tab.dirty) {
    try {
      snap = await api.snapshot();
    } catch (e) {
      const unread = await unreadVerdict(tab.id, e, deps);
      if (unread === "gone") return true;
      if (unread === null) return false;
      return deps.close(tab.id, unread === "discard");
    }
  }
  try {
    return { verdict: await mayClose(tab, snap, api, { confirm: deps.confirm }), snap };
  } catch (e) {
    deps.report(e);
    return false;
  }
}

/** The opener's offer for a kept pack tab, and whether the user took it. */
async function askOffer(id: string, deps: CloseFlowDeps): Promise<OpenerOffer | null> {
  let offer: OpenerOffer | null = null;
  try {
    offer = await deps.packOpener(id);
  } catch (e) {
    deps.report(e);
  }
  // Title and name are labels already: isolated at display only.
  const question = offer && t("ui-confirm-update-opener", { deck: isolate(offer.title), pack: isolate(offer.pack_name) });
  return offer && question && deps.confirm(question) ? offer : null;
}

/** Closes tab `id`: guard, save, offer, close, update; false if it stays. */
export async function closeTabFlow(id: string, deps: CloseFlowDeps): Promise<boolean> {
  if (deps.closing.has(id)) return false;
  deps.closing.add(id);
  try {
    if (!(await guarded(id, deps))) return await stalledClose(id, deps);
    let tab = await findTab(id, deps);
    if (!tab) return true;
    let read = await liveVerdict(tab, deps);
    if (typeof read === "boolean") return read;
    // Typing during the save is flushed by the guard once more, before the offer reads the version.
    for (let pass = 1; read.verdict !== "discard"; pass++) {
      if (read.verdict === null) return false;
      if (!(await guarded(id, deps))) return await stalledClose(id, deps);
      tab = await findTab(id, deps);
      if (!tab) return true;
      if (!tab.dirty) break;
      if (pass === 2) {
        deps.report(t("ui-close-writes-pending"));
        return false;
      }
      read = await liveVerdict(tab, deps);
      if (typeof read === "boolean") return read;
    }
    const offer = read.verdict === "keep" && isPackTab(tab, read.snap) ? await askOffer(id, deps) : null;
    if (!(await deps.close(id, read.verdict === "discard"))) return false;
    deps.stalls.delete(id);
    if (offer) {
      try {
        await deps.tabApi(offer.tab).takePackVersion(offer.pack, offer.version, offer.deck_uid);
      } catch (e) {
        deps.report(e);
      }
      deps.refresh();
    }
    return true;
  } catch (e) {
    // A workspace read failed (the tab list): the tab stays, the error shown.
    deps.report(e);
    return false;
  } finally {
    deps.closing.delete(id);
  }
}

/** Every tab's first stall is recorded; false when any was a first, else the stalled ids. */
function sortStalls(stalled: string[], deps: CloseFlowDeps): string[] | false {
  pruneStalls(deps);
  const firsts = stalled.filter((id) => !deps.stalls.has(id));
  for (const id of firsts) deps.stalls.set(id, deps.now());
  if (firsts.length === 0) return stalled;
  deps.report(t("ui-close-writes-pending"));
  return false;
}

/** Asks about each second stall; false on a no, else true with the ids collected. */
function askStuck(seconds: string[], discarded: string[], deps: CloseFlowDeps): boolean {
  for (const id of seconds) {
    deps.stalls.delete(id);
    if (!deps.confirm(t("ui-confirm-close-writes-pending"))) return false;
    discarded.push(id);
  }
  return true;
}

/** The ids of `ids` whose guard stalled (timed out or rejected), all guards run at once. */
async function stalledOf(ids: string[], deps: CloseFlowDeps): Promise<string[]> {
  const done = await Promise.all(ids.map((id) => guarded(id, deps)));
  return ids.filter((_, i) => !done[i]);
}

/** Closes the window after guards and saves; false if it stays. */
export async function closeWindowFlow(active: string | null, deps: CloseWindowDeps): Promise<boolean> {
  const visited: string[] = [];
  const discarded: string[] = [];
  /** Gives the focus back to the tab active before, then refuses; a failed activation is reported. */
  const stay = async () => {
    if (active) await deps.activate(active).catch(deps.report);
    return false;
  };
  try {
    const firstRound = sortStalls(await stalledOf((await deps.freshTabs()).map((x) => x.id), deps), deps);
    if (firstRound === false || !askStuck(firstRound, discarded, deps)) return await stay();
    for (const id of discarded) {
      deps.closing.add(id);
      visited.push(id);
    }
    for (const tab of await deps.freshTabs()) {
      if (deps.closing.has(tab.id) || discarded.includes(tab.id)) continue;
      deps.closing.add(tab.id);
      visited.push(tab.id);
      if (!tab.dirty) continue;
      await deps.activate(tab.id);
      const api = deps.tabApi(tab.id);
      let snap: Snapshot;
      try {
        snap = await api.snapshot();
      } catch (e) {
        const unread = await unreadVerdict(tab.id, e, deps);
        if (unread === null) return await stay();
        if (unread === "discard") discarded.push(tab.id);
        continue;
      }
      const verdict = await mayClose(tab, snap, api, { confirm: deps.confirm });
      if (verdict === null) return await stay();
      if (verdict === "discard") discarded.push(tab.id);
    }
    // The activations started flushes: joined, except for collected tabs (their guard is stuck).
    const lastRound = sortStalls(await stalledOf(visited.filter((id) => !discarded.includes(id)), deps), deps);
    if (lastRound === false || !askStuck(lastRound, discarded, deps)) return await stay();
    return await deps.closeWindow(discarded);
  } catch (e) {
    deps.report(e);
    return await stay();
  } finally {
    for (const id of visited) deps.closing.delete(id);
  }
}
