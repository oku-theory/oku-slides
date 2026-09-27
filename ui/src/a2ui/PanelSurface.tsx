// Panneau de droite : la surface A2UI que décrit la session.

import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import { SurfaceView } from "./SurfaceView";

/** Identifiant de la surface du panneau (PANEL_SURFACE côté session). */
const PANEL_SURFACE = "panel";

/** Le panneau de droite, rendu par `SurfaceView`. */
export function PanelSurface(props: {
  messages: A2uiMessage[];
  onAction: (action: A2uiClientAction) => void;
  onError: (message: string) => void;
}) {
  return <SurfaceView surfaceId={PANEL_SURFACE} {...props} />;
}
