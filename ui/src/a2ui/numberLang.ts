// Langue des nombres : celle du deck, pour la grille des données, la table de liste et la slide.
// App fournit la langue du deck ; sans fournisseur (ou deck sans langue), la langue de l'UI.

import { createContext, useContext } from "react";
import { locale } from "../i18n";

/** Langue du deck (étiquette BCP 47), `null` quand le deck n'en a pas. */
export const NumberLangContext = createContext<string | null>(null);

/** Langue des nombres : celle du deck, à défaut celle de l'UI. */
export function useNumberLang(): string {
  return useContext(NumberLangContext) || locale();
}
