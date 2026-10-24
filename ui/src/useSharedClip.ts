// L'élément copié d'une fenêtre, partagé avec les autres par l'hôte (t-1182).

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, inTauri, listenTauri, workspace, type ClipReply, type SharedClip } from "./api";
import type { Clip, ClipRef } from "./useClipboard";

/** L'élément copié que l'hôte garde, sans la fenêtre qui l'a envoyé. */
export function fromShared({ tab, text, element }: SharedClip): Clip {
  return { tab, text, element };
}

/** L'élément copié dans une autre fenêtre ; `null` pour l'écho de `label` ou sans copie. */
export function remoteClip(shared: SharedClip | null, label: string): Clip | null {
  return shared && shared.origin !== label ? fromShared(shared) : null;
}

/** L'élément copié tel que la fenêtre `label` l'envoie à l'hôte : sans sa coupe, propre à l'onglet. */
export function toShared({ tab, text, element }: Clip, label: string): SharedClip {
  return { tab, text, element, origin: label };
}

/**
 * Accorde l'élément copié de la fenêtre `label` aux numéros de l'hôte.
 *
 * Seule une copie de numéro plus grand que le dernier vu compte : évènements et réponses
 * peuvent arriver dans le désordre, et l'hôte diffuse l'écho d'une copie avant d'y répondre.
 */
export function clipSync(clip: ClipRef, label: string) {
  let last = 0;
  /** La dernière copie faite ici. */
  let mine: Clip | null = null;
  /** Remet la dernière copie d'ici si une copie d'ailleurs, plus ancienne, l'a remplacée ; vrai si oui. */
  const restore = (): boolean => {
    if (!mine || clip.current === mine) return false;
    // Sa coupe marquée est tombée avec le remplacement : elle revient en simple copie.
    mine.cut = undefined;
    clip.current = mine;
    return true;
  };
  return {
    /** Lue à l'ouverture : adoptée, même venue d'ici, sauf si une copie locale existe déjà. */
    read({ clip: shared, seq }: ClipReply) {
      if (seq <= last) return;
      last = seq;
      if (!clip.current && shared) clip.current = fromShared(shared);
    },
    /** Une copie faite ici part vers l'hôte. */
    sent(copied: Clip) {
      mine = copied;
    },
    /** Diffusée (`clip`) : vrai si l'élément copié de la fenêtre a changé. */
    event({ clip: shared, seq }: ClipReply): boolean {
      if (seq <= last) return false;
      last = seq;
      const copied = remoteClip(shared, label);
      if (!copied) return restore();
      clip.current = copied;
      return true;
    },
    /** Réponse à l'envoi de `copied` : vrai si l'élément copié de la fenêtre a changé. */
    shared({ seq }: ClipReply, copied: Clip): boolean {
      if (seq <= last) return false;
      last = seq;
      return copied === mine && restore();
    },
  };
}

/** L'accord d'une fenêtre avec les numéros de l'hôte (voir [`clipSync`]). */
type ClipSync = ReturnType<typeof clipSync>;

/** Ce que la fenêtre tient du presse-papiers partagé. */
export interface SharedClipboard {
  /** L'élément copié, ici ou dans une autre fenêtre. */
  clip: ClipRef;
  /** Croît à chaque copie venue d'une autre fenêtre. */
  gen: number;
  /** Envoie à l'hôte l'élément copié dans cette fenêtre. */
  share: (clip: Clip) => void;
}

/**
 * Le presse-papiers de la fenêtre `label`, partagé avec les autres fenêtres.
 *
 * L'hôte garde la dernière copie et la diffuse (`clip`) ; une fenêtre qui s'ouvre la relit.
 */
export function useSharedClip(label: string | null, onError: (message: string) => void): SharedClipboard {
  const clip = useRef<Clip | null>(null);
  // L'accord garde ses numéros tant que la fenêtre garde son label (jamais un cache jetable).
  const synced = useRef<{ label: string; sync: ClipSync } | null>(null);
  if (label !== null && synced.current?.label !== label) synced.current = { label, sync: clipSync(clip, label) };
  const sync = synced.current?.sync ?? null;
  const mounted = useRef(true);
  const [gen, setGen] = useState(0);
  const bump = useCallback(() => setGen((g) => g + 1), []);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    if (!sync) return;
    let live = true;
    const read = () =>
      workspace
        .clip()
        .then((reply) => {
          if (live) sync.read(reply);
        })
        .catch((e) => {
          if (live) onError(errorText(e));
        });
    if (!inTauri) {
      read();
      return () => {
        live = false;
      };
    }
    // L'écoute d'abord, la lecture ensuite : aucune copie perdue entre les deux.
    const stop = listenTauri(async () => {
      const { listen } = await import("@tauri-apps/api/event");
      try {
        return await listen<ClipReply>("clip", (e) => {
          if (live && sync.event(e.payload)) bump();
        });
      } finally {
        // Même sans écoute (échec signalé par `listenTauri`), la fenêtre relit la dernière copie.
        read();
      }
    });
    return () => {
      live = false;
      stop();
    };
  }, [sync, onError, bump]);
  const share = useCallback(
    (copied: Clip) => {
      // Sans label, aucune App n'est montée : rien n'a pu être copié.
      if (!sync || label === null) return;
      sync.sent(copied);
      workspace
        .setClip(toShared(copied, label))
        .then((reply) => {
          if (mounted.current && sync.shared(reply, copied)) bump();
        })
        .catch((e) => {
          if (mounted.current) onError(errorText(e));
        });
    },
    [sync, label, onError, bump],
  );
  return { clip, gen, share };
}
