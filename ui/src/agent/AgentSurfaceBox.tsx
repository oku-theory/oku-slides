// Le cadre commun des surfaces de l'agent (fiche, tiroir, fenêtre).

import { useEffect, useRef } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import { SurfaceView } from "../a2ui/SurfaceView";
import type { Surface } from "../api";
import type { QuickAction } from "./quick";
import { fromA2ui } from "./quick";
import { ARMING_MS } from "./useQuickKeys";

/** Ce que reçoit une surface de l'agent : sa description et où envoyer ses actions. */
export interface SurfaceProps {
  surface: Surface;
  onAction: (surface: Surface, action: QuickAction) => void;
  onError: (message: string) => void;
}

const FOCUSABLE = "button:not([disabled]), input, select, textarea, [tabindex]:not([tabindex='-1'])";

/**
 * Une surface dans son cadre. Un clic ne remonte pas jusqu'à la scène (qui
 * désélectionnerait) ; la classe `agent-keys` fait ignorer ses touches par les
 * raccourcis de l'app. `modal` : le cadre prend le focus, le garde (Tab
 * tourne à l'intérieur), le rend en se fermant, et ignore les clics des
 * premières 500 ms (une fenêtre qui en remplace une autre sous le curseur).
 */
export function AgentSurfaceBox({
  surface,
  onAction,
  onError,
  className,
  role,
  label,
  caption,
  modal = false,
}: SurfaceProps & { className: string; role: "dialog" | "alertdialog"; label: string; caption?: string; modal?: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const openedAt = useRef(Date.now());
  useEffect(() => {
    if (!modal) return;
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => {
      // Rendre le focus seulement s'il est encore ici (ou perdu) et que l'élément existe toujours.
      const active = document.activeElement;
      if (previous?.isConnected && (!active || active === document.body || ref.current?.contains(active))) previous.focus();
    };
  }, [modal]);

  const onKeyDown = (e: KeyboardEvent) => {
    if (!modal || e.key !== "Tab") return;
    const items = [...(ref.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? [])];
    if (items.length === 0) return e.preventDefault();
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    if (e.shiftKey && (active === first || active === ref.current)) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  };
  const onClickCapture = (e: MouseEvent) => {
    if (modal && Date.now() - openedAt.current < ARMING_MS) {
      e.preventDefault();
      e.stopPropagation();
    }
  };

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
      onKeyDown={onKeyDown}
      onClickCapture={onClickCapture}
    >
      <SurfaceView surfaceId={surface.id} messages={surface.messages} onAction={(a) => onAction(surface, fromA2ui(a))} onError={onError} />
      {caption && <p className="caption">{caption}</p>}
    </div>
  );
}
