// The selected element's settings, in a `host-element` surface built by the session from its
// widget's schema and current values, shown in the inspector. Each change goes to the element of
// the surface shown: one undo step; a refusal rereads the surface.

import type { Element, Snapshot, TabApi } from "./api";
import { type HostSettings, listKey, routeListChange } from "./useHostSettings";
import { useSelectionSurface } from "./useSelectionSurface";

/** Surface of the selected element's settings (the inspector). */
export const HOST_ELEMENT = "host-element";

const IDS = [HOST_ELEMENT] as const;

/**
 * The `host-element` surface of any `selected` element on `slide`, where `useSelectionSurface`
 * reads; `off` removes it. A `param` action with its key sets a value (`elementParam`) or
 * changes a list (`routeListChange`, `elementList`).
 */
export function useElementSettings(
  api: TabApi,
  snap: Snapshot | null,
  slide: string | null,
  selected: Element | undefined,
  off: boolean,
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>,
  say: (m: string, error?: boolean) => void,
): HostSettings {
  return useSelectionSurface({
    ids: IDS,
    api,
    snap,
    slide,
    selected,
    off,
    fetch: (api, scope) => api.elementPanel(scope.slide, scope.element),
    send: (api, { slide, element }, { name, context }) => {
      const key = context.key;
      if (name !== "param" || typeof key !== "string") return null;
      return routeListChange(
        context,
        (value) => api.elementParam(slide, element, key, value),
        (change, path) => api.elementList(slide, element, listKey(key, path), change),
      );
    },
    run,
    say,
  });
}
