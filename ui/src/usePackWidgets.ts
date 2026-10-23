// Les widgets des packs du snapshot : ceux de la scène et les sections de la grille d'ajout.

import { useMemo } from "react";
import type { Snapshot } from "./api";
import type { AddPackView } from "./packTypes";
import { widgetSections } from "./widgetChoices";

/** Les widgets qualifiés des packs en cours, et les sections de la grille d'ajout. */
export function usePackWidgets(snap: Snapshot | null) {
  // Clé par contenu : chaque snapshot apporte un nouveau tableau.
  const packWidgetsKey = (snap?.pack_widgets ?? []).join("\n");
  const packWidgets = useMemo(() => new Set(packWidgetsKey ? packWidgetsKey.split("\n") : []), [packWidgetsKey]);
  // Sections de la grille, par contenu : un nouveau tableau relancerait les aperçus et le
  // défilement de la grille ouverte. La session les traduit : une autre langue change le contenu.
  const addPacksKey = JSON.stringify(snap?.add_packs ?? []);
  const addSections = useMemo(() => widgetSections(JSON.parse(addPacksKey) as AddPackView[]), [addPacksKey]);
  return { packWidgets, addSections };
}
