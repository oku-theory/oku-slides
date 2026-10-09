// Clavier des surfaces de l'agent : 1–4 choisissent, Entrée valide (bouton principal),
// E retouche, Échap abandonne. Les actions
// sont lues dans la surface elle-même (composants et modèle de données) : ce que la
// touche envoie est ce que le clic enverrait.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "../api";
import type { Component } from "./a2uiModel";
import { components, dataModel, isPath, read } from "./a2uiModel";

/** Une action de surface : son nom et son contexte, valeurs résolues. */
export type QuickAction = { name: string; context: Record<string, unknown> };
/** Touche rapide, indépendante de la disposition du clavier. */
export type QuickKey = "1" | "2" | "3" | "4" | "Enter" | "Escape" | "e";

type ActionEvent = { name: string; context?: Record<string, unknown> };

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
  const chips = cs.find((c) => c.component === "ChoiceChips");
  const options = Array.isArray(chips?.options) ? (chips.options as { id?: unknown }[]) : [];
  /** Comme un clic : l'option choisie est écrite dans `value`, puis l'action la lit. */
  const choose = (id: unknown) =>
    chips && typeof id === "string" && isPath(chips.value) && options.some((o) => o.id === id)
      ? actionOf(chips, model, { [chips.value.path]: id })
      : null;
  if (key === "Enter") {
    // Le bouton principal seulement : des pastilles n'ont pas d'option par défaut (`ask_choice`
    // part d'un choix vide), Entrée ne choisit jamais à la place de l'utilisateur.
    const primary = cs.find((c) => c.component === "Button" && c.primary === true);
    return primary ? actionOf(primary, model) : null;
  }
  const n = Number(key);
  if (!Number.isInteger(n) || n < 1 || n > QUICK_KEYS) return null;
  if (chips) return choose(options[n - 1]?.id);
  return actionOf(numbered(cs)[n - 1], model);
}

/** Boîte en px : bord gauche, haut, largeur, hauteur. */
export type PxBox = { left: number; top: number; width: number; height: number };
/** Zone où la fiche doit tenir (px, mêmes coordonnées que la boîte). */
export type PxZone = { left: number; top: number; right: number; bottom: number };

/**
 * Place de la fiche de l'agent près de `anchor` (px, coordonnées du cadre de la scène).
 * Sous la boîte si la place suffit dans `zone`, sinon au-dessus ; si aucun côté ne suffit,
 * du côté le plus grand, recadrée verticalement. Alignée sur le bord de départ de la boîte
 * (le droit en droite à gauche), puis recadrée horizontalement dans la zone.
 * Avant mesure (`sheet` ou `zone` inconnus) : sous la boîte, sans recadrage.
 */
export function sheetPlacement(anchor: PxBox, sheet: { width: number; height: number } | null, zone: PxZone | null, rtl: boolean) {
  const w = sheet?.width ?? 0;
  const h = sheet?.height ?? 0;
  const below = anchor.top + anchor.height + GAP;
  const start = rtl ? anchor.left + anchor.width - w : anchor.left;
  if (!sheet || !zone) return { left: start, top: below, above: false };
  const aboveTop = anchor.top - GAP - h;
  let above: boolean;
  if (below + h <= zone.bottom) above = false;
  else if (aboveTop >= zone.top) above = true;
  else above = anchor.top - zone.top > zone.bottom - below;
  const top = Math.max(zone.top, Math.min(above ? aboveTop : below, zone.bottom - h));
  const left = Math.max(zone.left, Math.min(start, zone.right - w));
  return { left, top, above };
}
