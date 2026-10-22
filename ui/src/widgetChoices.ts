// Ce que propose la grille d'ajout : les éléments natifs, puis les widgets d'élément du pack.

import type { AddWidget } from "./api";
import { BASE } from "./builtinKinds";
import { t } from "./i18n";
import { ADD_NAMES, type AddName, packOnlyWidgets } from "./slideTools";

/** Une carte de la grille d'ajout. */
export interface WidgetChoice {
  /** Clé stable : le kind `base/<nom>` d'une carte native, ou `widget:<kind>`. */
  key: string;
  /** Nom technique, cherché par le filtre. */
  name: string;
  /** Nom montré sur la carte. */
  label: string;
  /** Ce que montre l'élément, en une phrase ; vide si le widget n'en dit rien. */
  description: string;
  /** Ce qu'ajoute la carte : un élément natif ou un widget du pack. */
  pick: { native: AddName } | { widget: AddWidget };
}

/** Les cartes de la grille : les cartes natives, puis les widgets du pack hors de `base`. */
export function widgetChoices(pack: AddWidget[]): WidgetChoice[] {
  const natives = ADD_NAMES.map((name) => ({
    key: BASE[name],
    name,
    label: t("ui-add-kind", { kind: name }),
    description: t("ui-add-kind-description", { kind: name }),
    pick: { native: name },
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
