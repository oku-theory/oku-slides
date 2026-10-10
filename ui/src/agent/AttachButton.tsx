// Le bouton qui ramène l'agent détaché dans la fenêtre principale.

import { t } from "../i18n";
import { Icon } from "../icons";

/** « Rattacher », icône seule : dans la fenêtre de l'agent, et en recours dans la fenêtre principale. */
export function AttachButton({ onClick, className = "" }: { onClick: () => void; className?: string }) {
  return (
    <button type="button" className={`token ghost agent-attach ${className}`.trim()} aria-label={t("ui-agent-attach")} title={t("ui-agent-attach")} onClick={onClick}>
      {Icon.attach}
    </button>
  );
}
