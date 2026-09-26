// Panneau de droite : la surface A2UI que décrit la session, rendue avec le
// catalogue Papier. Chaque nouvelle liste de messages recrée la surface.

import { useMemo, useRef } from "react";
import { A2uiSurface } from "@a2ui/react/v0_9";
import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import { panelCatalog } from "./catalog";

/** Identifiant de la surface du panneau (PANEL_SURFACE côté session). */
const PANEL_SURFACE = "panel";

export function PanelSurface({
  messages,
  onAction,
  onError,
}: {
  messages: A2uiMessage[];
  onAction: (action: A2uiClientAction) => void;
  onError: (message: string) => void;
}) {
  // Les gestionnaires changent à chaque rendu ; la surface, seulement avec les messages.
  const handlers = useRef({ onAction, onError });
  handlers.current = { onAction, onError };
  const surface = useMemo(() => {
    if (messages.length === 0) return null;
    const processor = new MessageProcessor([panelCatalog], (action) => handlers.current.onAction(action));
    try {
      processor.processMessages(messages);
    } catch (e) {
      handlers.current.onError(String(e));
      return null;
    }
    return processor.model.surfacesMap.get(PANEL_SURFACE) ?? null;
  }, [messages]);
  return surface ? <A2uiSurface surface={surface} /> : null;
}
