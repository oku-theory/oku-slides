// Ce que font le texte dicté et les sorties du brouillon, dans la fenêtre principale comme dans
// celle de l'agent : seule l'écriture au curseur (`insert`) diffère.

import { dictationRoute } from "../draft";
import type { Draft } from "./useDraft";
import type { DictationMode } from "./keys";

export interface DraftActions {
  /** Texte dicté : au curseur pendant l'édition (`insert`), sinon au brouillon. */
  onDictated: (text: string, mode: DictationMode) => void;
  /** « Envoyer » : le brouillon ne se ferme qu'une fois la consigne partie. */
  sendDraft: (text: string) => Promise<void>;
}

/** Ce dont les sorties du brouillon ont besoin. */
export interface DraftActionsDeps {
  draft: Pick<Draft, "append" | "close">;
  editing: boolean;
  /** Écrit le texte au curseur ; `false` : plus d'édition en cours, il revient au brouillon. */
  insert: (text: string) => boolean | Promise<boolean>;
  prompt: (text: string) => Promise<boolean>;
}

/** Route le texte dicté et envoie le brouillon à l'agent. */
export function draftActions({ draft, editing, insert, prompt }: DraftActionsDeps): DraftActions {
  const write = async (text: string) => {
    if (!(await insert(text))) draft.append(text);
  };
  return {
    onDictated: (text, mode) => {
      if (dictationRoute(mode, editing) === "cursor") void write(text);
      else draft.append(text);
    },
    sendDraft: async (text) => {
      if (await prompt(text)) draft.close();
    },
  };
}
