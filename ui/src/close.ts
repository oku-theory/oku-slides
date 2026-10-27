// Closing a tab: nothing is saved by hand, a modified tab is saved without a question
// (to the default folder when it has no file); a pack or theme tab saves its pack or theme, or asks.

import { pickSavePath, type Snapshot, type TabApi, type TabInfo } from "./api";
import { isolate, t } from "./i18n";
import { type AskReplaced, type PickSavePath, saveGuarded } from "./merge";
import type { PackMode, ThemeTabView } from "./packTypes";

/** Asks a yes/no question; window.confirm in the app. */
export type Confirm = (message: string) => boolean;

/** What saveForClose needs besides the tab's API. */
interface SaveForCloseOptions {
  /** Asks a yes/no question. */
  confirm: Confirm;
  /** Picks a save path for a deck without file. */
  pick?: PickSavePath;
  /** Asks whether to replace an existing file. */
  ask?: AskReplaced;
  /** The snapshot the flow already read. */
  first?: Snapshot;
}

/** The read-only question: the pack cannot be saved here, close and lose its changes? */
const readOnlyQuestion = (editing: PackMode) => t("ui-confirm-quit-read-only-pack", { pack: isolate(editing.display_name) });

/**
 * Saves a tab before it closes; false: given up.
 *
 * History closes first (the version shown is never written); then the in-deck workshop: its
 * pack is saved (a read-only one asks first and, on yes, is not saved), then closed, which gives
 * the deck back; the deck last. Only a file replaced by another deck asks a question.
 * A pack tab (`document.kind === "pack"`, which goes with `editing.host === "tab"`) has no deck to
 * give back: its pack is never closed here, and a clean one answers true (mayClose handles it).
 */
export async function saveForClose(api: TabApi, opts: SaveForCloseOptions): Promise<boolean> {
  let s = opts.first ?? (await api.snapshot());
  const pick = opts.pick ?? pickSavePath;
  if (s.history && s.dirty) s = await api.historyClose();
  if (s.editing?.host === "tab") return !s.dirty;
  if (s.editing) {
    if (s.dirty && s.editing.read_only) {
      if (!opts.confirm(readOnlyQuestion(s.editing))) return false;
    } else if (s.dirty) s = await api.savePack();
    s = await api.closePack();
  }
  if (!s.dirty) return true;
  return (await saveGuarded(api, s, opts.ask, pick)) !== null;
}

/** Saving deps of mayClose; injectable for tests. */
interface CloseDeps {
  /** Asks a yes/no question. */
  confirm: Confirm;
  /** Picks a save path for a deck without file. */
  pick?: PickSavePath;
  /** Asks whether to replace an existing file. */
  ask?: AskReplaced;
}

/**
 * The verdict of a pack tab: kept when clean or saved, discarded on request, null to stay.
 *
 * Never closePack nor a deck save: a pack tab has neither. A read-only tab is not saved (its
 * save would be refused); a savePack rejection propagates, so a real fault keeps the tab open.
 */
async function packTabVerdict(snap: Snapshot, editing: PackMode, api: TabApi, confirm: Confirm): Promise<"keep" | "discard" | null> {
  if (!snap.dirty) return "keep";
  if (editing.read_only) return confirm(readOnlyQuestion(editing)) ? "discard" : null;
  const saved = await api.savePack();
  if (!saved.dirty) return "keep";
  return confirm(t("ui-confirm-quit-pack")) ? "discard" : null;
}

/**
 * The verdict of a theme tab, as packTabVerdict: saved by saveTheme, the snapshot read after it.
 *
 * A read-only tab is not saved (another tab holds its theme); a saveTheme rejection propagates.
 */
async function themeTabVerdict(snap: Snapshot, theming: ThemeTabView, api: TabApi, confirm: Confirm): Promise<"keep" | "discard" | null> {
  if (!snap.dirty) return "keep";
  if (theming.read_only) {
    return confirm(t("ui-confirm-quit-read-only-theme", { theme: isolate(theming.display_name) })) ? "discard" : null;
  }
  await api.saveTheme();
  if (!(await api.snapshot()).dirty) return "keep";
  return confirm(t("ui-confirm-quit-theme")) ? "discard" : null;
}

/**
 * May this tab close? A modified one is saved first.
 *
 * "keep": close; "discard": close, dropping unsaved work; null: stay. `snap` is the live
 * snapshot the flow read (null when the tab's live state was not needed): a pack or theme tab
 * is known by it, never by the cached kind alone.
 */
export async function mayClose(tab: TabInfo, snap: Snapshot | null, api: TabApi, deps: CloseDeps): Promise<"keep" | "discard" | null> {
  if (snap?.editing?.host === "tab") return packTabVerdict(snap, snap.editing, api, deps.confirm);
  if (snap?.theming) return themeTabVerdict(snap, snap.theming, api, deps.confirm);
  if (!tab.dirty) return "keep";
  const saved = await saveForClose(api, { confirm: deps.confirm, pick: deps.pick, ask: deps.ask, first: snap ?? undefined });
  return saved ? "keep" : null;
}
