/**
 * Une surface de réglages de l'hôte, en fiche : en mode modèles (« Ce modèle », « Paramètres de
 * <widget> »), posée sous l'aperçu qu'elle règle plutôt qu'au dock de l'agent ; ou les paramètres
 * de la cible, sous la scène (`TargetPanel`), précédés dans la même fiche des réglages du modèle
 * (`lead`). La surface et le routage de ses actions viennent de `useHostSettings` ou
 * `useTargetSettings` ; elle est reconstruite à chaque relecture, donc pas annoncée.
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
  /** Section posée en tête de la même fiche (les réglages du modèle, au-dessus de la cible). */
  lead?: SettingsSection;
}

export default function HostSettings({ surface, outcome, onAction, onError, lead }: Props) {
  const sections = lead ? [lead, { surface, outcome }] : [{ surface, outcome }];
  return (
    <div className={`host-settings${lead ? " merged" : ""}`} aria-live="off">
      {sections.map((s) => (
        <div key={s.surface.id} className="host-settings-section" data-surface-id={s.surface.id}>
          <Suspense fallback={null}>
            <ActionOutcomeContext.Provider value={s.outcome}>
              <SurfaceView surfaceId={s.surface.id} messages={s.surface.messages} onAction={(a) => onAction(s.surface, a)} onError={onError} />
            </ActionOutcomeContext.Provider>
          </Suspense>
        </div>
      ))}
    </div>
  );
}
