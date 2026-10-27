// Sous-onglets de widgets du mode pack : logique pure.

import type { UiState } from "./api";

/** Sous-onglets gardés par l'hôte pour la reprise. */
export type KeptTabs = Pick<UiState, "widgets" | "widget">;

/** Un sous-onglet ouvert ; `id` stable : un renommage ne remonte pas l'éditeur. */
export interface OpenWidget {
  id: number;
  name: string;
}

/** The active sub-tab: `null` for the stage (a slide picked in the pile), or a widget sub-tab's id. */
export type ActiveTab = number | null;

/** Sous-onglets ouverts, dans l'ordre d'ouverture, et l'onglet actif. */
export interface WidgetTabs {
  tabs: OpenWidget[];
  active: ActiveTab;
  next: number;
}

/** Aucun sous-onglet : la scène. */
export const NO_TABS: WidgetTabs = { tabs: [], active: null, next: 1 };

/** Ajoute les sous-onglets gardés absents ; un actif déjà choisi reste. */
export function withRestored(s: WidgetTabs, names: string[], active: string | null): WidgetTabs {
  let out = s;
  for (const name of names) {
    if (!out.tabs.some((t) => t.name === name)) {
      out = { ...out, tabs: [...out.tabs, { id: out.next, name }], next: out.next + 1 };
    }
  }
  if (out.active === null) out = { ...out, active: out.tabs.find((t) => t.name === active)?.id ?? null };
  return out;
}

/** Active `name`, ouvert en fin de barre s'il ne l'est pas. */
export function opened(s: WidgetTabs, name: string): WidgetTabs {
  const found = s.tabs.find((t) => t.name === name);
  if (found) return activated(s, found.id);
  return { tabs: [...s.tabs, { id: s.next, name }], active: s.next, next: s.next + 1 };
}

/** Ouvre `name` en fin de barre sans l'activer (déjà ouvert : inchangé). */
export function openedBehind(s: WidgetTabs, name: string): WidgetTabs {
  if (s.tabs.some((t) => t.name === name)) return s;
  return { ...s, tabs: [...s.tabs, { id: s.next, name }], next: s.next + 1 };
}

/** Ouvre `name` écrit par l'agent : devant, sauf si l'actif a des frappes en attente. */
export function placeTouched(s: WidgetTabs, name: string, pending: (name: string) => boolean): WidgetTabs {
  const current = activeName(s);
  return current !== null && current !== name && pending(current) ? openedBehind(s, name) : opened(s, name);
}

/** Ferme `id` ; s'il était actif, son voisin de gauche le devient (aucun : la scène). */
export function closed(s: WidgetTabs, id: number): WidgetTabs {
  const i = s.tabs.findIndex((t) => t.id === id);
  if (i < 0) return s;
  const tabs = s.tabs.filter((t) => t.id !== id);
  const active = s.active !== id ? s.active : (tabs[i - 1]?.id ?? null);
  return { ...s, tabs, active };
}

/** Activates `id`; `null`: the stage. */
export function activated(s: WidgetTabs, id: ActiveTab): WidgetTabs {
  return s.active === id ? s : { ...s, active: id };
}

/** Le sous-onglet de `from` porte désormais `to`, même id, même place. */
export function renamed(s: WidgetTabs, from: string, to: string): WidgetTabs {
  if (!s.tabs.some((t) => t.name === from)) return s;
  return { ...s, tabs: s.tabs.map((t) => (t.name === from ? { ...t, name: to } : t)) };
}

/** Retire les sous-onglets dont le fichier n'existe plus ; actif retiré : la scène. */
export function pruned(s: WidgetTabs, names: ReadonlySet<string>): WidgetTabs {
  const tabs = s.tabs.filter((t) => names.has(t.name));
  if (tabs.length === s.tabs.length) return s;
  const active = tabs.some((t) => t.id === s.active) ? s.active : null;
  return { ...s, tabs, active };
}

/** The active widget sub-tab's name; `null`: the stage. */
export function activeName(s: WidgetTabs): string | null {
  return s.tabs.find((t) => t.id === s.active)?.name ?? null;
}

/** The active widget sub-tab's id; `null`: the stage. */
export function activeWidget(s: WidgetTabs): number | null {
  return s.active;
}

/** What the host keeps for resuming: the open sub-tabs and the active widget. */
export function kept(s: WidgetTabs): KeptTabs {
  return { widgets: s.tabs.map((t) => t.name), widget: activeName(s) };
}
