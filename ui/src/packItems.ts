// The items of a pack tab and their slides in the working deck: one per widget or layout.
//
// The session names a widget pack's preview slides `widget-<name>` and a layout pack's
// slides `layout-<name>` (`pack_mode_preview.rs`, `pack_mode.rs`); `_commun` has none.

import type { SlideSummary, Snapshot } from "./api";
import type { PackKind } from "./packTypes";
import { COMMON } from "./widgetList";

/** The slide id prefix of each pack kind's items. */
const PREFIX: Record<PackKind, string> = { widgets: "widget-", layouts: "layout-" };

/** The working deck slide of item `name`; `null` for the shared file. */
export function itemSlide(kind: PackKind, name: string): string | null {
  return name === COMMON ? null : `${PREFIX[kind]}${name}`;
}

/** The item a working deck slide shows; `null` for any other slide. */
export function slideItem(id: string): string | null {
  for (const prefix of Object.values(PREFIX)) {
    if (id.startsWith(prefix) && id.length > prefix.length) return id.slice(prefix.length);
  }
  return null;
}

/** The pack tab's items with their slides, in slide order; empty outside a pack tab. */
export function pileItems(snap: Snapshot): { name: string; slide: SlideSummary }[] {
  const kind = snap.editing?.kind;
  if (!kind) return [];
  const prefix = PREFIX[kind];
  return snap.slides
    .filter((s) => s.id.startsWith(prefix) && s.id.length > prefix.length)
    .map((s) => ({ name: s.id.slice(prefix.length), slide: s }));
}

/**
 * The item after `gone` in `order`, else the one before, among those `kept` accepts.
 *
 * Never the shared file; `null` when `gone` is not in `order` or no item is left.
 */
export function neighbour(order: readonly string[], gone: string, kept: (name: string) => boolean): string | null {
  const at = order.indexOf(gone);
  if (at < 0) return null;
  const ok = (n: string | undefined): n is string => n !== undefined && n !== gone && n !== COMMON && kept(n);
  for (let i = at + 1; i < order.length; i++) if (ok(order[i])) return order[i];
  for (let i = at - 1; i >= 0; i--) if (ok(order[i])) return order[i];
  return null;
}
