// Langue des nombres d'un tableau de liste : celle du deck, comme la grille et la slide.
// App fournit la langue du deck ; sans fournisseur (ou deck sans langue), la langue de l'UI.

import { createContext } from "react";

/** Langue du deck (étiquette BCP 47), `null` quand le deck n'en a pas. */
export const NumberLangContext = createContext<string | null>(null);
