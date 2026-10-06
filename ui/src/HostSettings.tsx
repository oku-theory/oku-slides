/**
 * Une surface de réglages de l'hôte, en fiche : en mode modèles (« Ce modèle », « Paramètres de
 * <widget> »), posée sous l'aperçu qu'elle règle plutôt qu'au dock de l'agent ; ou les paramètres
 * de la cible, en tête des notes (`TargetPanel`). La surface et le routage de ses actions viennent
 * de `useHostSettings` ou `useTargetSettings` ; elle est reconstruite à chaque relecture, donc pas annoncée.
 */
import { lazy, Suspense } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { ActionOutcomeContext, type ActionOutcome } from "./a2ui/actionOutcome";
import type { Surface } from "./api";

// Même morceau que le fil de l'agent : chargé une fois.
const SurfaceView = lazy(() => import("./a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

interface Props {
  surface: Surface;
  onAction: (s: Surface, a: A2uiClientAction) => void;
  /** Issue des actions, posée par le routeur pendant leur envoi (`useSessionSurface`). */
  outcome: ActionOutcome;
  onError: (message: string) => void;
}

export default function HostSettings({ surface, onAction, outcome, onError }: Props) {
  return (
    <div className="host-settings" data-surface-id={surface.id} aria-live="off">
      <Suspense fallback={null}>
        <ActionOutcomeContext.Provider value={outcome}>
          <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={(a) => onAction(surface, a)} onError={onError} />
        </ActionOutcomeContext.Provider>
      </Suspense>
    </div>
  );
}
