// Choix du schéma de l'interface dans le menu : celui du système, clair ou sombre.

import { t } from "./i18n";
import { Field, Select } from "./primitives";
import type { SchemeChoice } from "./scheme";

/** Liste du menu : `null` suit le système. */
export default function SchemePicker({ value, onChange }: { value: SchemeChoice; onChange: (choice: SchemeChoice) => void }) {
  return (
    <Field variant="menu" label={t("ui-scheme")}>
      <Select
        value={value ?? ""}
        onChange={(v) => onChange(v === "light" || v === "dark" ? v : null)}
        options={[
          { value: "", label: t("ui-scheme-system") },
          { value: "light", label: t("ui-scheme-light") },
          { value: "dark", label: t("ui-scheme-dark") },
        ]}
      />
    </Field>
  );
}
