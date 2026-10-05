// Séparateur vertical entre l'aperçu d'un widget et son code, à sa droite :
// on le glisse pour régler la largeur du code, on le double-clique pour
// revenir à la proportion par défaut.
//
// Glisser fait à la main (événements de pointeur), comme ColumnDivider : dans
// l'app Tauri, la webview réserve le glisser-déposer HTML5 au dépôt de fichiers.

import { useRef, useState } from "react";
import { fitCode } from "./codeSplit";

interface Props {
  label: string;
  /** Largeur du sous-onglet et largeur du code, lues au début d'un geste. */
  measure: () => { total: number; size: number };
  /** Largeur bornée : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
  onResize: (size: number, done: boolean) => void;
  /** Double-clic : proportion par défaut. */
  onReset: () => void;
}

/** Pas d'une flèche du clavier, en px. */
const STEP = 16;

interface Drag {
  pointer: number;
  start: number;
  measured: ReturnType<Props["measure"]>;
  /** +1 si aller vers la droite agrandit le code, -1 sinon. */
  sign: number;
  last: number;
  /** Le pointeur s'est déplacé : sans cela, un simple clic (ou ceux d'un double-clic) ne règle rien. */
  moved: boolean;
}

export default function CodeDivider({ label, measure, onResize, onReset }: Props) {
  const drag = useRef<Drag | null>(null);
  const [active, setActive] = useState(false);

  /** Le code est en fin de ligne : à droite, sauf de droite à gauche. Avancer vers lui le réduit. */
  const signOf = (el: HTMLElement) => (getComputedStyle(el).direction === "rtl" ? 1 : -1);

  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const measured = measure();
    // Pas de code mesuré : rien à régler.
    if (measured.total <= 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { pointer: e.pointerId, start: e.clientX, measured, sign: signOf(e.currentTarget), last: measured.size, moved: false };
    setActive(true);
  };

  const move = (e: React.PointerEvent<HTMLDivElement>, done: boolean) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    if (e.clientX !== d.start) d.moved = true;
    if (done) end();
    if (!d.moved) return;
    d.last = fitCode(d.measured.total, d.measured.size + d.sign * (e.clientX - d.start));
    onResize(d.last, done);
  };

  /** Geste interrompu : la largeur reste là où il s'est arrêté. */
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
    // Comme le pointeur : +1 vers la droite.
    const dir = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!dir) return;
    e.preventDefault();
    const m = measure();
    if (m.total <= 0) return;
    onResize(fitCode(m.total, m.size + signOf(e.currentTarget) * dir * STEP), true);
  };

  return (
    <div
      className={`code-divider${active ? " active" : ""}`}
      role="separator"
      aria-orientation="vertical"
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
