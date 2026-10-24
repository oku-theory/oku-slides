// Les images collées dans une vue : chaque fichier part à l'hôte, et sa miniature reste locale
// à cette vue (URL blob du fichier de l'utilisateur), retirée quand l'image quitte le brouillon.

import { useCallback, useEffect, useRef, useState } from "react";
import type { PastedImage } from "../api";
import { assetId, toBase64 } from "./pastedImages";

/** Ce que la vue montre des images collées, et comment elle en reçoit. */
export interface PastedImages {
  /** Miniatures des images collées dans cette vue, par id. */
  thumbs: ReadonlyMap<string, string>;
  /** Envoie les fichiers collés à l'hôte, un à la fois. */
  onPaste: (files: File[]) => void;
}

/** Envoie les images collées à l'hôte et garde leurs miniatures locales. */
export function usePastedImages(images: PastedImage[], paste: (data: string) => Promise<void>, say: (m: string) => void): PastedImages {
  const [thumbs, setThumbs] = useState<ReadonlyMap<string, string>>(new Map());
  const owned = useRef(new Map<string, string>());
  const send = useRef(paste);
  send.current = paste;
  const report = useRef(say);
  report.current = say;
  const mounted = useRef(false);
  const queue = useRef(Promise.resolve());
  // Une image retirée du brouillon (ici ou ailleurs) : sa miniature est libérée.
  useEffect(() => {
    const live = new Set(images.map((i) => i.id));
    let changed = false;
    for (const [id, url] of owned.current) {
      if (!live.has(id)) {
        URL.revokeObjectURL(url);
        owned.current.delete(id);
        changed = true;
      }
    }
    if (changed) setThumbs(new Map(owned.current));
  }, [images]);
  // StrictMode démonte puis remonte : tout est libéré, la vue repart sans miniature.
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      for (const url of owned.current.values()) URL.revokeObjectURL(url);
      owned.current.clear();
    };
  }, []);
  const onPaste = useCallback((files: File[]) => {
    // Fixé dès le collage : un changement d'onglet en cours de route n'y envoie pas l'image.
    const paste = send.current;
    for (const f of files) {
      // Un fichier en échec est signalé et la chaîne continue : le collage suivant part quand même.
      queue.current = queue.current
        .then(async () => {
          const buf = await f.arrayBuffer();
          const id = await assetId(buf);
          await paste(toBase64(buf));
          // Vue démontée entre-temps : une miniature créée ici ne serait jamais libérée.
          if (id && mounted.current && !owned.current.has(id)) {
            owned.current.set(id, URL.createObjectURL(f));
            setThumbs(new Map(owned.current));
          }
        })
        .catch((e) => report.current(String(e)));
    }
  }, []);
  return { thumbs, onPaste };
}
