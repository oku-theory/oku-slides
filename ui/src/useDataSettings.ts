// The selected chart's or table's data, in a `host-data` surface built by the session and shown
// in the data pane under the stage. Each change goes to the element of the surface shown, never
// to the current selection; a refusal rereads the surface.

import type { Element, Snapshot, TabApi } from "./api";
import type { DataEdit } from "./chart/dataEdit";
import { ownPanel } from "./hostParams";
import type { HostSettings } from "./useHostSettings";
import { useSelectionSurface } from "./useSelectionSurface";

/** Surface des données du graphique ou du tableau sélectionné. */
export const HOST_DATA = "host-data";

const IDS = [HOST_DATA] as const;

/** Une modification nommée par `edit` ; la session valide le reste. */
const isEdit = (v: unknown): v is DataEdit =>
  typeof v === "object" && v !== null && typeof (v as { edit?: unknown }).edit === "string";

/**
 * The `host-data` surface of `selected` on `slide`, read for a chart or a table
 * (`ownPanel`, from `host_params`) where `useSelectionSurface` reads; `off` removes it
 * (a pack tab, which shows no stage).
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
  const data = !!snap && ownPanel(snap.host_params, selected?.kind) !== null;
  return useSelectionSurface({
    ids: IDS,
    api,
    snap,
    slide,
    selected,
    off: off || !data,
    fetch: (api, scope) => api.dataPanel(scope.slide, scope.element),
    send: (api, scope, { name, context }) =>
      name === "data" && isEdit(context.edit) ? api.dataEdit(scope.slide, scope.element, context.edit) : null,
    run,
    say,
  });
}
