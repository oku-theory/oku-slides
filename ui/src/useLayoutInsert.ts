// L'ajout d'une slide d'après un layout choisi dans le sélecteur.

import type { Op, Snapshot } from "./api";
import type { LayoutView } from "./packTypes";
import { instantiate } from "./deck";
import { t } from "./i18n";

/** Ce dont l'ajout a besoin de l'onglet. */
interface LayoutInsertContext {
  snap: Snapshot | null;
  current: string | null;
  apply: (label: string, ops: Op[]) => Promise<boolean>;
  setCurrent: (id: string | null) => void;
  setSelected: (id: string | null) => void;
  setPicking: (picking: boolean) => void;
}

/** Ajoute, après la slide courante, une slide neuve du layout choisi. */
export function useLayoutInsert({ snap, current, apply, setCurrent, setSelected, setPicking }: LayoutInsertContext) {
  const insertLayout = (layout: LayoutView) => {
    if (!snap) return;
    setPicking(false);
    const index = snap.slides.findIndex((s) => s.id === current) + 1;
    // Nouveaux ids : la slide du layout sert de modèle, jamais deux fois sous le même.
    const slide = instantiate(layout.slide);
    apply(t("ui-undo-new-slide", { name: layout.label }), [{ op: "add_slide", index, slide }]).then((ok) => {
      if (!ok) return;
      setCurrent(slide.id);
      setSelected(null);
    });
  };
  return { insertLayout };
}
