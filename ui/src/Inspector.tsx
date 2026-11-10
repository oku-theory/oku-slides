// The inspector: the selection's settings, or the deck's.

import type { ReactNode } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "./api";
import HostSettings from "./HostSettings";
import { t } from "./i18n";
import { Button } from "./primitives";
import type { HostSettings as Settings } from "./useHostSettings";

/** What the inspector shows and what it triggers. */
export interface InspectorProps {
  /** The selected element's `host-element` settings; no surface: nothing selected, the deck shows. */
  element: Settings;
  /** The selected element is a chart or a table: a Data button opens its data pane. */
  data: boolean;
  /** Opens the data pane. */
  onData(): void;
  /** The deck's theme name, shown without a selection; `null`: no theme. */
  theme: string | null;
  /** An agent offer opened from its chip, framed before the element's settings. */
  offer?: ReactNode;
  /** A surface action of the inspector; `true` if it was taken. */
  onAction(s: Surface, a: A2uiClientAction): boolean;
  onError(e: unknown): void;
}

/** The inspector tab of a deck tab's side panel; the panel's rail opens and folds it. */
export function Inspector({ element, data, onData, theme, offer, onAction, onError }: InspectorProps) {
  return (
    <section className="inspector" aria-label={t("ui-inspector")}>
      <header className="inspector-head">
        <span className="pane-title">{t("ui-inspector")}</span>
      </header>
      {offer && (
        <section className="inspector-offer" aria-label={t("ui-inspector-offer")}>
          {offer}
        </section>
      )}
      {element.surface ? (
        <>
          <HostSettings surface={element.surface} outcome={element.outcome} onAction={onAction} onError={onError} />
          {data && <Button className="inspector-data pill" onClick={onData}>{t("ui-inspector-data")}</Button>}
        </>
      ) : (
        <div className="inspector-deck">
          <p className="inspector-theme">{theme ?? t("ui-inspector-no-theme")}</p>
        </div>
      )}
    </section>
  );
}
