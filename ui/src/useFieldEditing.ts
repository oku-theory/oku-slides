// Le champ en édition sur la scène, retenu par sa clé.

import { useEffect, useRef, useState } from "react";
import type { SlideView } from "./api";
import type { StageHandle } from "./Stage";
import type { FieldKey } from "./stageFields";
import { fieldKey, findField, primaryFieldOf } from "./stageFields";

/**
 * Champ en édition (`editing`, sa clé ; `editField`, relu dans la vue).
 * `editWhenShown(owner)` ouvre le champ principal d'un objet dès qu'une vue le
 * montre : un élément qui vient d'être ajouté n'a pas encore de champ.
 * `editKeyWhenShown(key)` ouvre de même le champ `key` (champ d'un item de
 * liste qui vient d'être inséré).
 * `leaveSlide(to)` : avant de passer à la slide `to`, la saisie ouverte
 * s'enregistre (par la scène, poignée `stage`) et l'édition se ferme ; sans
 * cela la vue suivante n'a plus le champ et la saisie se perd. Sans effet si
 * `to` est la slide affichée. Lit la vue par une référence : un abonnement
 * posé une fois (évènements de l'hôte) peut l'appeler.
 */
export function useFieldEditing(view: SlideView | null) {
  const [editing, setKey] = useState<FieldKey | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [pendingKey, setPendingKey] = useState<FieldKey | null>(null);
  const stage = useRef<StageHandle>(null);
  const shown = useRef(view);
  shown.current = view;
  useEffect(() => {
    if (!pending || !view) return;
    const f = primaryFieldOf(view, pending);
    if (!f) return;
    setPending(null);
    setKey(fieldKey(f));
  }, [view, pending]);
  useEffect(() => {
    if (!pendingKey || !view || !findField(view, pendingKey)) return;
    setPendingKey(null);
    setKey(pendingKey);
  }, [view, pendingKey]);
  const setEditing = (key: FieldKey | null) => {
    setPending(null);
    setPendingKey(null);
    setKey(key);
  };
  const editKeyWhenShown = (key: FieldKey) => {
    setPending(null);
    setPendingKey(key);
  };
  const leaveSlide = (to: string) => {
    if (shown.current?.slide.id === to) return;
    stage.current?.commitEditing();
    setEditing(null);
  };
  const editField = view ? findField(view, editing) : null;
  return { editing, editField, setEditing, editWhenShown: setPending, editKeyWhenShown, stage, leaveSlide };
}
