// The data pane: the selected chart's or table's data, under the stage.

import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Surface } from "./api";
import { dataPaneBounds, sheetRows } from "./dataPaneSize";
import Divider from "./Divider";
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
  /** The height kept for the pane, in px; `null`: its content's, up to 40% of the window. */
  height: number | null;
  /** Keeps a height set at the divider; `null` on a double-click: the default height. */
  onHeight(height: number | null): void;
}

/**
 * The data pane of a deck tab, a wide row under the stage. Its divider sets the sheet's data row
 * (`--data-h` on the sheet, its parent), live while dragging; maximized, the pane takes all the
 * stage's room but `--stage-min`, until restored or resized; both end when the surface goes.
 */
export function DataPane({ data, onAction, onError, onClose, height, onHeight }: DataPaneProps) {
  const pane = useRef<HTMLElement>(null);
  const [live, setLive] = useState<number | null>(null);
  const [max, setMax] = useState(false);
  const shown = live ?? height;
  // Without a surface (a selection without data, or the chart removed mid-drag), the next one opens
  // at the kept height, not maximized. Not on every new surface: an edit sends a new one.
  useEffect(() => {
    if (data.surface) return;
    setLive(null);
    setMax(false);
  }, [data.surface]);

  useLayoutEffect(() => {
    const sheet = pane.current?.parentElement;
    if (!sheet) return;
    if (shown === null) sheet.style.removeProperty("--data-h");
    else sheet.style.setProperty("--data-h", `${shown}px`);
    return () => void sheet.style.removeProperty("--data-h");
  }, [shown, data.surface]);

  if (!data.surface) return null;
  const measure = () => {
    const sheet = pane.current?.parentElement;
    if (!sheet) return null;
    const style = getComputedStyle(sheet);
    const rows = sheetRows(style.gridTemplateRows, style.getPropertyValue("--stage-min"));
    return { size: rows.pane, bounds: dataPaneBounds(rows) };
  };
  const classes = ["data-pane", shown !== null && "sized", max && "max"].filter(Boolean).join(" ");
  return (
    <section ref={pane} className={classes} aria-label={t("ui-data-pane")}>
      <Divider
        className="row-divider"
        label={t("ui-data-pane-resize")}
        axis="y"
        grow={-1}
        measure={measure}
        onResize={(size, done) => {
          setMax(false);
          setLive(done ? null : size);
          if (done) onHeight(size);
        }}
        onReset={() => {
          setMax(false);
          onHeight(null);
        }}
      />
      <header className="data-pane-head">
        <span className="pane-title">{t("ui-data-pane")}</span>
        <span className="data-pane-tools">
          <IconButton
            className="token ghost data-pane-max"
            label={t(max ? "ui-data-pane-restore" : "ui-data-pane-maximize")}
            icon={max ? Icon.paneRestore : Icon.paneMaximize}
            onClick={() => setMax(!max)}
          />
          <IconButton className="token ghost" label={t("ui-data-pane-close")} icon={Icon.close} onClick={onClose} />
        </span>
      </header>
      <div className="data-pane-body">
        <HostSettings surface={data.surface} outcome={data.outcome} onAction={onAction} onError={onError} />
      </div>
    </section>
  );
}
