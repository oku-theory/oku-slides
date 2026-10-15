// Séparateur générique entre deux panneaux : on le glisse (ou on presse les
// flèches) pour régler la taille d'un panneau, on le double-clique pour revenir
// à sa taille par défaut. ColumnDivider, CodeDivider et BandDivider le règlent.
//
// Glisser fait à la main (événements de pointeur) : dans l'app Tauri, la webview
// réserve le glisser-déposer HTML5 au dépôt de fichiers.

import { useRef, useState } from "react";
import { type Bounds, fitIn } from "./bounds";

/** Taille actuelle du panneau réglé et ses bornes, lues au début d'un geste. */
export interface Measured {
  size: number;
  bounds: Bounds;
}

/** Réglages d'un séparateur : son axe, son sens, sa mesure et ses retours. */
export interface DividerProps {
  /** Classe CSS du séparateur ; `active` s'y ajoute pendant un glisser. */
  className: string;
  /** Côté réglé, posé en `data-side` quand il y en a un. */
  side?: string;
  label: string;
  /** Axe du geste : `x` pour un séparateur vertical, `y` pour un horizontal. */
  axis: "x" | "y";
  /**
   * +1 si avancer vers la droite (`x`, de gauche à droite) ou vers le bas (`y`)
   * agrandit le panneau, -1 sinon ; de droite à gauche, l'axe `x` s'inverse.
   */
  grow: 1 | -1;
  /** Taille et bornes du panneau ; `null` : rien de mesuré, rien à régler. */
  measure: () => Measured | null;
  /** Taille bornée : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
  onResize: (size: number, done: boolean) => void;
  /** Double-clic : taille par défaut. */
  onReset: () => void;
}

/** Pas d'une flèche du clavier, en px. */
const STEP = 16;

interface Drag {
  pointer: number;
  start: number;
  measured: Measured;
  sign: number;
  last: number;
  /** Le pointeur s'est déplacé : sans cela, un simple clic (ou ceux d'un double-clic) ne règle rien. */
  moved: boolean;
}

/** Sens des flèches sur l'axe, avant `grow` : +1 vers la droite ou le bas. */
const ARROWS = {
  x: { ArrowRight: 1, ArrowLeft: -1 },
  y: { ArrowDown: 1, ArrowUp: -1 },
} as const satisfies Record<DividerProps["axis"], Record<string, number>>;

/** Séparateur réglable à la souris et au clavier, borné par `measure`. */
export default function Divider({ className, side, label, axis, grow, measure, onResize, onReset }: DividerProps) {
  const drag = useRef<Drag | null>(null);
  const [active, setActive] = useState(false);

  /** +1 si avancer sur l'axe agrandit le panneau ; l'axe `x` s'inverse de droite à gauche. */
  const signOf = (el: HTMLElement) => grow * (axis === "x" && getComputedStyle(el).direction === "rtl" ? -1 : 1);
  const at = (e: React.PointerEvent) => (axis === "x" ? e.clientX : e.clientY);

  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const measured = measure();
    if (!measured) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { pointer: e.pointerId, start: at(e), measured, sign: signOf(e.currentTarget), last: measured.size, moved: false };
    setActive(true);
  };

  const move = (e: React.PointerEvent<HTMLDivElement>, done: boolean) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    if (at(e) !== d.start) d.moved = true;
    if (done) end();
    // Sans mouvement, la taille par défaut reste par défaut.
    if (!d.moved) return;
    d.last = fitIn(d.measured.bounds, d.measured.size + d.sign * (at(e) - d.start));
    onResize(d.last, done);
  };

  /** Geste interrompu : la taille reste là où il s'est arrêté (le pointeur n'a plus de position sûre). */
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
    const arrows: Record<string, number> = ARROWS[axis];
    const dir = arrows[e.key] ?? 0;
    if (!dir) return;
    e.preventDefault();
    const m = measure();
    if (!m) return;
    onResize(fitIn(m.bounds, m.size + signOf(e.currentTarget) * dir * STEP), true);
  };

  return (
    <div
      className={`${className}${active ? " active" : ""}`}
      data-side={side}
      role="separator"
      aria-orientation={axis === "x" ? "vertical" : "horizontal"}
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
