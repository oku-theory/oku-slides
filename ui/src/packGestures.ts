// Pack gestures of a tab: open a pack, finish editing it.
//
// Plain functions, no hook: App builds them each render. Saving a pack is asked of the agent
// (t-988); a library pack opens in its own tab beside this one, a deck-only pack here.

import type { Snapshot, TabApi } from "./api";
import { type Confirm, saveForClose } from "./close";
import { widgetToOpen } from "./packOps";
import type { useFileActions } from "./useFileActions";
import type { PackWidgetEditor } from "./usePackWidgetEditor";

/** What clicking a widget does: nothing, open it here, or open its pack. */
type WidgetGesture = { kind: "none" } | { kind: "local"; name: string } | { kind: "open"; pack: string; name: string };

/**
 * What clicking widget `kind` does in this snapshot.
 *
 * In pack mode, only the edited pack's widgets open here (the preview stage also shows other
 * packs' elements); in deck mode, the pack pinned under the kind's namespace opens. A namespace
 * the deck does not pin does nothing.
 */
export function widgetGesture(snap: Snapshot, kind: string): WidgetGesture {
  const target = widgetToOpen(snap, kind);
  if (!target) return { kind: "none" };
  if (snap.editing) return target.pack === snap.editing.pack ? { kind: "local", name: target.name } : { kind: "none" };
  return target.pack ? { kind: "open", pack: target.pack, name: target.name } : { kind: "none" };
}

/** True when this tab may show the finish button. */
export function showFinish(snap: Snapshot, canClose: boolean): boolean {
  return !!snap.editing && (snap.editing.host !== "tab" || canClose);
}

/** What the pack gestures need from App. */
interface PackGesturesContext {
  /** This tab's requests. */
  api: TabApi;
  /** The last snapshot. */
  snap: Snapshot | null;
  /** The last snapshot, read after an await. */
  snapRef: { current: Snapshot | null };
  /** App's request runner: shows the error, answers null. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  /** The autosave to flush before finishing. */
  autosave: ReturnType<typeof useFileActions>["autosave"];
  /** The widget editor to settle before finishing. */
  widgets: PackWidgetEditor;
  /** Selects a slide, or none. */
  setCurrent: (id: string | null) => void;
  /** Selects an element, or none. */
  setSelected: (id: string | null) => void;
  /** True: this tab is still the active one afterwards; false on any failure. */
  onOpenPack: (pack: string, widget: string) => Promise<boolean>;
  /** Closes this tab; absent when the tab cannot close. */
  onClose?: () => void;
  /** Asks a yes/no question (App passes window.confirm). */
  confirm: Confirm;
}

/** What packGestures returns. */
interface PackGestures {
  /** Opens pack `pack` with widget `widget`, here or beside this tab. */
  editPack: (pack: string, widget: string) => Promise<void>;
  /** Ends editing: closes a pack tab, or leaves the in-deck workshop. */
  finishPack: () => Promise<void>;
}

/** The pack gestures of one tab: open a pack, finish it. */
export function packGestures(ctx: PackGesturesContext): PackGestures {
  const { api, snapRef, run, autosave, widgets, setCurrent, setSelected, onOpenPack, confirm } = ctx;
  const editPack = async (pack: string, widget: string) => {
    const before = snapRef.current?.editing?.pack;
    // A pack tab opened beside this one: this App unmounts, nothing to reset here.
    if (!(await onOpenPack(pack, widget))) return;
    // Entered here: the host opened the widget's sub-tab, which the new snapshot restores.
    const s = await run(api.snapshot());
    if (s !== null && s.editing?.pack !== before) {
      setCurrent(null);
      setSelected(null);
    }
  };
  const finishPack = async () => {
    const editing = ctx.snap?.editing;
    if (!editing) return;
    if (editing.host === "tab") {
      ctx.onClose?.();
      return;
    }
    await autosave.flush();
    // The workshop branch of saveForClose: savePack (or the read-only question), then closePack.
    await widgets.finish(async () => {
      try {
        await saveForClose(api, { confirm, first: snapRef.current ?? undefined });
      } catch (e) {
        // Shown as any refused request is.
        await run(Promise.reject(e));
      }
    });
    // saveForClose calls the API directly: the tab rereads what it left. The workshop may have
    // closed even when the deck save that followed was cancelled, so the reread decides.
    const s = await run(api.snapshot());
    if (s === null || s.editing) return;
    setCurrent(null);
    setSelected(null);
  };
  return { editPack, finishPack };
}
