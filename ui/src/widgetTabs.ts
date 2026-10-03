// Sous-onglets de widgets du mode modèles : logique pure.

import type { UiState } from "./api";

/** Sous-onglets gardés par l'hôte pour la reprise. */
export type KeptTabs = Pick<UiState, "widgets" | "widget">;

/** Un sous-onglet ouvert ; `id` stable : un renommage ne remonte pas l'éditeur. */
export interface OpenWidget {
  id: number;
  name: string;
}

/** Sous-onglets ouverts, dans l'ordre d'ouverture ; `active` : `null` pour « Modèles ». */
export interface WidgetTabs {
  tabs: OpenWidget[];
  active: number | null;
  next: number;
}

/** Aucun sous-onglet : « Modèles » seul. */
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

/** Ferme `id` ; s'il était actif, son voisin de gauche le devient (aucun : « Modèles »). */
export function closed(s: WidgetTabs, id: number): WidgetTabs {
  const i = s.tabs.findIndex((t) => t.id === id);
  if (i < 0) return s;
  const tabs = s.tabs.filter((t) => t.id !== id);
  const active = s.active !== id ? s.active : (tabs[i - 1]?.id ?? null);
  return { ...s, tabs, active };
}

/** Active `id` ; `null` : « Modèles ». */
export function activated(s: WidgetTabs, id: number | null): WidgetTabs {
  return s.active === id ? s : { ...s, active: id };
}

/** Le sous-onglet de `from` porte désormais `to`, même id, même place. */
export function renamed(s: WidgetTabs, from: string, to: string): WidgetTabs {
  if (!s.tabs.some((t) => t.name === from)) return s;
  return { ...s, tabs: s.tabs.map((t) => (t.name === from ? { ...t, name: to } : t)) };
}

/** Retire les sous-onglets dont le fichier n'existe plus ; actif retiré : « Modèles ». */
export function pruned(s: WidgetTabs, names: ReadonlySet<string>): WidgetTabs {
  const tabs = s.tabs.filter((t) => names.has(t.name));
  if (tabs.length === s.tabs.length) return s;
  const active = tabs.some((t) => t.id === s.active) ? s.active : null;
  return { ...s, tabs, active };
}

/** Nom du sous-onglet actif ; `null` : « Modèles ». */
export function activeName(s: WidgetTabs): string | null {
  return s.tabs.find((t) => t.id === s.active)?.name ?? null;
}

/** Ce que l'hôte garde pour la reprise. */
export function kept(s: WidgetTabs): KeptTabs {
  return { widgets: s.tabs.map((t) => t.name), widget: activeName(s) };
}
