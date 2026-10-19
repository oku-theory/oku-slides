// Le brouillon des dictées dans le panneau de droite : on relit, retouche ou
// complète ce qu'on a dit, puis on l'envoie à l'agent comme consigne.
// Il se lit sur une carte ; un clic l'ouvre en édition.
// Ouvert au clavier (⌘K), il s'ouvre en édition pour écrire sans dicter.

import { isShortcut } from "./agent/shortcut";
import { useEffect, useRef, useState } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";

/** Le brouillon, son envoi et son abandon ; rien sans brouillon ouvert. */
export function DictationDraft({
  text,
  dictations,
  typing,
  busy,
  notice = null,
  ready,
  shortcuts,
  onChange,
  onSend,
  onDiscard,
}: {
  /** `null` : pas de brouillon ouvert. */
  text: string | null;
  /** Nombre de dictées reçues : chacune ramène le curseur en fin de brouillon. */
  dictations: number;
  /** Nombre de demandes d'écriture au clavier : chacune ouvre le champ, curseur en fin. */
  typing: number;
  /** Un tour de l'agent est en cours : l'envoi attend sa fin. */
  busy: boolean;
  /** « L'agent répond ailleurs » : l'envoi attend, la note le dit. */
  notice?: string | null;
  /** Un agent répond : sans lui, rien ne s'envoie (le texte reste au brouillon). */
  ready: boolean;
  /** Mode deck de l'onglet principal : seul cas où l'hôte traite un mot-raccourci sans agent ni tour. */
  shortcuts: boolean;
  onChange: (text: string) => void;
  onSend: (text: string) => void;
  onDiscard: () => void;
}) {
  const area = useRef<HTMLTextAreaElement>(null);
  const note = useRef<HTMLDivElement>(null);
  const [editing, setEditing] = useState(false);
  // À chaque dictée, le focus revient au brouillon : ⌘↩ aussitôt.
  useEffect(() => {
    if (editing) toEnd(area.current);
    else note.current?.focus();
  }, [dictations]);
  // Une demande d'écriture ouvre le champ (l'effet suivant y met le curseur).
  useEffect(() => {
    if (typing === 0) return;
    if (editing) toEnd(area.current);
    else setEditing(true);
  }, [typing]);
  // Ouvert en édition : curseur en fin de texte. Refermé : le focus reste sur la carte.
  const opened = useRef(false);
  useEffect(() => {
    if (editing) toEnd(area.current);
    else if (opened.current) note.current?.focus();
    opened.current = editing;
  }, [editing]);

  if (text === null) return null;
  const said = text.trim();
  // Quitter un brouillon resté vide le referme : il n'y a rien à garder.
  const leave = () => {
    if (said === "") onDiscard();
    else setEditing(false);
  };
  // Un mot-raccourci (« thème ») part toujours en mode deck : l'hôte l'ouvre sans agent ni tour.
  // Ailleurs il irait à l'agent : mêmes garde-fous que tout texte.
  const canSend = said !== "" && ((shortcuts && isShortcut(said)) || (!busy && !notice && ready));
  const send = () => {
    if (canSend) onSend(said);
  };
  return (
    <section
      className="draft card prompt"
      aria-label={t("ui-draft")}
      onKeyDown={(e) => {
        if (e.key !== "Enter" || !(e.metaKey || e.ctrlKey)) return;
        // ⌘⇧↩ ne présente pas le deck depuis le brouillon : seul ⌘↩ y fait quelque chose.
        e.preventDefault();
        e.stopPropagation();
        if (!e.shiftKey) send();
      }}
    >
      {editing ? (
        <textarea
          ref={area}
          className="draft-text"
          value={text}
          rows={3}
          onChange={(e) => onChange(e.target.value)}
          onBlur={leave}
          onKeyDown={(e) => {
            if (e.key !== "Escape") return;
            e.stopPropagation();
            leave();
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
      {notice && <p className="caption draft-notice" role="status">{notice}</p>}
      <div className="draft-actions">
        <button type="button" className="token ghost draft-discard" aria-label={t("ui-draft-discard")} title={t("ui-draft-discard")} onClick={onDiscard}>{Icon.trash}</button>
        <button type="button" className="object token primary draft-send" disabled={!canSend} aria-label={t("ui-draft-send")} title={t("ui-draft-send-title")} onClick={send}>
          {Icon.send}
        </button>
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
