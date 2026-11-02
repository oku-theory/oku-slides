// Les boîtes de l'hôte (packs, réglages) suivent la langue de l'interface : la session n'a pas de
// requête de langue, le hub garde la boîte construite dans l'ancienne. Un changement la redemande.

import { useEffect, useRef } from "react";
import { errorText, type Surface } from "../api";
import { onLocaleChange } from "../i18n";

/**
 * Asks again for each shown turnless box `reshow` knows when the locale changes.
 *
 * The listener reads the current surfaces: the UI is remounted right after the change, but
 * locale listeners run first. `enabled`: only the panel that shows the thread
 * redemande (agent détaché, la fenêtre principale garde un panneau monté sans fil).
 */
export function useRelocalizedBoxes(
  surfaces: Surface[],
  reshow: ReadonlyMap<string, () => Promise<void>>,
  say: (m: string, error?: boolean) => void,
  enabled: boolean,
) {
  const latest = useRef({ surfaces, reshow, say, enabled });
  latest.current = { surfaces, reshow, say, enabled };
  useEffect(
    () =>
      onLocaleChange(() => {
        const now = latest.current;
        if (!now.enabled) return;
        // Une `Map` : aucune clé héritée du prototype ne passe pour une boîte.
        // A box an agent turn opened keeps its chip: reshowing it by command would drop its turn.
        for (const s of now.surfaces) if (s.turn === undefined) now.reshow.get(s.id)?.().catch((e) => now.say(errorText(e), true));
      }),
    [],
  );
}
