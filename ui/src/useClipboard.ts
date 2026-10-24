// Le presse-papiers d'une fenêtre : copier, couper et coller l'élément sélectionné, et la coupe marquée.

import { useCallback, useEffect, useState } from "react";
import { errorText, workspace, type Element, type Op, type Paragraph, type SlideView, type Snapshot, type TabApi } from "./api";
import type { AddWidget } from "./packTypes";
import { BASE } from "./builtinKinds";
import { copyAfterRefusedMove, findElement, literalToParagraphs, moveOps, pasteOps } from "./deck";
import { t } from "./i18n";
import { canAdd } from "./slideTools";
import type { FieldKey } from "./stageFields";
import { primaryFieldOf } from "./stageFields";

/** L'élément copié, son texte au presse-papiers, son onglet, et sa slide s'il est coupé. */
export type Clip = { element: Element; text: string; tab: string; cut?: { slide: string } };

/** Le presse-papiers de la fenêtre : il survit au changement d'onglet. */
export type ClipRef = { current: Clip | null };

/** Coller dans `tab` déplace l'élément s'il y a été coupé ; ailleurs, c'est une copie. */
export function movesHere(clip: Clip, tab: string): boolean {
  return clip.cut !== undefined && clip.tab === tab;
}

/** Colle une copie de `clip` ; venue d'un autre onglet, ses assets passent d'abord. */
export async function pasteCopy(
  clip: Clip,
  tab: string,
  carry: (from: string, to: string, element: Element) => Promise<unknown>,
  paste: () => Promise<boolean>,
): Promise<boolean> {
  if (clip.tab !== tab) await carry(clip.tab, tab, clip.element);
  return paste();
}

/** La coupe en cours, et de quoi l'abandonner. */
export interface Clipboard {
  clip: ClipRef;
  cutMark: { slide: string; element: string } | null;
  setCutMark: (mark: { slide: string; element: string } | null) => void;
  dropCut: () => void;
}

/**
 * L'élément copié (tenu par la fenêtre) et la coupe marquée de l'onglet.
 *
 * La coupe tombe quand son élément disparaît, ou quand l'onglet est quitté.
 */
export function useClipboard(clip: ClipRef, snap: Snapshot | null, view: SlideView | null): Clipboard {
  // Presse-papiers : ⌘C, ⌘X et ⌘V passent par les événements du navigateur.
  // L'élément copié reste ici ; le presse-papiers du système reçoit son texte,
  // qui dit au collage si l'élément est toujours la dernière chose copiée.
  // Couper, comme dans un tableur, ne retire rien : l'élément est marqué, et
  // le collage le déplace (`move_element`), ids et éditions des pairs gardés.
  // Le presse-papiers appartient à la fenêtre ; la coupe, à l'onglet : la quitter en fait une copie,
  // sans quoi un retour dans l'onglet déplacerait l'élément sans coupe marquée. Au montage aucune
  // coupe n'est à cet onglet : le démontage du StrictMode ne retire rien qui compte.
  useEffect(() => () => {
    if (clip.current) clip.current.cut = undefined;
  }, [clip]);
  const [cutMark, setCutMark] = useState<{ slide: string; element: string } | null>(null);
  /** Abandonne la coupe : le presse-papiers redevient une simple copie. */
  const dropCut = useCallback(() => {
    if (clip.current) clip.current.cut = undefined;
    setCutMark(null);
  }, [clip]);
  // L'élément coupé (ou sa slide) a disparu : plus rien à déplacer.
  useEffect(() => {
    if (!cutMark) return;
    const gone = (snap && !snap.slides.some((s) => s.id === cutMark.slide))
      || (view?.slide.id === cutMark.slide && !findElement(view.slide, cutMark.element));
    if (gone) dropCut();
  }, [snap, view, cutMark, dropCut]);
  return { clip, cutMark, setCutMark, dropCut };
}

/** Ce dont les événements du presse-papiers ont besoin de l'onglet. */
interface ClipboardEventsContext extends Omit<Clipboard, "cutMark"> {
  tab: string;
  api: TabApi;
  snap: Snapshot | null;
  snapRef: { current: Snapshot | null };
  view: SlideView | null;
  selected: string | null;
  editing: FieldKey | null;
  dialogOpen: boolean;
  widgetActive: boolean;
  apply: (label: string, ops: Op[]) => Promise<boolean>;
  addElement: (widget: AddWidget, override?: { text: Paragraph[]; label: string }) => void;
  setSelected: (id: string | null) => void;
  say: (text: string, error?: boolean) => void;
}

/** ⌘C, ⌘X et ⌘V, écoutés sur la fenêtre et réabonnés à chaque rendu. */
export function useClipboardEvents({
  tab, api, snap, snapRef, view, selected, editing, dialogOpen, widgetActive, apply, addElement,
  setSelected, say, clip, setCutMark, dropCut,
}: ClipboardEventsContext) {
  useEffect(() => {
    const skip = (e: ClipboardEvent) =>
      editing || dialogOpen || widgetActive || !view || snap?.history || snap?.read_only
      || (e.target as HTMLElement).closest?.('input, textarea, select, [contenteditable="true"]');
    const copy = (e: ClipboardEvent) => {
      if (skip(e) || !view || !selected) return false;
      const element = findElement(view.slide, selected);
      if (!element) return false;
      const text = primaryFieldOf(view, selected)?.text || element.kind;
      e.preventDefault();
      e.clipboardData?.setData("text/plain", text);
      clip.current = { element, text, tab };
      setCutMark(null);
      return true;
    };
    const cut = (e: ClipboardEvent) => {
      if (!view || !selected || !copy(e) || !clip.current) return;
      clip.current.cut = { slide: view.slide.id };
      setCutMark({ slide: view.slide.id, element: selected });
    };
    const paste = (e: ClipboardEvent) => {
      if (skip(e) || !view) return;
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (clip.current && text === clip.current.text) {
        e.preventDefault();
        const copied = clip.current;
        const { element, cut: from } = copied;
        // Le slot libre de la slide (canAdd le demande) doit accepter le kind de l'élément copié.
        if (!canAdd(view.slide.kind, element.kind, snap?.free_slots ?? {})) return;
        const here = view;
        const copyHere = () => {
          const { id, ops } = pasteOps(here, element);
          const applyPaste = () => apply(t("ui-undo-paste"), ops).then((ok) => {
            if (ok) setSelected(id);
            return ok;
          });
          // Venue d'un autre onglet : ses images passent d'abord dans la mémoire de celui-ci, et
          // dans son bundle si le collage les cite (t-1178). `apply` dit ses propres refus ; seul
          // l'échec de cet apport reste à dire.
          return pasteCopy(copied, tab, workspace.carryAssets, applyPaste).catch((err) => say(errorText(err), true));
        };
        // `!from` pour le typage : `movesHere` le vérifie aussi.
        if (!from || !movesHere(copied, tab)) {
          copyHere();
          return;
        }
        // Déplacer, une seule fois : ensuite, le presse-papiers colle des copies.
        dropCut();
        const { id, ops } = moveOps(here, element, from.slide);
        // Refusé : une copie seulement si l'élément n'est plus sur sa slide
        // (retiré entre-temps). Sinon l'erreur de l'hôte, déjà affichée, suffit.
        apply(t("ui-undo-move"), ops).then(async (ok) => {
          if (ok) setSelected(id);
          else if (
            await copyAfterRefusedMove(snapRef.current?.slides ?? [], from.slide, element.id, (s) => api.slide(s))
          ) copyHere();
        }).catch((err) => say(errorText(err), true));
      } else if (text.trim()) {
        // Du texte venu d'ailleurs devient un texte de base, parti de son départ dans la grille ;
        // son markdown reste littéral. Sans texte de base dans la grille, rien n'est collé.
        const widget = textWidget(snap);
        if (!widget || !canAdd(view.slide.kind, BASE.text, snap?.free_slots ?? {})) return;
        e.preventDefault();
        dropCut();
        addElement(widget, { text: literalToParagraphs(text), label: t("ui-undo-paste") });
      }
    };
    window.addEventListener("copy", copy);
    window.addEventListener("cut", cut);
    window.addEventListener("paste", paste);
    return () => {
      window.removeEventListener("copy", copy);
      window.removeEventListener("cut", cut);
      window.removeEventListener("paste", paste);
    };
  });
}

/** Le texte de base de la grille d'ajout, ou `undefined` s'il n'y est pas. */
function textWidget(snap: Snapshot | null): AddWidget | undefined {
  return snap?.add_packs.flatMap((p) => p.widgets).find((w) => w.name === BASE.text);
}
