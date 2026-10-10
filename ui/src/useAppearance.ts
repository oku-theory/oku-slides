// L'interface suit le thème des slides et le schéma (clair, sombre), à chaque changement.

import { useEffect } from "react";
import type { Appearance } from "./api";
import { applyAppearance } from "./appearance";
import { useScheme } from "./scheme";

/** Applique `appearance` (rien tant qu'elle manque), et la réapplique quand elle ou le schéma change. */
export function useAppearance(appearance: Appearance | undefined): void {
  const scheme = useScheme();
  const key = appearance ? JSON.stringify(appearance) : "";
  useEffect(() => {
    if (appearance) void applyAppearance(appearance, scheme);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- déclenché par le contenu de l'apparence
  }, [key, scheme]);
}
