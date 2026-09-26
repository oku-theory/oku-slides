// Fermer un onglet modifié : la question, et l'enregistrement qui la suit.

import { inTauri, pickSavePath, type TabApi, type TabInfo } from "./api";
import { t } from "./i18n";

/** Ce que l'utilisateur veut faire du travail non enregistré. */
export type CloseChoice = "save" | "discard" | "cancel";

/** Libellés des trois boutons du dialogue. */
export interface CloseLabels {
  save: string;
  discard: string;
  cancel: string;
}

/** Pose la question d'un document modifié qu'on ferme ; le choix de l'utilisateur. */
export type AskUnsaved = (name: string) => Promise<CloseChoice>;

/** Choisit où enregistrer un deck sans fichier ; `null` : renoncé. */
export type PickSavePath = (name: string) => Promise<string | null>;

/** Le choix d'après la réponse du dialogue (libellé cliqué, ou réponse système). */
export function choiceOf(result: string, labels: CloseLabels): CloseChoice {
  if (result === labels.save || result === "Yes") return "save";
  if (result === labels.discard || result === "No") return "discard";
  return "cancel";
}

/** Demande quoi faire d'un document modifié qu'on ferme. */
async function askUnsaved(name: string): Promise<CloseChoice> {
  const text = t("ui-close-unsaved", { name });
  // Navigateur (développement) : `window.confirm` n'a que deux boutons, pas de « ne pas enregistrer ».
  if (!inTauri) return window.confirm(text) ? "save" : "cancel";
  const labels = { save: t("ui-close-save"), discard: t("ui-close-discard"), cancel: t("ui-close-cancel") };
  const { message } = await import("@tauri-apps/plugin-dialog");
  const result = await message(text, { kind: "warning", buttons: { yes: labels.save, no: labels.discard, cancel: labels.cancel } });
  return choiceOf(result, labels);
}

/**
 * Enregistre un onglet avant de le fermer ; `false` : renoncé.
 *
 * L'ensemble de modèles en cours d'abord (puis le deck retrouvé), le deck
 * ensuite, avec un dialogue s'il n'a pas de fichier.
 */
export async function saveForClose(api: TabApi, pick: PickSavePath = pickSavePath): Promise<boolean> {
  let s = await api.snapshot();
  if (s.editing) {
    if (s.dirty) s = await api.saveTemplates(false);
    s = await api.closeTemplates();
  }
  if (!s.dirty) return true;
  const path = s.path ? null : await pick(s.name);
  if (!s.path && !path) return false;
  await api.save(path);
  return true;
}

/**
 * Peut-on fermer cet onglet ? Pose la question s'il est modifié.
 *
 * `"discard"` : fermer en abandonnant le travail ; `"keep"` : fermer (rien à
 * perdre, ou enregistré) ; `null` : ne pas fermer. `ask` et `pick` s'injectent
 * pour les tests (par défaut, la question et le dialogue réels).
 */
export async function mayClose(
  tab: TabInfo,
  api: TabApi,
  ask: AskUnsaved = askUnsaved,
  pick: PickSavePath = pickSavePath,
): Promise<"discard" | "keep" | null> {
  if (!tab.dirty) return "keep";
  const choice = await ask(tab.title);
  if (choice === "discard") return "discard";
  if (choice === "save") return (await saveForClose(api, pick)) ? "keep" : null;
  return null;
}
