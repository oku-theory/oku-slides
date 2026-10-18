// Un .pptx reçu de l'OS ouvre le dialogue d'import de sa fenêtre. La vue porte ce choix.

import { useEffect, useRef } from "react";
import type { WorkspaceView } from "./api";

/**
 * Ouvre le dialogue d'import pour chaque choix d'import visant la fenêtre `label`.
 *
 * `pick` dit s'il a pris le fichier : pris, `ack` demande à l'hôte d'oublier le choix (un
 * rechargement ne le rouvre pas). Les vues qui arrivent avant sa réponse le répètent encore :
 * seul un `id` plus grand que le dernier pris rouvre le dialogue. Refusé (un import tourne,
 * `busy`), le choix n'est ni pris ni oublié : il est repris quand l'import se termine.
 */
export function useImportPick(
  view: WorkspaceView | null,
  label: string | null,
  busy: boolean,
  pick: (path: string) => boolean,
  ack: (id: number) => void,
) {
  const handled = useRef(0);
  const latest = useRef({ pick, ack });
  latest.current = { pick, ack };
  const wanted = view?.import_pick;
  const id = wanted && label !== null && wanted.window === label ? wanted.id : 0;
  const path = wanted?.path ?? "";
  useEffect(() => {
    if (id <= handled.current) return;
    if (!latest.current.pick(path)) return;
    handled.current = id;
    latest.current.ack(id);
  }, [id, path, busy]);
}
