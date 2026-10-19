// Panneau des agents : choisir l'agent ACP de l'app (surface `agent-picker` de l'hôte).

import { lazy, Suspense } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "../api";
import { fromA2ui } from "./quick";

// Même morceau que le fil de l'agent : chargé une fois.
const SurfaceView = lazy(() => import("../a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

interface Props {
  surface: Surface | null;
  onSelect: (id: string) => void;
  /** Le bouton « Télécharger le modèle » de l'agent local. */
  onDownload: () => void;
  /** Le bouton « Annuler » du téléchargement en cours. */
  onCancelDownload: () => void;
  onError: (message: string) => void;
}

/**
 * Une section du menu des réglages de la dictée (la surface porte son titre). La classe
 * `.agent-picker` garde ses touches loin de la question ouverte (`useQuickKeys`). Sans surface
 * (pas encore reçue), rien.
 */
export function AgentPicker({ surface, onSelect, onDownload, onCancelDownload, onError }: Props) {
  if (!surface) return null;
  const onAction = (a: A2uiClientAction) => {
    const action = fromA2ui(a);
    const id = action.context.id;
    if (action.name === "select" && typeof id === "string") onSelect(id);
    else if (action.name === "download_model") onDownload();
    else if (action.name === "cancel_download") onCancelDownload();
  };
  return (
    <div className="agent-picker">
      <Suspense fallback={null}>
        <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={onAction} onError={onError} />
      </Suspense>
    </div>
  );
}
