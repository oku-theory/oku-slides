// Clavier des surfaces de l'agent : 1–4 choisissent, Entrée valide, E retouche,
// Échap abandonne. Les actions sont lues dans la surface elle-même (composants
// et modèle de données) : ce que la touche envoie est ce que le clic enverrait.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "../api";

/** Une action de surface : son nom et son contexte, valeurs résolues. */
export type QuickAction = { name: string; context: Record<string, unknown> };
/** Touche rapide, indépendante de la disposition du clavier. */
export type QuickKey = "1" | "2" | "3" | "4" | "Enter" | "Escape" | "e";

type Component = { id: string; component: string; [k: string]: unknown };
type ActionEvent = { name: string; context?: Record<string, unknown> };
type Message = {
  updateComponents?: { components?: unknown };
  updateDataModel?: { path?: string; value?: unknown };
};

/** Écart sous l'élément (px). */
const GAP = 8;
/** Touches rapides : 1 à 4. */
const QUICK_KEYS = 4;
/** Nom de l'action qui abandonne une surface (jamais numérotée). */
const DISMISS = "dismiss";

/** L'action A2UI d'un composant (clic), au format des actions rapides. */
export const fromA2ui = (a: A2uiClientAction): QuickAction => ({ name: a.name, context: a.context ?? {} });

/**
 * La touche rapide d'un évènement clavier ; `null` si ce n'en est pas une.
 * Chiffres et E sont lus par leur position (`code`) : 1–4 marchent en AZERTY,
 * E sur un clavier cyrillique. Modificateurs, répétition et saisie en cours
 * de composition (IME) ne sont pas des touches rapides.
 */
export function quickKey(e: {
  key: string;
  code: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  repeat: boolean;
  isComposing?: boolean;
}): QuickKey | null {
  if (e.metaKey || e.ctrlKey || e.altKey || e.repeat || e.isComposing) return null;
  if (e.key === "Enter" || e.key === "Escape") return e.key;
  const digit = /^(?:Digit|Numpad)([1-4])$/.exec(e.code)?.[1];
  if (digit) return digit as QuickKey;
  return e.code === "KeyE" ? "e" : null;
}

/** Composants de la surface, les mises à jour successives appliquées (la dernière gagne). */
function components(s: Surface): Component[] {
  const byId = new Map<string, Component>();
  for (const m of s.messages as unknown as Message[]) {
    const list = m.updateComponents?.components;
    if (!Array.isArray(list)) continue;
    for (const c of list) {
      if (c && typeof c === "object" && typeof (c as Component).id === "string") byId.set((c as Component).id, c as Component);
    }
  }
  return [...byId.values()];
}

/** Modèle de données de la surface, les mises à jour successives appliquées. */
function dataModel(s: Surface): Record<string, unknown> {
  let root: Record<string, unknown> = {};
  for (const m of s.messages as unknown as Message[]) {
    const u = m.updateDataModel;
    if (!u) continue;
    if (!u.path || u.path === "/") root = u.value && typeof u.value === "object" ? { ...(u.value as Record<string, unknown>) } : {};
    else root = { ...root, [u.path.replace(/^\//, "")]: u.value };
  }
  return root;
}

/** La valeur d'un chemin (`/a/b`) dans le modèle ; `undefined` si absente. */
function read(model: Record<string, unknown>, path: string): unknown {
  let v: unknown = model;
  for (const part of path.split("/").filter(Boolean)) {
    if (!v || typeof v !== "object") return undefined;
    v = (v as Record<string, unknown>)[part];
  }
  return v;
}

const isPath = (v: unknown): v is { path: string } =>
  !!v && typeof v === "object" && typeof (v as { path?: unknown }).path === "string";

function eventOf(c: Component | undefined): ActionEvent | null {
  const ev = (c?.action as { event?: unknown } | undefined)?.event as ActionEvent | undefined;
  return ev && typeof ev.name === "string" ? ev : null;
}

/**
 * L'action d'un composant, valeurs liées lues dans `model` (ou dans `bound`,
 * écrit juste avant, comme le fait un clic). Une valeur introuvable : `null`,
 * plutôt qu'une action incomplète.
 */
function actionOf(c: Component | undefined, model: Record<string, unknown>, bound: Record<string, unknown> = {}): QuickAction | null {
  const ev = eventOf(c);
  if (!ev) return null;
  const context: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(ev.context ?? {})) {
    if (!isPath(v)) context[k] = v;
    else {
      const value = v.path in bound ? bound[v.path] : read(model, v.path);
      if (value === undefined) return null;
      context[k] = value;
    }
  }
  return { name: ev.name, context };
}

/** Les composants numérotés 1–4 : boutons et miniatures qui portent une action, sauf « abandonner ». */
const numbered = (cs: Component[]) =>
  cs.filter((c) => (c.component === "Button" || c.component === "SlideThumb") && eventOf(c) && eventOf(c)?.name !== DISMISS);

/** L'action qu'une touche déclenche dans `surface` ; `null` si elle n'en déclenche pas. */
export function quickAction(surface: Surface, key: string): QuickAction | null {
  if (key === "Escape") return { name: DISMISS, context: {} };
  const cs = components(surface);
  const model = dataModel(surface);
  if (key === "Enter") return actionOf(cs.find((c) => c.component === "Button" && c.primary === true), model);
  const n = Number(key);
  if (!Number.isInteger(n) || n < 1 || n > QUICK_KEYS) return null;
  const chips = cs.find((c) => c.component === "ChoiceChips");
  if (chips) {
    // Comme un clic : l'option choisie est écrite dans `value`, puis l'action la lit.
    const options = Array.isArray(chips.options) ? (chips.options as { id?: unknown }[]) : [];
    const option = options[n - 1];
    if (!option || typeof option.id !== "string" || !isPath(chips.value)) return null;
    return actionOf(chips, model, { [chips.value.path]: option.id });
  }
  return actionOf(numbered(cs)[n - 1], model);
}

/**
 * Coin de la fiche sous `target` (en pt, mis à l'échelle). En droite à gauche,
 * `left` est le bord droit : la fiche est posée avec `translateX(-100%)`.
 */
export function sheetPosition(target: { x: number; y: number; w: number; h: number }, scale: number, rtl: boolean) {
  return { left: (rtl ? target.x + target.w : target.x) * scale, top: (target.y + target.h) * scale + GAP };
}
