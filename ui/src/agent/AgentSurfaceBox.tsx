// Le cadre commun des surfaces de l'agent (fiche, tiroir, fenêtre).

import { useRef } from "react";
import { QuickHintsContext } from "../a2ui/quickHints";
import { SurfaceView } from "../a2ui/SurfaceView";
import type { Surface } from "../api";
import type { QuickAction } from "./quick";
import { fromA2ui } from "./quick";
import { useModalFrame } from "../modalFrame";

/** Ce que reçoit une surface de l'agent : sa description et où envoyer ses actions. */
export interface SurfaceProps {
  surface: Surface;
  /** La surface reçoit les touches rapides (`useQuickKeys`) : ses chiffres 1–4 et le rappel des touches s'affichent. */
  hints: boolean;
  onAction: (surface: Surface, action: QuickAction) => void;
  onError: (message: string) => void;
}

/**
 * Une surface dans son cadre. Un clic ne remonte pas jusqu'à la scène (qui
 * désélectionnerait) ; la classe `agent-keys` fait ignorer ses touches par les
 * raccourcis de l'app. `modal` : le cadre de `useModalFrame` (focus pris,
 * gardé et rendu, clics des premières 500 ms ignorés : une fenêtre qui en
 * remplace une autre sous le curseur) ; Échap reste aux touches rapides.
 */
export function AgentSurfaceBox({
  surface,
  hints,
  onAction,
  onError,
  className,
  role,
  label,
  caption,
  modal = false,
}: SurfaceProps & { className: string; role: "dialog" | "alertdialog"; label: string; caption?: string; modal?: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const { onClickCapture } = useModalFrame(ref, { active: modal });

  return (
    <div
      ref={ref}
      tabIndex={-1}
      className={`${className} agent-keys`}
      role={role}
      aria-label={label}
      aria-modal={modal || undefined}
      data-surface-id={surface.id}
      onPointerDown={(e) => e.stopPropagation()}
      onClickCapture={onClickCapture}
    >
      <QuickHintsContext.Provider value={hints}>
        <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={(a) => onAction(surface, fromA2ui(a))} onError={onError} />
      </QuickHintsContext.Provider>
      {hints && caption && <p className="caption">{caption}</p>}
    </div>
  );
}
