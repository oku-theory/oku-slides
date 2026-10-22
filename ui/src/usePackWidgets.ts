// Les widgets des packs du snapshot : ceux de la scène et les cartes de la grille d'ajout.

import { useMemo } from "react";
import type { AddWidget, Snapshot } from "./api";
import { locale } from "./i18n";
import { widgetChoices } from "./widgetChoices";

/** Les widgets qualifiés des packs en cours, et les cartes de la grille d'ajout. */
export function usePackWidgets(snap: Snapshot | null) {
  // Clé par contenu : chaque snapshot apporte un nouveau tableau.
  const packWidgetsKey = (snap?.pack_widgets ?? []).join("\n");
  const packWidgets = useMemo(() => new Set(packWidgetsKey ? packWidgetsKey.split("\n") : []), [packWidgetsKey]);
  // Cartes de la grille d'ajout, par contenu : un nouveau tableau relancerait le défilement de la
  // grille ouverte. `lang` : les libellés natifs viennent de t(), qui suit la langue sans être lu ici.
  const addWidgetsKey = JSON.stringify(snap?.add_widgets ?? []);
  const lang = locale();
  const addChoices = useMemo(() => widgetChoices(JSON.parse(addWidgetsKey) as AddWidget[]), [addWidgetsKey, lang]);
  return { packWidgets, addChoices };
}
