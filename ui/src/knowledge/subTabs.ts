// Each knowledge tab's open files and shown file, kept outside its view: a tab switch unmounts
// the view, and coming back shows the same sub-tabs. The workspace drops the entries of closed tabs.

import type { KnowledgeNode } from "./types";

/** A knowledge tab's sub-tabs: its open files, as the last tree has them, and the shown one. */
export interface SubTabs {
  open: KnowledgeNode[];
  active: string | null;
}

const kept = new Map<string, SubTabs>();

/** The sub-tabs knowledge tab `tab` last showed; `undefined` before its first read. */
export function subTabsOf(tab: string): SubTabs | undefined {
  return kept.get(tab);
}

/** Keeps knowledge tab `tab`'s sub-tabs. */
export function keepSubTabs(tab: string, subTabs: SubTabs): void {
  kept.set(tab, subTabs);
}

/** Drops the sub-tabs of every tab not in `live`. */
export function pruneSubTabs(live: ReadonlySet<string>): void {
  for (const tab of kept.keys()) if (!live.has(tab)) kept.delete(tab);
}
