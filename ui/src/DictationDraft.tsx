// Le brouillon des dictées dans le panneau de droite : on relit, retouche ou
// complète ce qu'on a dit, puis on choisit — l'insérer tel quel ou l'envoyer
// à l'agent comme consigne. Il se lit sur un post-it ; un clic l'ouvre en édition.

import { useEffect, useRef, useState } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";

/** Le brouillon et ses trois actions ; rien sans brouillon ouvert. */
export function DictationDraft({
  text,
  dictations,
  busy,
  insertTo,
  onChange,
  onInsert,
  onSend,
  onDiscard,
}: {
  /** `null` : pas de brouillon ouvert. */
  text: string | null;
  /** Nombre de dictées reçues : chacune ramène le curseur en fin de brouillon. */
  dictations: number;
  /** Un tour de l'agent est en cours : l'envoi attend sa fin, l'insertion non. */
  busy: boolean;
  /** Où « Insérer » écrira (« dans le titre »…) ; `null` : aucune zone choisie. */
  insertTo: string | null;
  onChange: (text: string) => void;
  onInsert: (text: string) => void;
  onSend: (text: string) => void;
  onDiscard: () => void;
}) {
  const area = useRef<HTMLTextAreaElement>(null);
  const note = useRef<HTMLDivElement>(null);
  const [editing, setEditing] = useState(false);
  // À chaque dictée, le focus revient au brouillon : ⌘↩ ou ⌘⇧↩ aussitôt.
  useEffect(() => {
    if (editing) toEnd(area.current);
    else note.current?.focus();
  }, [dictations]);
  // Ouvert en édition : curseur en fin de texte. Refermé : le focus reste sur le post-it.
  const opened = useRef(false);
  useEffect(() => {
    if (editing) toEnd(area.current);
    else if (opened.current) note.current?.focus();
    opened.current = editing;
  }, [editing]);

  if (text === null) return null;
  const said = text.trim();
  const canInsert = said !== "" && insertTo !== null;
  const canSend = said !== "" && !busy;
  const insert = () => {
    if (canInsert) onInsert(said);
  };
  const send = () => {
    if (canSend) onSend(said);
  };
  return (
    <section
      className="draft postit"
      aria-label={t("ui-draft")}
      onKeyDown={(e) => {
        if (e.key !== "Enter" || !(e.metaKey || e.ctrlKey)) return;
        e.preventDefault();
        e.stopPropagation();
        if (e.shiftKey) insert();
        else send();
      }}
    >
      {editing ? (
        <textarea
          ref={area}
          className="draft-text"
          value={text}
          rows={3}
          onChange={(e) => onChange(e.target.value)}
          onBlur={() => setEditing(false)}
          onKeyDown={(e) => {
            if (e.key !== "Escape") return;
            e.stopPropagation();
            setEditing(false);
          }}
        />
      ) : (
        <div
          ref={note}
          className="draft-note"
          role="button"
          tabIndex={0}
          onClick={() => setEditing(true)}
          onKeyDown={(e) => {
            if (e.key !== "Enter" || e.metaKey || e.ctrlKey) return;
            e.preventDefault();
            e.stopPropagation();
            setEditing(true);
          }}
        >
          {text}
        </div>
      )}
      <div className="draft-actions">
        <button type="button" className="token ghost draft-discard" aria-label={t("ui-draft-discard")} title={t("ui-draft-discard")} onClick={onDiscard}>{Icon.trash}</button>
        <span className="draft-exits">
          {/* Où ira le texte, juste avant le bouton qui l'y insère. */}
          <span className="caption draft-insert-to">→ {insertTo ?? t("ui-dictation-select-zone")}</span>
          <button type="button" className="token ghost draft-insert" disabled={!canInsert} aria-label={t("ui-draft-insert")} title={t("ui-draft-insert-title")} onClick={insert}>
            {Icon.insert}
          </button>
          <button type="button" className="object token primary draft-send" disabled={!canSend} aria-label={t("ui-draft-send")} title={t("ui-draft-send-title")} onClick={send}>
            {Icon.send}
          </button>
        </span>
      </div>
    </section>
  );
}

/** Focus sur le champ, curseur en fin de texte. */
function toEnd(el: HTMLTextAreaElement | null) {
  if (!el) return;
  el.focus();
  el.setSelectionRange(el.value.length, el.value.length);
}
