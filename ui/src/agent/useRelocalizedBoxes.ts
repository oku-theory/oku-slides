// Les boîtes de l'hôte (packs, réglages) suivent la langue de l'interface : la session n'a pas de
// requête de langue, le hub garde la boîte construite dans l'ancienne. Un changement la redemande.

import { useEffect, useRef } from "react";
import { errorText, type Surface } from "../api";
import { onLocaleChange } from "../i18n";

/**
 * Redemande chaque boîte montrée dont `reshow` connaît l'identifiant quand la langue change.
 *
 * L'écouteur lit les surfaces à jour : l'interface est remontée juste après le changement, mais
 * les écouteurs de la langue passent avant. `enabled` : seul le panneau qui montre le fil
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
        for (const s of now.surfaces) now.reshow.get(s.id)?.().catch((e) => now.say(errorText(e), true));
      }),
    [],
  );
}
