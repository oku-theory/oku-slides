// La dictée d'un onglet : où va le texte dicté (au curseur, au brouillon), le brouillon et ses
// sorties — l'insérer tel quel dans le deck, ou l'envoyer à l'agent comme consigne.

import { useMemo, useState } from "react";
import type { AddKind } from "./slideTools";
import { draftActions } from "./agent/draftActions";
import { useDraft } from "./agent/useDraft";
import type { Op, Paragraph, SlideView, Snapshot } from "./api";
import { fieldOps, literalText, literalToParagraphs } from "./deck";
import { deckVocabulary } from "./draft";
import { t } from "./i18n";
import type { FieldKey } from "./stageFields";
import { fieldLabel, primaryFieldOf } from "./stageFields";

/** Ce que la dictée lit de l'onglet, et de quoi écrire dans le deck ou parler à l'agent. */
export interface DictationDeps {
  snap: Snapshot | null;
  view: SlideView | null;
  selected: string | null;
  /** Le champ en édition sur la scène : la dictée va au curseur. */
  editing: FieldKey | null;
  apply: (label: string, ops: Op[]) => Promise<boolean>;
  addElement: (kind: AddKind, text?: Paragraph[], label?: string) => void;
  say: (text: string, error?: boolean) => void;
  /** Une consigne pour l'agent ; vrai si elle est partie. */
  prompt: (text: string) => Promise<boolean>;
  /** Écrit un brouillon dans l'hôte (`agent.setDraft`). */
  setDraft: (text: string | null, origin: string) => Promise<void>;
}

/** Destination, vocabulaire, brouillon et texte dicté au curseur. */
export function useDictation({ snap, view, selected, editing, apply, addElement, say, prompt, setDraft }: DictationDeps) {
  const [dictated, setDictated] = useState<{ seq: number; text: string } | null>(null);
  // Hors édition, la dictée écrit dans le premier champ de l'élément sélectionné.
  const writable = view && selected ? primaryFieldOf(view, selected) : null;
  // Où « Insérer » écrit le brouillon ; `null` : aucune zone où écrire.
  const insertTo = editing
    ? t("ui-dictation-to-cursor")
    : writable
      ? writable.kind === "plain"
        ? t("ui-dictation-to-field", { field: fieldLabel(writable).toLowerCase() })
        : t("ui-dictation-to-block")
      : view?.slide.kind === "content"
        ? t("ui-dictation-new-block")
        : null;
  // Où va ⌘D : au curseur pendant l'édition, au brouillon ailleurs.
  const destination = t(editing ? "ui-dictation-to-cursor" : "ui-dictation-to-draft");
  const vocabulary = useMemo(() => deckVocabulary(snap), [snap]);

  // Hors édition, une dictée s'écrit dans le brouillon, tenu par l'hôte : « Insérer » l'écrit
  // tel quel dans le deck, « Envoyer » la donne à l'agent. ⌘K l'ouvre pour écrire.
  const draft = useDraft(setDraft);

  /** Écrit un texte tel quel : au curseur, dans la zone sélectionnée ou en nouveau bloc. */
  const insertText = (text: string): boolean => {
    if (editing) {
      setDictated((d) => ({ seq: (d?.seq ?? 0) + 1, text }));
      return true;
    }
    if (!view) return false;
    if (writable) {
      const previous = writable.text;
      const keep = writable.kind === "rich" && previous.trim() !== "" && previous !== t("ui-new-text");
      // Un champ riche relit son texte en markdown : la dictée y reste littérale.
      const dictated = writable.kind === "rich" ? literalText(text) : text;
      apply(t("ui-undo-dictation"), fieldOps(view, writable, keep ? `${previous}\n${dictated}` : dictated));
      return true;
    }
    if (view.slide.kind === "content") {
      addElement("text", literalToParagraphs(text), t("ui-undo-dictation"));
      return true;
    }
    say(t("ui-toast-select-zone", { text }), true);
    return false;
  };

  const actions = draftActions({ draft, editing: editing !== null, insert: insertText, prompt });
  return { dictated, insertTo, destination, vocabulary, draft, actions, insertText };
}
