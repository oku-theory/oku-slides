/** Bouton à bascule « Code source » d'un sous-onglet de widget, à droite d'annuler / rétablir. */
import { t } from "./i18n";
import { Icon } from "./icons";

interface Props {
  /** Code affiché (bouton enfoncé). */
  checked: boolean;
  /** Id de l'éditeur du sous-onglet actif (`widget-code-<id>`). */
  controls: string;
  onChange: (checked: boolean) => void;
}

export default function CodeSwitch({ checked, controls, onChange }: Props) {
  const label = t("ui-widgets-source");
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
      {Icon.code}
    </button>
  );
}
