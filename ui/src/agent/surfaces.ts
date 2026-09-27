// Surfaces ouvertes par l'agent, dans l'ordre d'arrivée.

import type { AgentState, McpEvent, Place, Surface } from "../api";

/**
 * Applique un évènement des outils aux surfaces.
 * Une surface remplacée garde sa place ; `surfaces` (liste de l'hôte, prise telle quelle) remplace tout.
 */
export function surfacesReducer(list: Surface[], e: McpEvent): Surface[] {
  switch (e.kind) {
    case "surface": {
      const i = list.findIndex((s) => s.id === e.surface.id);
      return i < 0 ? [...list, e.surface] : list.map((s, j) => (j === i ? e.surface : s));
    }
    case "close_surface":
      return list.filter((s) => s.id !== e.id);
    case "surfaces":
      return e.list;
    default:
      return list;
  }
}

/** Les surfaces d'une place, dans l'ordre d'arrivée. */
export const byPlace = (list: Surface[], place: Place) => list.filter((s) => s.place === place);

/** Une question de l'agent attend l'utilisateur : ⌘⇧D y répond au lieu d'arrêter le tour. */
export const isAsking = (list: Surface[]) =>
  list.some((s) => s.place === "sheet" || s.place === "thread" || s.place === "modal");

/**
 * Un tour de l'agent est en cours et n'attend pas l'utilisateur : ⌘⇧D et
 * Échap l'arrêtent. Pendant une question, ⌘⇧D y répond (Stack::prompt).
 */
export const agentBusy = (state: AgentState, list: Surface[]) => state !== "idle" && !isAsking(list);

/** Où s'affichent les surfaces ouvertes ; aucune n'est jamais invisible. */
export interface Routed {
  /** Fiche posée sous son élément, sur la scène. */
  stage?: Surface;
  /** Tiroir des variantes (le plus récent : l'hôte n'en garde qu'un). */
  drawer?: Surface;
  /** Fenêtre de confirmation (la plus ancienne : une question en attente n'est jamais remplacée sous le curseur). */
  modal?: Surface;
  /** Fil de l'agent, dans l'ordre d'arrivée : questions sans ancre, post-its, fiches hors de la scène. */
  thread: Surface[];
}

/**
 * Répartit les surfaces. `shown` : la slide affichée et ses cibles (`null`
 * pendant un changement de slide) ; une fiche n'y va que si son élément y est.
 */
export function routeSurfaces(list: Surface[], shown: { slide: string; targets: string[] } | null): Routed {
  const sheets = byPlace(list, "sheet");
  const stage = sheets.find((s) => !!shown && s.anchor?.slide === shown.slide && shown.targets.includes(s.anchor.element));
  return {
    stage,
    drawer: byPlace(list, "drawer").at(-1),
    modal: byPlace(list, "modal")[0],
    thread: list.filter((s) => s.place === "thread" || s.place === "postit" || (s.place === "sheet" && s !== stage)),
  };
}
