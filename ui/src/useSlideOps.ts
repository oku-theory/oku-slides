// Les gestes d'un onglet sur ses slides. On y applique des ops, ajoute un élément ou un widget,
// déplace, supprime et montre une slide.

import { useCallback } from "react";
import type { AddWidget, Element, Op, Paragraph, SlideView, Snapshot, TabApi } from "./api";
import { neighbourSlide, newElementOps, newId } from "./deck";
import { t } from "./i18n";
import { newElement, newPackWidget } from "./newElement";
import type { AddKind } from "./slideTools";
import { canRemoveSlide } from "./slideTools";
import type { FieldKey } from "./stageFields";
import type { TemplateWidgets } from "./useTemplateWidgets";

/** Ce dont les gestes sur les slides ont besoin de l'onglet. */
interface SlideOpsContext {
  api: TabApi;
  snap: Snapshot | null;
  view: SlideView | null;
  current: string | null;
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  dropCut: () => void;
  setCurrent: (id: string | null) => void;
  setSelected: (id: string | null) => void;
  setEditing: (key: FieldKey | null) => void;
  editWhenShown: (owner: string) => void;
  showSlide: (id: string) => void;
  widgetActive: boolean;
  widgets: TemplateWidgets;
}

/** `apply` et les gestes sur les slides et leurs éléments. */
export function useSlideOps({
  api, snap, view, current, run, dropCut, setCurrent, setSelected, setEditing, editWhenShown,
  showSlide, widgetActive, widgets,
}: SlideOpsContext) {
  const apply = useCallback(
    /** Vrai si l'hôte a appliqué les ops. */
    async (label: string, ops: Op[], merge?: string) => {
      // Toute modification abandonne la coupe, comme dans un tableur.
      dropCut();
      return (await run(api.apply(label, ops, merge))) !== null;
    },
    [run, dropCut],
  );

  /** Ajoute un texte, une carte, une forme ou un graphique ; sur une grille, accroché aux nœuds. */
  const addElement = (kind: AddKind, text?: Paragraph[], label?: string) => {
    const element = newElement(kind, newId(kind[0]));
    if (kind === "text" && text) element.params = { ...element.params, body: text };
    place(element, label ?? t("ui-undo-add", { kind }), kind === "text" && !text);
  };

  /** Ajoute un widget du pack, avec les défauts de ses paramètres. */
  const addPackWidget = (w: AddWidget) => {
    place(newPackWidget(w.name, newId("w")), t("ui-undo-add-widget", { name: w.label ?? w.name }), false);
  };

  /** Pose `element` sur la slide affichée, puis le sélectionne (et l'édite si `edit`). */
  const place = (element: Element, label: string, edit: boolean) => {
    if (!view) return;
    const ops = newElementOps(view, element);
    apply(label, ops).then(() => {
      setSelected(element.id);
      if (edit) editWhenShown(element.id);
    });
  };

  // --- Réarranger les slides par glisser-déposer -----------------------------
  const moveSlide = (id: string, slot: number) => {
    if (!snap) return;
    const from = snap.slides.findIndex((s) => s.id === id);
    // `slot` : place entre deux vignettes (0 = avant la première).
    const to = slot > from ? slot - 1 : slot;
    if (from < 0 || to === from) return;
    apply(t(snap.editing ? "ui-undo-move-template" : "ui-undo-move-slide"), [{ op: "move_slide", slide: id, index: to }]);
  };

  /** Supprime une slide ; la slide affichée supprimée, on passe à sa voisine. */
  const removeSlide = (id: string) => {
    if (!snap || !canRemoveSlide(snap.slides.length)) return;
    const next = id === current ? neighbourSlide(snap.slides, id) : current;
    run(api.apply(t("ui-undo-remove-slide"), [{ op: "remove_slide", slide: id }])).then((s) => {
      if (s) setCurrent(next);
    });
  };

  /** Montre une slide, sans sélection ; un sous-onglet widget ouvert cède la place au modèle, « Thème » reste. */
  const goToSlide = (id: string) => {
    showSlide(id);
    setSelected(null);
    setEditing(null);
    if (widgetActive) widgets.activate(null);
  };

  return { apply, addElement, addPackWidget, moveSlide, removeSlide, goToSlide };
}
