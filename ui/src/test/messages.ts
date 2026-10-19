// Langue de l'interface remise à zéro entre les fichiers de test : `bun test`
// d'un bloc fait tourner tous les fichiers dans un même processus, où l'état de
// `../i18n` survit d'un fichier à l'autre. Un fichier qui installe des messages
// enregistre `afterAll(resetMessages)` (ou `afterEach`) ; `messages.test.ts` y
// veille.

import { applyMessages } from "../i18n";

/** Remet la langue d'avant chargement : `t` rend l'identifiant brut. */
export function resetMessages(): void {
  applyMessages({ locale: "en", resources: [], chosen: null });
}
