// Ce que propose la grille d'ajout : les widgets d'élément de chaque pack épinglé, par section.

import type { AddPackView, AddWidget } from "./packTypes";

/** Une carte de la grille d'ajout. */
export interface WidgetChoice {
  /** Clé stable : le kind du widget. */
  key: string;
  /** Nom technique (le kind), cherché par le filtre. */
  name: string;
  /** Nom montré sur la carte. */
  label: string;
  /** Ce que montre l'élément, en une phrase ; vide si le widget n'en dit rien. */
  description: string;
  /** Le widget ajouté, avec son départ. */
  widget: AddWidget;
}

/** Une section de la grille : un pack de widgets et ses cartes. */
export interface WidgetSection {
  namespace: string;
  name: string;
  choices: WidgetChoice[];
}

/** Les sections de la grille, une par pack, dans l'ordre des packs et de leurs widgets. */
export function widgetSections(packs: AddPackView[]): WidgetSection[] {
  return packs.map((p) => ({
    namespace: p.namespace,
    name: p.name,
    choices: p.widgets.map((w) => ({ key: w.name, name: w.name, label: w.label, description: w.description ?? "", widget: w })),
  }));
}

/** Minuscules sans accents : « Étapes » et « etapes » se valent. */
function fold(s: string): string {
  return s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** Les cartes dont le nom, le libellé ou la description ont chaque mot. */
export function filterChoices(list: WidgetChoice[], query: string): WidgetChoice[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return list;
  return list.filter((c) => {
    const hay = fold(`${c.name} ${c.label} ${c.description}`);
    return words.every((w) => hay.includes(w));
  });
}
