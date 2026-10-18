// Le mode modèles d'un onglet. On y entre, en sort, enregistre, change de jeu et insère un modèle.

import { useMemo } from "react";
import type { AddWidget, Op, PackInfo, Snapshot, TabApi, TemplateView } from "./api";
import { instantiate } from "./deck";
import { locale, t } from "./i18n";
import { setTemplatesOp } from "./packOps";
import type { useFileActions } from "./useFileActions";
import type { TemplateWidgets } from "./useTemplateWidgets";
import { widgetChoices } from "./widgetChoices";

/** Ce dont le mode modèles a besoin de l'onglet. */
interface TemplatesModeContext {
  api: TabApi;
  snap: Snapshot | null;
  snapRef: { current: Snapshot | null };
  current: string | null;
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  say: (text: string, error?: boolean) => void;
  apply: (label: string, ops: Op[]) => Promise<boolean>;
  autosave: ReturnType<typeof useFileActions>["autosave"];
  widgets: TemplateWidgets;
  setCurrent: (id: string | null) => void;
  setSelected: (id: string | null) => void;
  setPicking: (picking: boolean) => void;
}

/** Les gestes du mode modèles, et les widgets du pack offerts à l'ajout. */
export function useTemplatesMode({
  api, snap, snapRef, current, run, say, apply, autosave, widgets, setCurrent, setSelected,
  setPicking,
}: TemplatesModeContext) {
  const insertTemplate = (tpl: TemplateView) => {
    if (!snap) return;
    setPicking(false);
    const index = snap.slides.findIndex((s) => s.id === current) + 1;
    const slide = instantiate(tpl.slide);
    if (snap.editing) {
      // Nouveau modèle : copie du modèle choisi, sous un nom à lui.
      slide.name = t("ui-template-variant-name", { name: tpl.name });
      slide.description = tpl.description;
    }
    const label = t(snap.editing ? "ui-undo-new-template" : "ui-undo-new-slide", { name: tpl.name });
    apply(label, [{ op: "add_slide", index, slide }]).then(() => {
      setCurrent(slide.id);
      setSelected(null);
    });
  };

  // --- Édition des modèles ------------------------------------------------
  // Un pack du deck seul (import) s'y édite aussi : ses enregistrements restent dans le deck.
  const editTemplates = async (): Promise<boolean> => {
    if (!snap?.templates) return false;
    if (!(await run(api.editTemplates(snap.templates.id)))) return false;
    setCurrent(null);
    setSelected(null);
    return true;
  };
  // Clé par contenu : chaque snapshot apporte un nouveau tableau.
  const packWidgetsKey = (snap?.pack_widgets ?? []).join("\n");
  const packWidgets = useMemo(() => new Set(packWidgetsKey ? packWidgetsKey.split("\n") : []), [packWidgetsKey]);
  // Cartes de la grille d'ajout, par contenu : un nouveau tableau relancerait le défilement de la
  // grille ouverte. `lang` : les libellés natifs viennent de t(), qui suit la langue sans être lu ici.
  const addWidgetsKey = JSON.stringify(snap?.add_widgets ?? []);
  const lang = locale();
  const addChoices = useMemo(() => widgetChoices(JSON.parse(addWidgetsKey) as AddWidget[]), [addWidgetsKey, lang]);
  const finishTemplates = async () => {
    if (!snap?.editing) return;
    await autosave.flush();
    // Un enregistrement refusé laisse des modèles non enregistrés.
    if (snapRef.current?.dirty && !window.confirm(t("ui-confirm-quit-templates"))) return;
    let closed = false;
    await widgets.finish(async () => { closed = !!(await run(api.closeTemplates())); });
    if (closed) {
      setCurrent(null);
      setSelected(null);
    }
  };
  const saveTemplates = async (asNew = false) => {
    const s = await run(api.saveTemplates(asNew));
    if (s?.editing) say(t("ui-toast-templates-saved", { name: s.editing.name }));
  };
  const adoptSet = (set: PackInfo) => {
    if (!snap || snap.editing || set.id === snap.templates?.id) return;
    apply(t("ui-undo-templates", { name: set.name }), [setTemplatesOp(set)]);
  };

  return { insertTemplate, editTemplates, packWidgets, addChoices, finishTemplates, saveTemplates, adoptSet };
}
