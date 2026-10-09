// Les miniatures des reçus de l'agent : le cache des SVG de l'onglet (partagé avec la pile),
// aux empreintes actuelles du deck. Utilisé par `App` et par la fenêtre de l'agent.

import { useMemo } from "react";
import type { Snapshot, TabApi } from "./api";
import type { SlideThumbs } from "./a2ui/slideThumbs";
import { useSvgCache } from "./useSvgCache";

export function useSlideThumbs(api: TabApi, slides: Snapshot["slides"] | undefined): SlideThumbs {
  return useThumbsOver(useSvgCache(api), slides);
}

/** Les mêmes miniatures sur un cache déjà tenu (celui que `App` partage avec la pile). */
export function useThumbsOver(svgs: ReturnType<typeof useSvgCache>, slides: Snapshot["slides"] | undefined): SlideThumbs {
  return useMemo<SlideThumbs>(
    () => ({ cached: svgs.cached, svg: svgs.svg, current: (id) => slides?.find((s) => s.id === id)?.hash }),
    [svgs, slides],
  );
}
