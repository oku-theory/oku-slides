// L'import d'un .pptx, tenu par l'espace de travail : il survit à son dialogue.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, type ImportTarget } from "./api";

/** Ce que le dialogue a envoyé : la cible et le peaufinage. */
export interface ImportChoice {
  target: ImportTarget;
  tune: boolean;
}

/** L'import en cours ou à lancer : son fichier, son dialogue, son état. */
export interface ImportJob {
  path: string;
  /** Les choix de l'import lancé : le dialogue rouvert les garde. */
  choice?: ImportChoice;
  /** Le dialogue est à l'écran. */
  shown: boolean;
  /** L'hôte importe. */
  busy: boolean;
  /** Le dernier refus de l'hôte, localisé, montré par le dialogue. */
  error: string | null;
}

/**
 * L'import d'un `.pptx` de la fenêtre.
 *
 * `start` lance l'import du fichier choisi (rejeté avec le message localisé de l'hôte) ; un refus
 * arrivé quand le dialogue est fermé va à `onLateError`. Fermer le dialogue pendant l'import ne
 * l'arrête pas ; un autre choix de fichier pendant ce temps rouvre le dialogue de l'import qui
 * tourne, et `pick` le refuse (renvoie `false`). Le dialogue ouvert retient les raccourcis des
 * onglets (`holdsKeys`).
 */
export function useImportJob(onLateError: (message: string) => void) {
  const [job, setJob] = useState<ImportJob | null>(null);
  // À jour dès l'écriture : une réponse peut arriver avant le rendu qui suit une fermeture.
  const current = useRef(job);
  const put = useCallback((j: ImportJob | null) => {
    current.current = j;
    setJob(j);
  }, []);
  const late = useRef(onLateError);
  late.current = onLateError;
  // Démonté (fenêtre fermée, StrictMode) : les réponses tardives sont ignorées.
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  // Pris : le dialogue montre `path`. Pendant un import, il rouvre celui qui tourne et refuse `path`.
  const pick = useCallback(
    (path: string): boolean => {
      const j = current.current;
      if (j?.busy) {
        put({ ...j, shown: true });
        return false;
      }
      put({ path, shown: true, busy: false, error: null });
      return true;
    },
    [put],
  );

  const start = useCallback(
    async (choice: ImportChoice, importFile: (path: string) => Promise<void>) => {
      const j = current.current;
      if (!j || j.busy) return;
      put({ ...j, choice, busy: true, error: null });
      try {
        await importFile(j.path);
        if (alive.current) put(null);
      } catch (e) {
        if (!alive.current) return;
        const message = errorText(e);
        const now = current.current;
        if (now?.shown) put({ ...now, busy: false, error: message });
        else {
          put(null);
          late.current(message);
        }
      }
    },
    [put],
  );

  const close = useCallback(() => {
    const j = current.current;
    if (j) put(j.busy ? { ...j, shown: false } : null);
  }, [put]);

  const holdsKeys = useCallback(() => !!current.current?.shown, []);

  return { job, pick, start, close, holdsKeys };
}
