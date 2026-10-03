// SVG des slides en URL d'objet, mis en cache par empreinte de rendu.

import { useCallback, useEffect, useRef, useState } from "react";
import type { TabApi } from "./api";

/**
 * SVG des slides, en URL d'objet, mis en cache par empreinte de rendu.
 *
 * Pendant le rendu d'une nouvelle empreinte, la dernière image reçue de la
 * slide reste montrée : la scène et la pile ne se vident pas à chaque édition.
 */
export function useSvgCache(api: TabApi) {
  const cache = useRef(new Map<string, string>());
  /** Dernière image reçue de chaque slide, montrée en attendant la suivante. */
  const latest = useRef(new Map<string, string>());
  const [, bump] = useState(0);
  const pending = useRef(new Set<string>());
  const alive = useRef(true);
  useEffect(() => {
    // StrictMode monte, démonte puis remonte en développement : on repart
    // d'un cache vide et vivant à chaque montage.
    alive.current = true;
    const [urls, waiting, shown] = [cache.current, pending.current, latest.current];
    return () => {
      alive.current = false;
      urls.forEach((url) => URL.revokeObjectURL(url));
      urls.clear();
      waiting.clear();
      shown.clear();
    };
  }, []);
  const get = useCallback((id: string, hash: string): string | null => {
    const hit = cache.current.get(hash);
    if (hit) {
      latest.current.set(id, hit);
      return hit;
    }
    if (hash && !pending.current.has(hash)) {
      pending.current.add(hash);
      api.svg(id).then((svg) => {
        pending.current.delete(hash);
        if (!alive.current) return;
        const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
        cache.current.set(hash, url);
        latest.current.set(id, url);
        bump((n) => n + 1);
      }).catch(() => pending.current.delete(hash));
    }
    return latest.current.get(id) ?? null;
  }, [api]);
  return get;
}
