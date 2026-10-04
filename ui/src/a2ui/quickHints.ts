// Les chiffres 1–4 des pastilles ne s'affichent que là où les touches rapides arrivent :
// une question du dock qui ne les reçoit pas (une autre est plus près du micro) les cache,
// et aucune surface ne les montre quand les touches sont coupées (édition, pointage, widget).
// Chaque endroit qui rend des surfaces (fil, dock, fiche, tiroir, fenêtre) fournit la valeur ;
// sans fournisseur, pas de chiffres (une fiche hors de la slide, dans le fil, n'en reçoit jamais).

import { createContext } from "react";

/** Vrai : la surface reçoit les touches rapides, ses pastilles montrent 1–4 (faux par défaut). */
export const QuickHintsContext = createContext(false);
