// Sélecteur d'un thème : la bibliothèque, plus le thème montré s'il n'en fait pas partie
// (thème du deck au menu ⋯, base du brouillon dans l'onglet « Thème »).

import type { ThemeInfo, ThemeRef } from "./api";
import { t } from "./i18n";

interface Props {
  /** Thème embarqué par le deck. */
  theme: ThemeRef | null;
  /** Nom du thème embarqué, quand le deck le fournit. */
  themeName: string | null;
  /** Thèmes de la bibliothèque. */
  themes: ThemeInfo[];
  onPick: (id: string) => void;
  /** Étiquette ; par défaut celle du menu ⋯. */
  label?: string;
}

export default function ThemeMenu({ theme, themeName, themes, onPick, label }: Props) {
  // Thème reçu dans le fichier d'un autre : il reste affiché, sans pouvoir être rechoisi.
  const foreign = theme !== null && !themes.some((th) => th.id === theme.id);
  return (
    <label className="menu-theme">
      <span className="caption">{label ?? t("ui-toolbar-theme")}</span>
      <select className="select" value={theme?.id ?? ""} onChange={(e) => onPick(e.target.value)}>
        {foreign && <option value={theme.id} disabled>{themeName ?? t("ui-toolbar-theme-embedded")}</option>}
        {themes.map((th) => <option key={th.id} value={th.id}>{th.name}</option>)}
      </select>
    </label>
  );
}
