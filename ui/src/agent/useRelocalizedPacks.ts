// Le panneau des packs suit la langue de l'interface : la session n'a pas de requête de langue,
// le hub garde le panneau construit dans l'ancienne. Un changement de langue le redemande.

import { useEffect, useRef } from "react";
import { errorText, type Surface } from "../api";
import { onLocaleChange } from "../i18n";
import { HOST_PACKS } from "./surfaces";

/**
 * Redemande le panneau des packs (`showPacks`) quand la langue change pendant qu'il est montré.
 *
 * L'écouteur lit les surfaces à jour : l'interface est remontée juste après le changement, mais
 * les écouteurs de la langue passent avant.
 */
export function useRelocalizedPacks(surfaces: Surface[], showPacks: () => Promise<void>, say: (m: string, error?: boolean) => void) {
  const latest = useRef({ surfaces, showPacks, say });
  latest.current = { surfaces, showPacks, say };
  useEffect(
    () =>
      onLocaleChange(() => {
        const now = latest.current;
        if (now.surfaces.some((s) => s.id === HOST_PACKS)) now.showPacks().catch((e) => now.say(errorText(e), true));
      }),
    [],
  );
}
