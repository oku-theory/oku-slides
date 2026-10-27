/**
 * A theme tab's header: the theme's name and save state, centred above the stage, and
 * "Finish" at the top right, which closes the tab (the close flow saves it first).
 */
import type { ThemeTabView } from "./packTypes";
import { t } from "./i18n";
import { Icon } from "./icons";

interface Props {
  /** The theme tab's view: its cleaned name. */
  theming: ThemeTabView;
  /** The draft has changes not saved yet. */
  dirty: boolean;
  /** Closes the tab; absent: the last tab stays, no finish button. */
  onFinish?: () => void;
}

export default function ThemeTabHeader({ theming, dirty, onFinish }: Props) {
  return (
    <>
      <header className="label editing-pack">
        <span className="brand">{t("ui-tab-theme")}</span>
        <span className="doc-name">{theming.display_name}</span>
        <span className="caption" role="status">{t(dirty ? "ui-status-unsaved" : "ui-status-saved")}</span>
      </header>
      {onFinish && (
        <div className="tray">
          <button className="object token" aria-label={t("ui-toolbar-finish")} title={t("ui-toolbar-finish")} onClick={onFinish}>
            {Icon.check}
          </button>
        </div>
      )}
    </>
  );
}
