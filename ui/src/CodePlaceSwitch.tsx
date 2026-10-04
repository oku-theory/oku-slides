/** Bouton à bascule « Code à droite » d'un sous-onglet de widget, à droite du bouton du code source. */
import { t } from "./i18n";
import { Icon } from "./icons";

interface Props {
  /** Code à droite de l'aperçu (bouton enfoncé) ; relâché, il est dessous. */
  checked: boolean;
  /** Id de l'éditeur du sous-onglet actif (`widget-code-<id>`). */
  controls: string;
  onChange: (checked: boolean) => void;
}

export default function CodePlaceSwitch({ checked, controls, onChange }: Props) {
  const label = t("ui-widgets-code-right");
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
      {Icon.codeRight}
    </button>
  );
}
