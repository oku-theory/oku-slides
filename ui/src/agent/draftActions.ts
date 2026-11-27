// Ce que font le texte dicté et les sorties du brouillon, dans la fenêtre principale comme dans
// celle de l'agent : seule l'écriture au curseur (`insert`) diffère.

import { dictationRoute } from "../draft";
import { t } from "../i18n";
import type { AgentHandle } from "./AgentPanel";
import type { Draft } from "./useDraft";
import type { DictationMode } from "./keys";

export interface DraftActions {
  /**
   * Texte dicté : au curseur pendant l'édition (`insert`), sinon au brouillon. With `send`
   * (dictation finished by hand), a draft-bound text sends the whole draft to the agent.
   */
  onDictated: (text: string, mode: DictationMode, send: boolean) => void;
  /**
   * « Envoyer » : le texte ne quitte le brouillon qu'une fois la consigne partie ; une dictée
   * arrivée entre-temps y reste. Ignored while another send is in flight.
   */
  sendDraft: (text: string, attachments: string[]) => Promise<void>;
}

/** Ce dont les sorties du brouillon ont besoin. */
export interface DraftActionsDeps {
  draft: Pick<Draft, "append" | "drop">;
  /**
   * A send in flight, kept across renders by the caller (`useRef(false)`): from the prompt
   * until the host has queued the turn, not until the turn ends.
   */
  flight: { current: boolean };
  /** Removes a sent image from a draft that keeps text added during the send. */
  removeAttachment: (id: string) => void;
  editing: boolean;
  /** Écrit le texte au curseur ; `false` : plus d'édition en cours, il revient au brouillon. */
  insert: (text: string) => boolean | Promise<boolean>;
  prompt: (text: string, attachments: string[]) => Promise<boolean>;
}

/** Route le texte dicté et envoie le brouillon à l'agent. */
export function draftActions({ draft, flight, editing, insert, prompt, removeAttachment }: DraftActionsDeps): DraftActions {
  const write = async (text: string) => {
    if (!(await insert(text))) draft.append(text);
  };
  const sendDraft = async (text: string, attachments: string[]) => {
    if (flight.current) return;
    flight.current = true;
    try {
      // Attachments added during the send stay with the text added meanwhile; sent ones go.
      if ((await prompt(text, attachments)) && draft.drop(text)) attachments.forEach(removeAttachment);
    } finally {
      flight.current = false;
    }
  };
  return {
    onDictated: (text, mode, send) => {
      if (dictationRoute(mode, editing) === "cursor") return void write(text);
      const next = draft.append(text);
      // A refused prompt has said why (toast); the draft stays open with the text. During
      // another send the text just stays in the draft.
      if (send) void sendDraft(next.text.trim(), next.attachments.map((i) => i.id));
    },
    sendDraft,
  };
}

/** The prompt of an agent panel; without a mounted panel it says so and sends nothing. */
export function promptThrough(handle: { current: Pick<AgentHandle, "prompt"> | null }, fail: (message: string) => void) {
  return (text: string, attachments: string[]): Promise<boolean> => {
    if (handle.current) return handle.current.prompt(text, attachments);
    fail(t("ui-agent-panel-missing"));
    return Promise.resolve(false);
  };
}
