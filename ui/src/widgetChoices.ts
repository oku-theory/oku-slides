// Ce que propose le carrousel d'ajout : les éléments natifs, puis les widgets d'élément du pack.

import type { AddWidget } from "./api";
import { t } from "./i18n";
import { ADD_KINDS, type AddKind, packOnlyWidgets } from "./slideTools";

/** Une carte du carrousel d'ajout. */
export interface WidgetChoice {
  /** Clé stable : la sorte native, ou `widget:<nom>`. */
  key: string;
  /** Nom technique, cherché par le filtre. */
  name: string;
  /** Nom montré sur la carte. */
  label: string;
  /** Ce que montre l'élément, en une phrase ; vide si le widget n'en dit rien. */
  description: string;
  /** Ce qu'ajoute la carte : un élément natif ou un widget du pack. */
  pick: { native: AddKind } | { widget: AddWidget };
}

/** Les cartes du carrousel : les sortes natives, puis les widgets du pack sans doublon. */
export function widgetChoices(pack: AddWidget[]): WidgetChoice[] {
  const natives = ADD_KINDS.map((kind) => ({
    key: kind,
    name: kind,
    label: t("ui-add-kind", { kind }),
    description: t("ui-add-kind-description", { kind }),
    pick: { native: kind },
  }));
  const widgets = packOnlyWidgets(pack).map((w) => ({
    key: `widget:${w.name}`,
    name: w.name,
    label: w.label ?? w.name,
    description: w.description ?? "",
    pick: { widget: w },
  }));
  return [...natives, ...widgets];
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
