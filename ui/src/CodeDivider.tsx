// Séparateur entre l'aperçu d'un widget et son code : on le glisse pour régler
// la taille du code, on le double-clique pour revenir à la proportion par défaut.
// Horizontal quand le code est dessous, vertical quand il est à droite.
//
// Glisser fait à la main (événements de pointeur), comme ColumnDivider : dans
// l'app Tauri, la webview réserve le glisser-déposer HTML5 au dépôt de fichiers.

import { useRef, useState } from "react";
import { type CodeAxis, fitCode } from "./codeSplit";

interface Props {
  /** Axe affiché : orientation du séparateur. */
  axis: CodeAxis;
  label: string;
  /** Axe affiché, taille du sous-onglet sur cet axe et taille du code, lus au début d'un geste. */
  measure: () => { axis: CodeAxis; total: number; size: number };
  /** Taille bornée : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
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
  /** +1 si avancer sur l'axe (vers le bas, vers la droite) agrandit le code, -1 sinon. */
  sign: number;
  last: number;
}

/** Position du pointeur sur l'axe. */
const along = (axis: CodeAxis, e: { clientX: number; clientY: number }) => (axis === "below" ? e.clientY : e.clientX);

export default function CodeDivider({ axis, label, measure, onResize, onReset }: Props) {
  const drag = useRef<Drag | null>(null);
  const [active, setActive] = useState(false);

  /** Le code est en fin de ligne : à droite, sauf de droite à gauche. Avancer vers lui le réduit. */
  const signOf = (el: HTMLElement, a: CodeAxis) => (a === "right" && getComputedStyle(el).direction === "rtl" ? 1 : -1);

  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const measured = measure();
    drag.current = {
      pointer: e.pointerId,
      start: along(measured.axis, e),
      measured,
      sign: signOf(e.currentTarget, measured.axis),
      last: measured.size,
    };
    setActive(true);
  };

  const move = (e: React.PointerEvent<HTMLDivElement>, done: boolean) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    d.last = fitCode(d.measured.total, d.measured.size + d.sign * (along(d.measured.axis, e) - d.start));
    if (done) end();
    onResize(d.last, done);
  };

  /** Geste interrompu : la taille reste là où il s'est arrêté. */
  const cancel = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    end();
    onResize(d.last, true);
  };

  const end = () => {
    drag.current = null;
    setActive(false);
  };

  const key = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const m = measure();
    // Comme le pointeur : +1 en avançant sur l'axe (vers le bas, vers la droite).
    const [back, ahead] = m.axis === "below" ? ["ArrowUp", "ArrowDown"] : ["ArrowLeft", "ArrowRight"];
    const dir = e.key === ahead ? 1 : e.key === back ? -1 : 0;
    if (!dir) return;
    e.preventDefault();
    onResize(fitCode(m.total, m.size + signOf(e.currentTarget, m.axis) * dir * STEP), true);
  };

  return (
    <div
      className={`code-divider${active ? " active" : ""}`}
      data-axis={axis}
      role="separator"
      aria-orientation={axis === "below" ? "horizontal" : "vertical"}
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
