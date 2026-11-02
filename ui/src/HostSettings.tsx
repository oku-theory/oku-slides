/**
 * A host settings surface, as a card: in pack mode (« Paramètres de <widget> »), under the preview
 * it sets rather than at the agent's dock; in a deck tab, the selected element's settings in the
 * inspector and its chart's or table's data in the data pane. The surface and the routing of its
 * actions come from `useHostSettings`, `useElementSettings`, `useDataSettings` or `useThemeSettings`;
 * it is rebuilt at each reread, so not announced.
 */
import { lazy, Suspense } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { ActionOutcomeContext, type ActionOutcome } from "./a2ui/actionOutcome";
import type { Surface } from "./api";

// Même morceau que le fil de l'agent : chargé une fois.
const SurfaceView = lazy(() => import("./a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

/** Une surface de la fiche et l'issue de ses actions. */
export interface SettingsSection {
  surface: Surface;
  /** Issue des actions, posée par le routeur pendant leur envoi (`useSessionSurface`). */
  outcome: ActionOutcome;
}

interface Props extends SettingsSection {
  onAction: (s: Surface, a: A2uiClientAction) => void;
  onError: (message: string) => void;
}

export default function HostSettings({ surface, outcome, onAction, onError }: Props) {
  return (
    <div className="host-settings" aria-live="off">
      <div className="host-settings-section" data-surface-id={surface.id}>
        <Suspense fallback={null}>
          <ActionOutcomeContext.Provider value={outcome}>
            <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={(a) => onAction(surface, a)} onError={onError} />
          </ActionOutcomeContext.Provider>
        </Suspense>
      </div>
    </div>
  );
}
