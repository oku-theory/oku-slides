// The inspector: the selection's settings, or the deck's.

import type { ReactNode } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "./api";
import HostSettings from "./HostSettings";
import { t } from "./i18n";
import { Icon } from "./icons";
import { Button, IconButton } from "./primitives";
import type { HostSettings as Settings } from "./useHostSettings";

/** What the inspector shows and what it triggers. */
export interface InspectorProps {
  /** The selected element's `host-element` settings; no surface: nothing selected, the deck shows. */
  element: Settings;
  /** The selected element is a chart or a table: a Data button opens its data pane. */
  data: boolean;
  /** Opens the data pane. */
  onData(): void;
  /** The deck, shown without a selection: its theme's name and the packs popover's opener. */
  deck: { theme: string | null; onPacks(): void };
  /** An agent offer opened from its chip, framed before the element's settings. */
  offer?: ReactNode;
  /** A surface action of the inspector; `true` if it was taken. */
  onAction(s: Surface, a: A2uiClientAction): boolean;
  onError(e: unknown): void;
  onClose(): void;
}

/** The inspector column of a deck tab, beside the stage. */
export function Inspector({ element, data, onData, deck, offer, onAction, onError, onClose }: InspectorProps) {
  return (
    <aside className="inspector" aria-label={t("ui-inspector")}>
      <header className="inspector-head">
        <span className="pane-title">{t("ui-inspector")}</span>
        <IconButton className="token ghost" label={t("ui-inspector-close")} icon={Icon.close} onClick={onClose} />
      </header>
      {offer && (
        <section className="inspector-offer" aria-label={t("ui-inspector-offer")}>
          {offer}
        </section>
      )}
      {element.surface ? (
        <>
          <HostSettings surface={element.surface} outcome={element.outcome} onAction={onAction} onError={onError} />
          {data && <Button raised className="inspector-data" onClick={onData}>{t("ui-inspector-data")}</Button>}
        </>
      ) : (
        <div className="inspector-deck">
          <p className="inspector-theme">{deck.theme ?? t("ui-inspector-no-theme")}</p>
          <Button raised aria-haspopup="dialog" data-popover-anchor="host-packs@inspector" onClick={deck.onPacks}>{t("ui-inspector-packs")}</Button>
        </div>
      )}
    </aside>
  );
}
