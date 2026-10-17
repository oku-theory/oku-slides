// Le cadre commun des fenêtres modales : focus pris et rendu, Tab qui tourne à l'intérieur,
// Échap, et la garde des premières 500 ms (le clic ou la frappe qui a ouvert la fenêtre).

import { useEffect, useRef, useState } from "react";
import type { MouseEvent, RefObject } from "react";

/** Délai avant qu'une fenêtre n'accepte une touche ou un clic. Une frappe en cours ne la valide pas. */
export const ARMING_MS = 500;

/** Ce que Tab peut atteindre dans une fenêtre, avant le filtre de `tabbables`. */
export const FOCUSABLE = "button, input, select, textarea, [tabindex]:not([tabindex='-1'])";

const disabled = (el: HTMLElement) =>
  ("disabled" in el && (el as HTMLButtonElement).disabled === true) || el.closest("fieldset[disabled]") !== null;

/** Une radio non cochée d'un groupe déjà coché : Tab ne s'y arrête pas. */
const skippedRadio = (root: HTMLElement, el: HTMLElement) =>
  el instanceof HTMLInputElement &&
  el.type === "radio" &&
  !el.checked &&
  el.name !== "" &&
  [...root.querySelectorAll<HTMLInputElement>('input[type="radio"]')].some((r) => r.name === el.name && r.checked);

/**
 * Les éléments que Tab parcourt dans `root`, dans l'ordre du document.
 *
 * Ni désactivés (aussi par un `fieldset` désactivé), ni `tabindex="-1"`, et d'un groupe de radios,
 * seulement la cochée.
 */
export function tabbables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (el) => el.getAttribute("tabindex") !== "-1" && !disabled(el) && !skippedRadio(root, el),
  );
}

/** Le focus suivant pour Tab (`back` : Maj+Tab). `null` : le navigateur fait déjà le bon pas. */
function wrapTarget(root: HTMLElement, back: boolean): HTMLElement | null {
  const items = tabbables(root);
  if (items.length === 0) return root;
  const first = items[0];
  const last = items[items.length - 1];
  const active = document.activeElement;
  // Le focus a quitté la fenêtre (bouton désactivé sous lui) : il y revient par un bord.
  if (!(active instanceof Node) || !root.contains(active)) return back ? last : first;
  if (back && (active === first || active === root)) return last;
  if (!back && active === last) return first;
  return null;
}

/** Les réglages du cadre. */
export interface ModalOptions {
  /** Faux : le cadre n'est pas modal. Rien n'est pris (ni focus, ni touches, ni clics). */
  active?: boolean;
  /** Échap ferme la fenêtre ; absent, Échap n'est pas touché ici (l'appelant le traite). */
  onEscape?: () => void;
}

/** Ce que le cadre rend à poser sur l'élément de la fenêtre. */
export interface ModalFrame {
  /** Ignore les clics des premières `ARMING_MS` ms. */
  onClickCapture: (e: MouseEvent) => void;
  /** Vrai une fois la garde passée : l'attribut `data-armed` de la fenêtre (e2e). */
  armed: boolean;
}

/**
 * Rend modale la fenêtre `ref`.
 *
 * Elle prend le focus et le rend en se fermant (s'il y est encore), Tab et Maj+Tab tournent à
 * l'intérieur (écoutés sur la fenêtre, comme Échap : un focus perdu y revient), Échap appelle
 * `onEscape` (jamais pendant la garde, et sans atteindre l'app), et les clics des premières
 * `ARMING_MS` ms sont ignorés.
 */
export function useModalFrame(ref: RefObject<HTMLElement | null>, { active = true, onEscape }: ModalOptions = {}): ModalFrame {
  const openedAt = useRef(Date.now());
  const [armed, setArmed] = useState(false);
  const escape = useRef(onEscape);
  escape.current = onEscape;

  useEffect(() => {
    if (!active) return;
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => {
      // Rendre le focus seulement s'il est encore ici (ou perdu) et que l'élément existe toujours.
      const now = document.activeElement;
      if (previous?.isConnected && (!now || now === document.body || ref.current?.contains(now))) previous.focus();
    };
  }, [active]);

  useEffect(() => {
    if (!active) return;
    const timer = setTimeout(() => setArmed(true), ARMING_MS);
    return () => clearTimeout(timer);
  }, [active]);

  useEffect(() => {
    if (!active) return;
    const onKey = (e: KeyboardEvent) => {
      const root = ref.current;
      if (!root) return;
      if (e.key === "Tab") {
        const target = wrapTarget(root, e.shiftKey);
        if (!target) return;
        e.preventDefault();
        target.focus();
      } else if (e.key === "Escape" && escape.current) {
        e.preventDefault();
        e.stopPropagation();
        if (Date.now() - openedAt.current >= ARMING_MS) escape.current();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [active]);

  const onClickCapture = (e: MouseEvent) => {
    if (active && Date.now() - openedAt.current < ARMING_MS) {
      e.preventDefault();
      e.stopPropagation();
    }
  };
  return { onClickCapture, armed };
}
