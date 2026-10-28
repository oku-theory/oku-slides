// Pack gestures of a tab: open a pack, finish editing it.
//
// Plain functions, no hook: App builds them each render. Saving a pack is asked of the agent
// (t-988); a pack opens in its own tab beside this one, a deck-only pack linked to its deck.

import type { Snapshot } from "./api";
import { widgetToOpen } from "./packOps";

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

/** True when this tab may show the finish button: a pack tab that can close. */
export function showFinish(snap: Snapshot, canClose: boolean): boolean {
  return !!snap.editing && canClose;
}

/** What the pack gestures need from App. */
interface PackGesturesContext {
  /** True: this tab is still the active one afterwards; false on any failure. */
  onOpenPack: (pack: string, widget: string) => Promise<boolean>;
  /** Closes this tab; absent when the tab cannot close. */
  onClose?: () => void;
}

/** What packGestures returns. */
interface PackGestures {
  /** Opens pack `pack` with widget `widget` in its own tab beside this one. */
  editPack: (pack: string, widget: string) => Promise<void>;
  /** Ends editing: closes this pack tab, whose close flow saves its pack. */
  finishPack: () => Promise<void>;
}

/** The pack gestures of one tab: open a pack, finish it. */
export function packGestures(ctx: PackGesturesContext): PackGestures {
  const editPack = async (pack: string, widget: string) => {
    // The pack opens in its own tab (widgetGesture answers "open" only in deck mode).
    await ctx.onOpenPack(pack, widget);
  };
  const finishPack = async () => {
    ctx.onClose?.();
  };
  return { editPack, finishPack };
}
