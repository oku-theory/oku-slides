// Bandeau du pack : le deck embarque une autre version que ma version courante. La session
// le décrit en surface A2UI (`Notice`) ; il ne bloque rien et se ferme pour la session.
// Une rangée sur toute la largeur de la feuille, sous le titre.

import { lazy, Suspense, useRef } from "react";
import type { Surface } from "./api";
import { type PackNoticeCommand, packNoticeCommand } from "./packNoticeCommand";

// Même morceau que le fil de l'agent : chargé une fois.
const SurfaceView = lazy(() => import("./a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

/**
 * Les messages de `surface`, gardés tant que leur contenu ne change pas : chaque snapshot en
 * apporte une copie neuve, que `SurfaceView` reconstruirait (focus perdu, statut réannoncé).
 */
function useStableMessages(surface: Surface) {
  const json = JSON.stringify(surface.messages);
  const kept = useRef({ json, messages: surface.messages });
  if (kept.current.json !== json) kept.current = { json, messages: surface.messages };
  return kept.current.messages;
}

export default function PackNotice({ surface, onCommand, onError }: {
  surface: Surface;
  /** Envoie la requête d'un bouton du bandeau. */
  onCommand: (command: PackNoticeCommand) => void;
  onError: (message: string) => void;
}) {
  const messages = useStableMessages(surface);
  return (
    <div className="pack-notice">
      <Suspense fallback={null}>
        <SurfaceView
          surfaceId={surface.id}
          messages={messages}
          onAction={(a) => {
            const command = packNoticeCommand(a.name);
            // Un évènement inconnu est un écart de contrat avec la session : signalé, jamais envoyé.
            if (command) onCommand(command);
            else console.error(`unknown pack notice action: ${a.name}`);
          }}
          onError={onError}
        />
      </Suspense>
    </div>
  );
}
