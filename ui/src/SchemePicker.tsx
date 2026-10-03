// Choix du schéma de l'interface dans le menu : celui du système, clair ou sombre.

import { t } from "./i18n";
import type { SchemeChoice } from "./scheme";

/** Liste du menu : `null` suit le système. */
export default function SchemePicker({ value, onChange }: { value: SchemeChoice; onChange: (choice: SchemeChoice) => void }) {
  return (
    <label className="menu-theme">
      <span className="caption">{t("ui-scheme")}</span>
      <select
        className="select"
        value={value ?? ""}
        onChange={(e) => onChange(e.target.value === "light" || e.target.value === "dark" ? e.target.value : null)}
      >
        <option value="">{t("ui-scheme-system")}</option>
        <option value="light">{t("ui-scheme-light")}</option>
        <option value="dark">{t("ui-scheme-dark")}</option>
      </select>
    </label>
  );
}
