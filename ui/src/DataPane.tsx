// The data pane: the selected chart's or table's data, under the stage.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "./api";
import HostSettings from "./HostSettings";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconButton } from "./primitives";
import type { HostSettings as Settings } from "./useHostSettings";

/** What the data pane shows and what it triggers. */
export interface DataPaneProps {
  /** The selected chart's or table's `host-data` surface; none: the pane draws nothing. */
  data: Settings;
  /** A surface action of the pane; `true` if it was taken. */
  onAction(s: Surface, a: A2uiClientAction): boolean;
  onError(e: unknown): void;
  onClose(): void;
}

/** The data pane of a deck tab, a wide row under the stage. */
export function DataPane({ data, onAction, onError, onClose }: DataPaneProps) {
  if (!data.surface) return null;
  return (
    <section className="data-pane" aria-label={t("ui-data-pane")}>
      <header className="data-pane-head">
        <span className="pane-title">{t("ui-data-pane")}</span>
        <IconButton className="token ghost" label={t("ui-data-pane-close")} icon={Icon.close} onClick={onClose} />
      </header>
      <HostSettings surface={data.surface} outcome={data.outcome} onAction={onAction} onError={onError} />
    </section>
  );
}
