// Sélecteur d'un thème pour le deck. Il propose la bibliothèque, ceux de la présentation (thème du
// deck seul d'un import), plus le thème montré s'il ne fait partie d'aucune des deux (thème du
// deck au menu ⋯, base du brouillon dans l'onglet « Thème »).

import type { ThemeInfo, ThemeRef } from "./api";
import { t } from "./i18n";

interface Props {
  /** Thème embarqué par le deck. */
  theme: ThemeRef | null;
  /** Nom du thème embarqué, quand le deck le fournit. */
  themeName: string | null;
  /** Thèmes de la bibliothèque. */
  themes: ThemeInfo[];
  /** Thèmes du deck seul embarqués dans la présentation : ils se reprennent comme les autres. */
  deckThemes: ThemeInfo[];
  onPick: (id: string) => void;
  /** Étiquette ; par défaut celle du menu ⋯. */
  label?: string;
}

export default function ThemeMenu({ theme, themeName, themes, deckThemes, onPick, label }: Props) {
  // Thème reçu dans le fichier d'un autre : il reste affiché, sans pouvoir être rechoisi.
  const known = (id: string) => themes.some((th) => th.id === id) || deckThemes.some((th) => th.id === id);
  const foreign = theme !== null && !known(theme.id);
  return (
    <label className="menu-theme">
      <span className="caption">{label ?? t("ui-toolbar-theme")}</span>
      <select className="select" value={theme?.id ?? ""} onChange={(e) => onPick(e.target.value)}>
        {foreign && <option value={theme.id} disabled>{themeName ?? t("ui-toolbar-theme-embedded")}</option>}
        {themes.map((th) => <option key={th.id} value={th.id}>{th.name}</option>)}
        {deckThemes.length > 0 && (
          <optgroup label={t("ui-toolbar-theme-deck-group")}>
            {deckThemes.map((th) => <option key={th.id} value={th.id}>{th.name}</option>)}
          </optgroup>
        )}
      </select>
    </label>
  );
}
