// Le brouillon des dictées : relu, puis inséré tel quel ou envoyé à l'agent.

import type { DictationMode } from "./agent/keys";
import type { Snapshot } from "./api";

/** Ajoute une dictée au brouillon ; `null` : pas de brouillon ouvert. */
export function appendDraft(draft: string | null, text: string): string {
  const before = draft?.trimEnd() ?? "";
  return before ? `${before} ${text}` : text;
}

/**
 * The draft once `sent` has gone to the agent: what was added meanwhile stays (`null`: nothing
 * left, the draft closes). A draft edited so that it no longer starts with `sent` is kept whole.
 */
export function afterSend(draft: string | null, sent: string): string | null {
  const now = draft?.trim() ?? "";
  // A word boundary after `sent`: "ok" sent does not cut "okay".
  if (!now.startsWith(sent) || /\S/.test(now.charAt(sent.length))) return draft;
  const left = now.slice(sent.length).trim();
  return left === "" ? null : left;
}

/** Où écrire une dictée : au curseur d'un texte en cours d'édition, ou au brouillon. */
export function dictationRoute(mode: DictationMode, editing: boolean): "cursor" | "draft" {
  return mode === "auto" && editing ? "cursor" : "draft";
}

/** Les mots du deck utiles à la dictée : son titre et ceux des slides. */
export function deckVocabulary(snap: Snapshot | null): string[] {
  return snap ? [snap.title, ...snap.slides.map((s) => s.title)].filter((s): s is string => !!s) : [];
}
