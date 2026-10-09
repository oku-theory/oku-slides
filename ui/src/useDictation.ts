// La dictée d'un onglet : où va le texte dicté (au curseur, au brouillon), le brouillon et ses
// sorties — l'insérer tel quel dans le deck, ou l'envoyer à l'agent comme consigne.

import { useMemo, useState } from "react";
import type { AddKind } from "./slideTools";
import type { DictationMode } from "./agent/keys";
import type { Op, Paragraph, SlideView, Snapshot } from "./api";
import { fieldOps, textToParagraphs } from "./deck";
import { appendDraft, dictationRoute } from "./draft";
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
}

/** Destination, vocabulaire, brouillon et texte dicté au curseur. */
export function useDictation({ snap, view, selected, editing, apply, addElement, say, prompt }: DictationDeps) {
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
  const vocabulary = useMemo(
    () => (snap ? [snap.title, ...snap.slides.map((s) => s.title)].filter((s): s is string => !!s) : []),
    [snap],
  );

  // Hors édition, une dictée s'écrit dans le brouillon : « Insérer » l'écrit
  // tel quel dans le deck, « Envoyer » la donne à l'agent. ⌘K l'ouvre pour écrire.
  const [draft, setDraft] = useState<{ text: string | null; dictations: number; typing: number }>({ text: null, dictations: 0, typing: 0 });
  const closeDraft = () => setDraft((d) => ({ ...d, text: null }));
  const typeDraft = () => setDraft((d) => ({ ...d, text: d.text ?? "", typing: d.typing + 1 }));
  // Le brouillon ne se ferme qu'une fois la consigne partie : un échec garde le texte dicté.
  const sendDraft = (text: string) =>
    prompt(text).then((sent) => {
      if (sent) closeDraft();
    });
  const insertDraft = (text: string) => {
    closeDraft();
    insertText(text);
  };

  const onDictated = (text: string, mode: DictationMode) => {
    if (dictationRoute(mode, editing !== null) === "cursor") insertText(text);
    else setDraft((d) => ({ ...d, text: appendDraft(d.text, text), dictations: d.dictations + 1 }));
  };

  /** Écrit un texte tel quel : au curseur, dans la zone sélectionnée ou en nouveau bloc. */
  const insertText = (text: string) => {
    if (editing) {
      setDictated((d) => ({ seq: (d?.seq ?? 0) + 1, text }));
      return;
    }
    if (!view) return;
    if (writable) {
      const previous = writable.text;
      const keep = writable.kind === "rich" && previous.trim() !== "" && previous !== t("ui-new-text");
      apply(t("ui-undo-dictation"), fieldOps(view, writable, keep ? `${previous}\n${text}` : text));
      return;
    }
    if (view.slide.kind === "content") {
      addElement("text", textToParagraphs(text), t("ui-undo-dictation"));
      return;
    }
    say(t("ui-toast-select-zone", { text }), true);
  };

  const setDraftText = (text: string) => setDraft((d) => ({ ...d, text }));
  return { dictated, insertTo, destination, vocabulary, draft, setDraftText, closeDraft, typeDraft, sendDraft, insertDraft, onDictated };
}
