// Ce qui est déplié dans une surface (détail, timeline), gardé quand elle est reconstruite :
// le reçu remplacé par sa version sans actions (expiration des suites) ne se replie pas.

import type { ComponentContext } from "@a2ui/web_core/v0_9";
import { createContext, useContext, useState } from "react";

/** Composant (identifiant et chemin de données) → déplié. Une par surface, tenue par `SurfaceView`. */
export type OpenMemory = Map<string, boolean>;

export const OpenMemoryContext = createContext<OpenMemory | null>(null);

/**
 * L'état déplié du composant de `context`, `initial` par défaut ; gardé dans la mémoire de sa surface.
 * `part` distingue les lignes dépliables d'un même composant (étapes et appels d'une timeline).
 */
export function useRememberedOpen(context: ComponentContext, initial = false, part = ""): [boolean, (open: boolean) => void] {
  const memory = useContext(OpenMemoryContext);
  // Le chemin distingue les copies d'un même composant dans une liste.
  const key = `${context.componentModel.id}@${context.dataContext.path}${part ? `#${part}` : ""}`;
  const [open, setOpen] = useState(() => memory?.get(key) ?? initial);
  return [
    open,
    (next) => {
      memory?.set(key, next);
      setOpen(next);
    },
  ];
}
