// Titre de la présentation dans l'en-tête de l'onglet : un bouton qui
// devient une saisie au clic. Ses touches restent les siennes (elles ne
// déclenchent pas les raccourcis de l'app).

import { useState } from "react";
import { t } from "./i18n";

/** Titre de la présentation, modifiable sur place (clic, Entrée pour valider, Échap pour annuler). */
export default function TitleField({ value, display, onCommit }: { value: string; display: string; onCommit: (v: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  if (!editing) {
    return (
      <button
        className="doc-name doc-name-button"
        title={t("ui-title-rename")}
        onClick={() => { setDraft(value); setEditing(true); }}
      >
        {display}
      </button>
    );
  }
  const commit = () => {
    setEditing(false);
    if (draft !== value) onCommit(draft);
  };
  return (
    <input
      className="doc-name doc-name-input"
      aria-label={t("ui-title-field")}
      autoFocus
      value={draft}
      size={Math.max(12, draft.length + 1)}
      onFocus={(e) => e.currentTarget.select()}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") { e.preventDefault(); commit(); }
        else if (e.key === "Escape") { e.preventDefault(); setEditing(false); }
        e.stopPropagation();
      }}
    />
  );
}
