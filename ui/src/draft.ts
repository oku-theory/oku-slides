// Le brouillon des dictées : relu, puis inséré tel quel ou envoyé à l'agent.

import type { DictationMode } from "./agent/keys";

/** Ajoute une dictée au brouillon ; `null` : pas de brouillon ouvert. */
export function appendDraft(draft: string | null, text: string): string {
  const before = draft?.trimEnd() ?? "";
  return before ? `${before} ${text}` : text;
}

/** Où écrire une dictée : au curseur d'un texte en cours d'édition, ou au brouillon. */
export function dictationRoute(mode: DictationMode, editing: boolean): "cursor" | "draft" {
  return mode === "auto" && editing ? "cursor" : "draft";
}
