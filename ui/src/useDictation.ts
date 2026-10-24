// La dictée d'un onglet : où va le texte dicté (au curseur, au brouillon), le brouillon et ses
// sorties — le texte dicté pendant l'édition s'écrit au curseur, le brouillon part à l'agent.

import { useMemo, useState } from "react";
import { draftActions } from "./agent/draftActions";
import { useDraft } from "./agent/useDraft";
import type { Snapshot } from "./api";
import { deckVocabulary } from "./draft";
import { t } from "./i18n";
import type { FieldKey } from "./stageFields";

/** Ce que la dictée lit de l'onglet, et de quoi parler à l'agent. */
export interface DictationDeps {
  snap: Snapshot | null;
  /** Le champ en édition sur la scène : la dictée va au curseur. */
  editing: FieldKey | null;
  /** Une consigne pour l'agent ; vrai si elle est partie. */
  prompt: (text: string, images: string[]) => Promise<boolean>;
  /** Écrit un brouillon dans l'hôte (`agent.setDraft`). */
  setDraft: (text: string | null, origin: string) => Promise<void>;
}

/** Destination, vocabulaire, brouillon et texte dicté au curseur. */
export function useDictation({ snap, editing, prompt, setDraft }: DictationDeps) {
  const [dictated, setDictated] = useState<{ seq: number; text: string } | null>(null);
  // Où va ⌘D : au curseur pendant l'édition, au brouillon ailleurs.
  const destination = t(editing ? "ui-dictation-to-cursor" : "ui-dictation-to-draft");
  const vocabulary = useMemo(() => deckVocabulary(snap), [snap]);

  // Hors édition, une dictée s'écrit dans le brouillon, tenu par l'hôte : « Envoyer » la donne
  // à l'agent. ⌘K l'ouvre pour écrire.
  const draft = useDraft(setDraft);

  /** Écrit un texte tel quel au curseur ; `false` hors édition. */
  const insertText = (text: string): boolean => {
    if (!editing) return false;
    setDictated((d) => ({ seq: (d?.seq ?? 0) + 1, text }));
    return true;
  };

  const actions = draftActions({ draft, editing: editing !== null, insert: insertText, prompt });
  return { dictated, destination, vocabulary, draft, actions, insertText };
}
