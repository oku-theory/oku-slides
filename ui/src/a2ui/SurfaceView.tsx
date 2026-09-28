// Une surface A2UI (panneau de droite ou agent), rendue avec le catalogue Papier.
// Chaque nouvelle liste de messages recrée la surface ; l'ancienne est libérée.

import { useEffect, useRef, useState } from "react";
import { A2uiSurface } from "@a2ui/react/v0_9";
import type { A2uiClientAction, A2uiMessage, SurfaceModel } from "@a2ui/web_core/v0_9";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import { catalog } from "./catalog";

/** Une surface construite, ou l'erreur de validation de ses messages. */
export interface Built {
  surface: SurfaceModel<never> | null;
  error: string | null;
}

/** Construit la surface `surfaceId` à partir de `messages` ; n'émet rien, ne lève rien. */
export function buildSurface(
  messages: A2uiMessage[],
  surfaceId: string,
  onAction: (action: A2uiClientAction) => void,
): Built {
  if (messages.length === 0) return { surface: null, error: null };
  const processor = new MessageProcessor([catalog], onAction);
  try {
    processor.processMessages(messages);
  } catch (e) {
    return { surface: null, error: String(e) };
  }
  return { surface: (processor.model.surfacesMap.get(surfaceId) as SurfaceModel<never> | undefined) ?? null, error: null };
}

/** Rend la surface `surfaceId` décrite par `messages` ; une surface invalide va à `onError`. */
export function SurfaceView({
  messages,
  surfaceId,
  onAction,
  onError,
}: {
  messages: A2uiMessage[];
  surfaceId: string;
  onAction: (action: A2uiClientAction) => void;
  onError: (message: string) => void;
}) {
  // Les gestionnaires changent à chaque rendu ; la surface, seulement avec les messages.
  const handlers = useRef({ onAction, onError });
  handlers.current = { onAction, onError };
  // Construite et libérée par le même effet : un remontage (StrictMode) en
  // reconstruit une, au lieu de réafficher celle que le démontage a vidée.
  const [surface, setSurface] = useState<SurfaceModel<never> | null>(null);
  useEffect(() => {
    const built = buildSurface(messages, surfaceId, (action) => handlers.current.onAction(action));
    // Signalée après le rendu : le parent peut mettre son état à jour sans avertissement de React.
    if (built.error) handlers.current.onError(built.error);
    setSurface(built.surface);
    return () => built.surface?.dispose();
  }, [messages, surfaceId]);
  // `.a2ui-surface` : les raccourcis de l'app ignorent les touches tapées dans une surface.
  return surface ? (
    <div className="a2ui-surface">
      <A2uiSurface surface={surface} />
    </div>
  ) : null;
}
