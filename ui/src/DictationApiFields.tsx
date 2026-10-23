import { useEffect, useState } from "react";
import type { SttApiConfig } from "./api";
import { t } from "./i18n";
import { Field } from "./primitives";

const FIELDS: [keyof SttApiConfig, string][] = [
  ["url", "ui-dictation-api-url"],
  ["model", "ui-dictation-api-model"],
  ["key_env", "ui-dictation-api-key-env"],
];

/** Réglages du service de dictée par API : adresse, modèle et variable d'environnement de la clé. */
export function DictationApiFields({ api, onChange }: { api: SttApiConfig; onChange: (api: SttApiConfig) => void }) {
  const [draft, setDraft] = useState(api);
  // L'hôte reste la source : ses valeurs reviennent après chaque réglage.
  useEffect(() => setDraft(api), [api.url, api.model, api.key_env]);

  const commit = (key: keyof SttApiConfig) => {
    if (draft[key] !== api[key]) onChange({ ...api, [key]: draft[key] });
  };

  return (
    <>
      {FIELDS.map(([key, label]) => (
        <Field key={key} variant="row" label={t(label)}>
          <input
            value={draft[key]}
            spellCheck={false}
            onChange={(e) => setDraft({ ...draft, [key]: e.target.value })}
            onBlur={() => commit(key)}
            // Les raccourcis de l'app ne voient pas la saisie.
            onKeyDown={(e) => {
              if (e.key === "Enter") commit(key);
              e.stopPropagation();
            }}
          />
        </Field>
      ))}
      <p className="caption">{t("ui-dictation-api-note")}</p>
    </>
  );
}
