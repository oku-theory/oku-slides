// Closing a tab: nothing is saved by hand, a modified tab is saved without a question
// (to the default folder when it has no file); a pack or theme tab saves its pack or theme, or asks.

import { pickSavePath, type Snapshot, type TabApi, type TabInfo } from "./api";
import { isolate, t } from "./i18n";
import { type AskReplaced, type PickSavePath, saveGuarded } from "./merge";
import type { PackMode, ThemeTabView } from "./packTypes";
import type { Confirm, Question } from "./useConfirm";

/** A question whose yes closes without saving: its confirming button says so. */
export const discardQuestion = (message: string): Question => ({ message, ok: t("ui-confirm-discard-ok"), danger: true });

/** What saveForClose needs besides the tab's API. */
interface SaveForCloseOptions {
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
 * History closes first (the version shown is never written), then the deck is saved. Only a
 * file replaced by another deck asks a question. A pack tab (`editing` set) has no deck to save:
 * it answers whether it is clean, and mayClose saves its pack (packTabVerdict).
 */
export async function saveForClose(api: TabApi, opts: SaveForCloseOptions = {}): Promise<boolean> {
  let s = opts.first ?? (await api.snapshot());
  const pick = opts.pick ?? pickSavePath;
  if (s.history && s.dirty) s = await api.historyClose();
  if (s.editing) return !s.dirty;
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
 * Never a deck save: a pack tab has none. A deck-pack tab whose save its deck has not taken
 * (`unsent`) is saved too. The snapshot is read again after savePack, as themeTabVerdict does:
 * the save's answer predates its forward to the deck. A read-only tab is not saved (its save
 * would be refused); a savePack rejection propagates, so a real fault keeps the tab open.
 */
async function packTabVerdict(snap: Snapshot, editing: PackMode, api: TabApi, confirm: Confirm): Promise<"keep" | "discard" | null> {
  if (!snap.dirty && !editing.unsent) return "keep";
  if (editing.read_only) return (await confirm(discardQuestion(readOnlyQuestion(editing)))) ? "discard" : null;
  await api.savePack();
  const after = await api.snapshot();
  if (!after.dirty && !after.editing?.unsent) return "keep";
  return (await confirm(discardQuestion(t("ui-confirm-quit-pack")))) ? "discard" : null;
}

/**
 * The verdict of a theme tab, as packTabVerdict: saved by saveTheme, the snapshot read after it.
 *
 * A read-only tab is not saved (another tab holds its theme); a saveTheme rejection propagates.
 */
async function themeTabVerdict(snap: Snapshot, theming: ThemeTabView, api: TabApi, confirm: Confirm): Promise<"keep" | "discard" | null> {
  if (!snap.dirty) return "keep";
  if (theming.read_only) {
    const question = discardQuestion(t("ui-confirm-quit-read-only-theme", { theme: isolate(theming.display_name) }));
    return (await confirm(question)) ? "discard" : null;
  }
  await api.saveTheme();
  if (!(await api.snapshot()).dirty) return "keep";
  return (await confirm(discardQuestion(t("ui-confirm-quit-theme")))) ? "discard" : null;
}

/**
 * May this tab close? A modified one is saved first.
 *
 * "keep": close; "discard": close, dropping unsaved work; null: stay. `snap` is the live
 * snapshot the flow read (null when the tab's live state was not needed): a pack or theme tab
 * is known by it, never by the cached kind alone.
 */
export async function mayClose(tab: TabInfo, snap: Snapshot | null, api: TabApi, deps: CloseDeps): Promise<"keep" | "discard" | null> {
  if (snap?.editing) return packTabVerdict(snap, snap.editing, api, deps.confirm);
  if (snap?.theming) return themeTabVerdict(snap, snap.theming, api, deps.confirm);
  if (!tab.dirty) return "keep";
  const saved = await saveForClose(api, { pick: deps.pick, ask: deps.ask, first: snap ?? undefined });
  return saved ? "keep" : null;
}
