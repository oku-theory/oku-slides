/**
 * Bouton à bascule Scène / Planche d'un sous-onglet de widget, à côté du
 * bouton du code source : enfoncé, tous les cas en vignettes (planche) ;
 * relâché, le cas courant en grand (scène).
 */
import { t } from "./i18n";
import { Icon } from "./icons";

interface Props {
  /** Planche montrée (bouton enfoncé). */
  checked: boolean;
  /** Id du panneau du sous-onglet actif (`widget-panel-<id>`). */
  controls: string;
  onChange: (checked: boolean) => void;
}

export default function BoardSwitch({ checked, controls, onChange }: Props) {
  const label = t("ui-widgets-board");
  return (
    <button
      type="button"
      className={`token ghost code-switch${checked ? " on" : ""}`}
      aria-label={label}
      title={label}
      aria-pressed={checked}
      aria-controls={controls}
      onClick={() => onChange(!checked)}
    >
      {Icon.board}
    </button>
  );
}
