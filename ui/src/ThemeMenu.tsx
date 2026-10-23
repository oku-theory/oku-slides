// Sélecteur d'un thème de la bibliothèque, par espace de noms. Il montre aussi le thème épinglé
// qui n'en fait plus partie (reçu dans le fichier d'un autre, ou base d'un brouillon).

import type { ThemeInfo } from "./api";
import { t } from "./i18n";
import { Field } from "./primitives";

interface Props {
  /** Espace de noms du thème montré. */
  theme: string | null;
  /** Nom du thème montré, quand le deck le fournit. */
  themeName: string | null;
  /** Thèmes de la bibliothèque. */
  themes: ThemeInfo[];
  /** Reçoit l'espace de noms choisi. */
  onPick: (namespace: string) => void;
  /** Étiquette du champ. */
  label: string;
}

export default function ThemeMenu({ theme, themeName, themes, onPick, label }: Props) {
  // Thème absent de la bibliothèque : il reste affiché, sans pouvoir être rechoisi.
  const foreign = theme !== null && !themes.some((th) => th.namespace === theme);
  return (
    <Field variant="menu" label={label}>
      <select className="select" value={theme ?? ""} onChange={(e) => onPick(e.target.value)}>
        {foreign && <option value={theme} disabled>{themeName ?? t("ui-toolbar-theme-embedded")}</option>}
        {themes.map((th) => <option key={th.namespace} value={th.namespace}>{th.name}</option>)}
      </select>
    </Field>
  );
}
