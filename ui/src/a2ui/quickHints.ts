// Les chiffres 1–4 des pastilles ne s'affichent que là où les touches rapides arrivent :
// une question du dock qui ne les reçoit pas (une autre est plus près du micro) les cache.

import { createContext } from "react";

/** Vrai (par défaut) : la surface reçoit les touches rapides, ses pastilles montrent 1–4. */
export const QuickHintsContext = createContext(true);
