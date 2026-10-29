/** The "Source code" toggle of a pack tab's selected item, right of undo and redo. */
import { t } from "./i18n";
import { Icon } from "./icons";

/** The switch's state and the editor it shows. */
interface Props {
  /** Code shown (button pressed). */
  checked: boolean;
  /** The selected item's editor id (`widget-code`); absent while no item is selected. */
  controls?: string;
  onChange: (checked: boolean) => void;
}

/** Shows or hides the selected item's source code. */
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
