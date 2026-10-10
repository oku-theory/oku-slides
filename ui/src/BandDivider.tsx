// Séparateur horizontal entre la scène et la bande de réglages dessous : on le
// glisse pour régler la hauteur de la bande, on le double-clique pour revenir
// à la hauteur par défaut.
//
// Glisser fait à la main (événements de pointeur), comme CodeDivider : dans
// l'app Tauri, la webview réserve le glisser-déposer HTML5 au dépôt de fichiers.

import { useRef, useState } from "react";
import { fitBand } from "./bandSplit";

interface Props {
  label: string;
  /** Hauteur de la colonne (scène et bande) et hauteur de la bande, lues au début d'un geste. */
  measure: () => { total: number; size: number };
  /** Hauteur bornée : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
  onResize: (size: number, done: boolean) => void;
  /** Double-clic : hauteur par défaut. */
  onReset: () => void;
}

/** Pas d'une flèche du clavier, en px. */
const STEP = 16;

interface Drag {
  pointer: number;
  start: number;
  measured: ReturnType<Props["measure"]>;
  last: number;
  /** Le pointeur s'est déplacé : sans cela, un simple clic (ou ceux d'un double-clic) ne règle rien. */
  moved: boolean;
}

export default function BandDivider({ label, measure, onResize, onReset }: Props) {
  const drag = useRef<Drag | null>(null);
  const [active, setActive] = useState(false);

  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const measured = measure();
    // Rien de mesuré : rien à régler.
    if (measured.total <= 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { pointer: e.pointerId, start: e.clientY, measured, last: measured.size, moved: false };
    setActive(true);
  };

  const move = (e: React.PointerEvent<HTMLDivElement>, done: boolean) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    if (e.clientY !== d.start) d.moved = true;
    if (done) end();
    if (!d.moved) return;
    // La bande est sous le séparateur : monter l'agrandit.
    d.last = fitBand(d.measured.total, d.measured.size - (e.clientY - d.start));
    onResize(d.last, done);
  };

  /** Geste interrompu : la hauteur reste là où il s'est arrêté. */
  const cancel = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    end();
    if (d.moved) onResize(d.last, true);
  };

  const end = () => {
    drag.current = null;
    setActive(false);
  };

  const key = (e: React.KeyboardEvent<HTMLDivElement>) => {
    // Comme le pointeur : monter agrandit la bande.
    const dir = e.key === "ArrowUp" ? 1 : e.key === "ArrowDown" ? -1 : 0;
    if (!dir) return;
    e.preventDefault();
    const m = measure();
    if (m.total <= 0) return;
    onResize(fitBand(m.total, m.size + dir * STEP), true);
  };

  return (
    <div
      className={`band-divider${active ? " active" : ""}`}
      role="separator"
      aria-orientation="horizontal"
      aria-label={label}
      tabIndex={0}
      onPointerDown={down}
      onPointerMove={(e) => move(e, false)}
      onPointerUp={(e) => move(e, true)}
      onPointerCancel={cancel}
      onDoubleClick={onReset}
      onKeyDown={key}
    />
  );
}
