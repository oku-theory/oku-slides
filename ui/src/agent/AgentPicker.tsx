// Panneau des agents : choisir l'agent ACP de l'app (surface `agent-picker` de l'hôte).

import { lazy, Suspense, useEffect, useRef } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { t } from "../i18n";
import type { Surface } from "../api";
import { fromA2ui } from "./quick";

// Même morceau que le fil de l'agent : chargé une fois.
const SurfaceView = lazy(() => import("../a2ui/SurfaceView").then((m) => ({ default: m.SurfaceView })));

interface Props {
  surface: Surface | null;
  onSelect: (id: string) => void;
  onClose: () => void;
  onError: (message: string) => void;
}

/** La fiche, sous l'en-tête du fil ; Échap la ferme. Sans surface (pas encore reçue), rien. */
export function AgentPicker({ surface, onSelect, onClose, onError }: Props) {
  const box = useRef<HTMLDivElement>(null);
  const shown = surface !== null;
  // Le focus va à la fiche quand elle apparaît : Échap la ferme sans passer par la souris.
  useEffect(() => {
    if (shown) box.current?.focus();
  }, [shown]);
  if (!surface) return null;
  const onAction = (a: A2uiClientAction) => {
    const action = fromA2ui(a);
    const id = action.context.id;
    if (action.name === "select" && typeof id === "string") onSelect(id);
  };
  return (
    <div
      ref={box}
      className="agent-picker"
      role="dialog"
      aria-label={t("ui-agent-picker")}
      tabIndex={-1}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          onClose();
        }
      }}
    >
      <Suspense fallback={null}>
        <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={onAction} onError={onError} />
      </Suspense>
    </div>
  );
}
