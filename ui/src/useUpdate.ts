// Mise à jour de l'app dans une fenêtre : état de l'hôte, « Plus tard », installation.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, subscribeUpdates, type UpdateApi, type UpdateOffer, type UpdateStatus } from "./api";

/** Ce qu'une fenêtre montre et peut faire d'une mise à jour. */
export interface UpdateState {
  /** Offre à montrer (masquée par « Plus tard », sauf pendant l'installation). */
  offer: UpdateOffer | null;
  installing: boolean;
  /** Avancée du téléchargement, `null` si inconnue. */
  percent: number | null;
  /** Recherches demandées par le menu de l'app, comptées. */
  requested: number;
  /** Recherche ; `null` sans hôte, rejet sur une erreur de l'hôte. */
  check(): Promise<UpdateStatus | null>;
  install(): void;
  later(): void;
  openRelease(): void;
}

/** État de la mise à jour pour une fenêtre ; `api` absent : rien à montrer. */
export function useUpdate(api: UpdateApi | null, onError: (text: string) => void): UpdateState {
  const [status, setStatus] = useState<UpdateStatus>({ offer: null, installing: false });
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [percent, setPercent] = useState<number | null>(null);
  const [requested, setRequested] = useState(0);
  const [pending, setPending] = useState(false);
  // Toujours le dernier rappel, sans réabonner l'effet quand il change.
  const report = useRef(onError);
  report.current = onError;
  // Seule la dernière lecture demandée compte (réponses dans le désordre).
  const seq = useRef(0);

  const reread = useCallback(() => {
    if (!api) return;
    const n = ++seq.current;
    api.status().then(
      (s) => {
        if (n !== seq.current) return;
        setStatus(s);
        // L'hôte a fini (ou n'a jamais commencé) : le clic en attente est retombé.
        if (!s.installing) setPending(false);
      },
      (e) => console.error("mise à jour", e),
    );
  }, [api]);

  useEffect(() => {
    if (!api) return;
    return subscribeUpdates(
      api,
      {
        onChanged: reread,
        onProgress: (p) => setPercent(p.total ? Math.min(100, Math.floor((p.received / p.total) * 100)) : null),
        onCheckRequested: () => setRequested((n) => n + 1),
      },
      reread,
    );
  }, [api, reread]);

  const check = useCallback(async () => {
    if (!api) return null;
    const s = await api.check();
    seq.current++;
    setDismissed(null);
    setStatus(s);
    return s;
  }, [api]);

  const install = useCallback(() => {
    if (!api) return;
    setPending(true);
    // Le pourcentage d'un essai précédent ne doit pas rester affiché.
    setPercent(null);
    api.install().catch((e) => {
      setPending(false);
      report.current(errorText(e));
    });
  }, [api]);

  const later = useCallback(() => setDismissed(status.offer?.version ?? null), [status.offer]);

  const openRelease = useCallback(() => {
    api?.openRelease().catch((e) => report.current(errorText(e)));
  }, [api]);

  const installing = status.installing || pending;
  const offer = status.offer && (installing || status.offer.version !== dismissed) ? status.offer : null;
  return { offer, installing, percent, requested, check, install, later, openRelease };
}
