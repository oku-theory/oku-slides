// Fermer un onglet modifié : on n'enregistre jamais à la main, il est
// enregistré sans question (au dossier par défaut s'il n'a pas de fichier).

import { pickSavePath, type TabApi, type TabInfo } from "./api";
import { type AskReplaced, type PickSavePath, saveGuarded } from "./merge";

/**
 * Enregistre un onglet avant de le fermer ; `false` : renoncé.
 *
 * L'historique se ferme d'abord (la version montrée ne s'écrit jamais) ;
 * le pack en cours ensuite (puis le deck retrouvé), le deck
 * enfin ; seul un fichier remplacé par un autre deck pose une question
 * (`ask` et `pick`, injectables pour les tests).
 */
export async function saveForClose(api: TabApi, pick: PickSavePath = pickSavePath, ask?: AskReplaced): Promise<boolean> {
  let s = await api.snapshot();
  if (s.history && s.dirty) s = await api.historyClose();
  if (s.editing) {
    if (s.dirty) s = await api.savePack();
    s = await api.closePack();
  }
  if (!s.dirty) return true;
  return (await saveGuarded(api, s, ask, pick)) !== null;
}

/**
 * Peut-on fermer cet onglet ? Modifié, il est d'abord enregistré.
 *
 * `"keep"` : fermer (rien à perdre) ; `null` : ne pas fermer (enregistrement
 * refusé). `pick` et `ask` s'injectent pour les tests.
 */
export async function mayClose(
  tab: TabInfo,
  api: TabApi,
  pick: PickSavePath = pickSavePath,
  ask?: AskReplaced,
): Promise<"keep" | null> {
  if (!tab.dirty) return "keep";
  return (await saveForClose(api, pick, ask)) ? "keep" : null;
}
