// Données du graphique ou du tableau sélectionné, en une surface `host-data` construite par la
// session et montrée au dock de l'agent. Chaque modification part vers l'élément de la surface
// affichée, jamais vers la sélection courante ; un refus relit la surface.

import type { Element, Snapshot, TabApi } from "./api";
import type { DataEdit } from "./chart/dataEdit";
import { ownPanel } from "./hostParams";
import { type HostSettings, useSessionSurface } from "./useHostSettings";

/** Surface des données du graphique ou du tableau sélectionné. */
export const HOST_DATA = "host-data";

const IDS = [HOST_DATA] as const;

/** Une modification nommée par `edit` ; la session valide le reste. */
const isEdit = (v: unknown): v is DataEdit =>
  typeof v === "object" && v !== null && typeof (v as { edit?: unknown }).edit === "string";

/**
 * La surface `host-data` de `selected` sur `slide` : lue pour un graphique ou un tableau
 * (`ownPanel`, d'après `host_params`), hors lecture seule et historique ; `off` la retire
 * (onglet « Thème », sous-onglet widget). Relue quand l'élément, le rendu ou l'annulation changent.
 */
export function useDataSettings(
  api: TabApi,
  snap: Snapshot | null,
  slide: string | null,
  selected: Element | undefined,
  off: boolean,
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>,
  say: (m: string, error?: boolean) => void,
): HostSettings {
  const hash = snap?.slides.find((s) => s.id === slide)?.hash;
  const data = !!snap && ownPanel(snap.host_params, selected?.kind) !== null;
  return useSessionSurface({
    ids: IDS,
    active: data && !!slide && !!selected && !snap.read_only && !snap.history && !off,
    scope: { slide: slide ?? "", element: selected?.id ?? "" },
    // Un refus force la relecture (`sync`) : le modèle porte la valeur refusée.
    key: [slide, selected?.id, hash, snap?.undo_label, snap?.redo_label].join("|"),
    fetch: (scope) => api.dataPanel(scope.slide, scope.element),
    send: (scope, { name, context }) =>
      name === "data" && isEdit(context.edit) ? api.dataEdit(scope.slide, scope.element, context.edit) : null,
    run,
    say,
  });
}
