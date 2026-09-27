// Le brouillon des dictées dans le panneau de droite : on relit, retouche ou
// complète ce qu'on a dit, puis on choisit — l'insérer tel quel ou l'envoyer
// à l'agent comme consigne.

import { useEffect, useRef } from "react";
import { t } from "./i18n";

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
  // À chaque dictée, le curseur va en fin de brouillon : ⌘↩ ou ⌘⇧↩ aussitôt.
  useEffect(() => {
    const el = area.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
  }, [dictations]);

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
    <section className="draft" aria-label={t("ui-draft")}>
      <textarea
        ref={area}
        className="draft-text"
        value={text}
        rows={3}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== "Enter" || !(e.metaKey || e.ctrlKey)) return;
          e.preventDefault();
          if (e.shiftKey) insert();
          else send();
        }}
      />
      <div className="draft-actions">
        <button type="button" className="linkish draft-discard" onClick={onDiscard}>{t("ui-draft-discard")}</button>
        <span className="draft-exits">
          <button type="button" className="btn ghost draft-insert" disabled={!canInsert} title={t("ui-draft-insert-title")} onClick={insert}>
            {t("ui-draft-insert")}
          </button>
          <button type="button" className="btn draft-send" disabled={!canSend} title={t("ui-draft-send-title")} onClick={send}>
            {t("ui-draft-send")}
          </button>
        </span>
      </div>
      <span className="caption draft-insert-to">→ {insertTo ?? t("ui-dictation-select-zone")}</span>
    </section>
  );
}
