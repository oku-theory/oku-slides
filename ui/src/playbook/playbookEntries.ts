// The playbook's entries (fixture written by oku-slides-a2ui) and how the gallery groups them.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";

/** A gallery group, as the Rust `Group` serializes it. */
export type PlaybookGroup = "layout" | "atom" | "field" | "host_only";

/** One example: a component in one variant, as v0.9 messages. */
export interface PlaybookEntry {
  component: string;
  variant: string;
  note: string;
  group: PlaybookGroup;
  agent: boolean;
  surface: string;
  messages: A2uiMessage[];
}

/** One gallery section: a group, its title and its entries. */
export interface PlaybookSection {
  group: PlaybookGroup;
  title: string;
  entries: PlaybookEntry[];
}

/** The groups in gallery order, with their titles. */
export const GROUPS: readonly { group: PlaybookGroup; title: string }[] = [
  { group: "layout", title: "Layout" },
  { group: "atom", title: "Atoms" },
  { group: "field", title: "Fields" },
  { group: "host_only", title: "Host only" },
];

/** The entries whose component name contains `filter`, ignoring case. */
export function visible(entries: PlaybookEntry[], filter: string): PlaybookEntry[] {
  const f = filter.trim().toLowerCase();
  return f ? entries.filter((e) => e.component.toLowerCase().includes(f)) : entries;
}

/** The entries of each group, in gallery order, empty groups included. */
export function byGroup(entries: PlaybookEntry[]): PlaybookSection[] {
  return GROUPS.map(({ group, title }) => ({ group, title, entries: entries.filter((e) => e.group === group) }));
}

/** The components of an entry, as its updateComponents message carries them. */
export function componentsOf(entry: PlaybookEntry): unknown[] {
  return entry.messages.flatMap((m) => ("updateComponents" in m ? m.updateComponents.components : []));
}
