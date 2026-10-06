// Réglages de la cible : les paramètres de l'élément sélectionné, ou, sans sélection, du
// widget de la slide ; une surface construite par la session, posée en tête des notes.
// Chaque action part vers la slide et l'élément de la surface affichée, jamais vers la
// sélection courante.

import type { Snapshot, TabApi } from "./api";
import { type HostSettings, routeListChange, useSessionSurface } from "./useHostSettings";

/** Surface des paramètres de la cible. */
export const HOST_TARGET = "host-target";

const IDS = [HOST_TARGET] as const;

/** L'élément sélectionné, résolu dans la vue : son id et sa nature. */
export interface TargetElement {
  id: string;
  kind: string;
}

/**
 * Réglages de la cible, relus quand la slide, l'élément, le rendu, le pack ou l'annulation
 * changent. `element` : l'élément sélectionné résolu dans la vue (`null` : aucun). Un
 * graphique sélectionné ne demande rien : la carte montre son propre panneau.
 */
export function useTargetSettings(
  api: TabApi,
  snap: Snapshot | null,
  current: string | null,
  element: TargetElement | null,
  widgetActive: boolean,
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>,
  say: (m: string, error?: boolean) => void,
): HostSettings {
  const slide = snap?.slides.find((s) => s.id === current);
  const id = element?.id ?? null;
  return useSessionSurface({
    ids: IDS,
    active: !!current && !!snap && !snap.read_only && !snap.history && !widgetActive && element?.kind !== "chart",
    scope: { slide: current ?? "", element: id },
    // Deux réglages de suite du même champ gardent le même libellé d'annulation : si le rendu
    // (`hash`) ne change pas, la surface n'est pas relue, ce qui est voulu — le champ garde sa
    // valeur locale ; un refus force la relecture (`sync`).
    key: [current, id, slide?.hash, snap?.editing?.base, snap?.templates?.version, snap?.undo_label, snap?.redo_label].join("|"),
    fetch: (scope) => api.targetSettings(scope.slide, scope.element),
    send: (scope, field, context) =>
      routeListChange(
        context,
        (value) => api.targetParam(scope.slide, scope.element, field, value),
        (change) => api.targetList(scope.slide, scope.element, field, change),
      ),
    run,
    say,
  });
}
