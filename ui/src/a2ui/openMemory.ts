// Ce qui est déplié dans une surface (détail, timeline), gardé quand elle est reconstruite :
// le reçu remplacé par sa version sans actions (expiration des suites) ne se replie pas.

import type { ComponentContext } from "@a2ui/web_core/v0_9";
import { createContext, useContext, useState } from "react";

/** Composant (identifiant et chemin de données) → déplié. Une par surface, tenue par `SurfaceView`. */
export type OpenMemory = Map<string, boolean>;

export const OpenMemoryContext = createContext<OpenMemory | null>(null);

/** L'état déplié du composant de `context`, replié par défaut ; gardé dans la mémoire de sa surface. */
export function useRememberedOpen(context: ComponentContext): [boolean, (open: boolean) => void] {
  const memory = useContext(OpenMemoryContext);
  // Le chemin distingue les copies d'un même composant dans une liste.
  const key = `${context.componentModel.id}@${context.dataContext.path}`;
  const [open, setOpen] = useState(() => memory?.get(key) ?? false);
  return [
    open,
    (next) => {
      memory?.set(key, next);
      setOpen(next);
    },
  ];
}
