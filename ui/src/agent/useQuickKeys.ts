// Touches rapides de la question en cours : un seul écouteur, sur la fenêtre.
//
// La surface n'a pas besoin du focus : les raccourcis de l'app (⌘S, ⌘Z) restent
// actifs, un brouillon n'est pas envoyé en perdant le focus, et le rappel des
// touches reste vrai où que soit le curseur.

import { useEffect, useRef } from "react";
import type { Surface } from "../api";
import type { QuickAction } from "./quick";
import { quickAction, quickKey } from "./quick";
import type { Routed } from "./surfaces";
import { isQuestion } from "./surfaces";

/** Délai avant qu'une fenêtre n'accepte une touche ou un clic (une frappe en cours ne la valide pas). */
export const ARMING_MS = 500;

/** La surface qui reçoit les touches, et sa sorte (`dock` : une question du fil). */
export interface QuickTarget {
  surface: Surface;
  kind: "sheet" | "drawer" | "modal" | "dock";
}

/**
 * La surface qui reçoit les touches : la fenêtre, sinon la fiche, sinon une question du
 * dock (elle bloque l'agent, le tiroir non), sinon le tiroir. Plusieurs questions au dock :
 * la dernière, au plus près du micro, comme une réponse dictée (`answer_pending`).
 * Les réglages du dock n'en reçoivent pas : leurs champs ont leurs propres touches.
 */
export function quickTarget({ modal, sheet, drawer, dock }: Pick<Routed, "modal" | "drawer" | "dock"> & { sheet?: Surface }): QuickTarget | null {
  if (modal) return { surface: modal, kind: "modal" };
  if (sheet) return { surface: sheet, kind: "sheet" };
  const question = dock.filter(isQuestion).at(-1);
  if (question) return { surface: question, kind: "dock" };
  return drawer ? { surface: drawer, kind: "drawer" } : null;
}

const editable = (el: EventTarget | null) =>
  el instanceof Element && !!el.closest("input, textarea, select, [contenteditable=''], [contenteditable='true']");

/**
 * Les touches rapides de `target` (voir `quickTarget`) : 1–4, Entrée, E, Échap.
 * Ignorées dans un champ, avec un modificateur, en répétition, ou si l'évènement
 * est déjà traité (Échap d'une dictée). Entrée sur un bouton reste son clic. Une
 * question du dock ne prend que 1–4 et Entrée. Une fenêtre n'accepte qu'Échap : confirmer
 * une action non annulable ou une permission demande un geste délibéré.
 */
export function useQuickKeys(target: QuickTarget | null, enabled: boolean, onAction: (s: Surface, a: QuickAction) => void) {
  const openedAt = useRef<{ id: string; at: number } | null>(null);
  if (target && openedAt.current?.id !== target.surface.id) openedAt.current = { id: target.surface.id, at: Date.now() };
  const state = useRef({ target, enabled, onAction });
  state.current = { target, enabled, onAction };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const { target: t, enabled: on, onAction: send } = state.current;
      if (!t || !on || e.defaultPrevented || editable(e.target)) return;
      const key = quickKey(e);
      if (!key) return;
      if (key === "Enter" && e.target instanceof Element && e.target.closest("button")) return;
      if (t.kind === "modal") {
        if (key !== "Escape" || Date.now() - (openedAt.current?.at ?? 0) < ARMING_MS) return;
      } else if (t.kind === "dock" && (key === "Escape" || key === "e")) {
        // Une question du dock reste pendant qu'on édite : Échap garde son sens dans l'app
        // (désélectionner) ; elle s'abandonne par son bouton.
        return;
      } else if (key === "e") {
        const input = document.querySelector<HTMLInputElement>(`[data-surface-id="${CSS.escape(t.surface.id)}"] input`);
        if (!input) return;
        e.preventDefault();
        e.stopPropagation();
        input.focus();
        return;
      }
      const action = quickAction(t.surface, key);
      if (!action) return;
      e.preventDefault();
      e.stopPropagation();
      send(t.surface, action);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
}
