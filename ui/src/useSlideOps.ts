// Les gestes d'un onglet sur ses slides. On y applique des ops, ajoute un widget de la grille,
// déplace, supprime et montre une slide.

import { useCallback } from "react";
import type { Element, Op, Paragraph, SlideView, Snapshot, TabApi } from "./api";
import type { AddWidget } from "./packTypes";
import { neighbourSlide, newElementOps, newId } from "./deck";
import { t } from "./i18n";
import { BASE } from "./builtinKinds";
import { newElement } from "./newElement";
import { canRemoveSlide } from "./slideTools";
import type { FieldKey } from "./stageFields";

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
}

/** `apply` et les gestes sur les slides et leurs éléments. */
export function useSlideOps({
  api, snap, view, current, run, dropCut, setCurrent, setSelected, setEditing, editWhenShown,
  showSlide,
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

  /**
   * Ajoute un widget de la grille, parti de son départ ; sur une grille, accroché aux nœuds.
   * `override` (collage) : le texte remplace le `body` du départ, sous son propre libellé. Un
   * texte de base ajouté sans lui s'ouvre en édition.
   */
  const addElement = (widget: AddWidget, override?: { text: Paragraph[]; label: string }) => {
    const element = newElement(widget.name, newId(idPrefix(widget.name)), widget.seed);
    if (override) element.params = { ...element.params, body: override.text };
    place(element, override?.label ?? t("ui-undo-add-widget", { name: widget.label }), widget.name === BASE.text && !override);
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
  // En mode pack, les slides du deck de travail sont fixes (une par layout) : l'hôte refuserait.
  const moveSlide = (id: string, slot: number) => {
    if (!snap || snap.editing) return;
    const from = snap.slides.findIndex((s) => s.id === id);
    // `slot` : place entre deux vignettes (0 = avant la première).
    const to = slot > from ? slot - 1 : slot;
    if (from < 0 || to === from) return;
    apply(t("ui-undo-move-slide"), [{ op: "move_slide", slide: id, index: to }]);
  };

  /** Supprime une slide ; la slide affichée supprimée, on passe à sa voisine. */
  const removeSlide = (id: string) => {
    if (!snap || snap.editing || !canRemoveSlide(snap.slides.length)) return;
    const next = id === current ? neighbourSlide(snap.slides, id) : current;
    run(api.apply(t("ui-undo-remove-slide"), [{ op: "remove_slide", slide: id }])).then((s) => {
      if (s) setCurrent(next);
    });
  };

  /** Shows a slide, unselected. */
  const goToSlide = (id: string) => {
    showSlide(id);
    setSelected(null);
    setEditing(null);
  };

  return { apply, addElement, moveSlide, removeSlide, goToSlide };
}

/** Préfixe des ids d'un élément neuf : la première lettre du nom du widget (`acme/kpi` → `k`). */
function idPrefix(kind: string): string {
  return kind.slice(kind.indexOf("/") + 1).charAt(0) || "e";
}
