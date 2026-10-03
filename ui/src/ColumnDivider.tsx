// Séparateurs entre les colonnes de la table : on les glisse pour régler la
// largeur de la pile ou du fil, on les double-clique pour la rendre par défaut.
// Les largeurs sont posées sur la table (--pile-w, --notes-w), que lit sa grille.
//
// Glisser fait à la main (événements de pointeur), comme la pile : dans l'app
// Tauri, la webview réserve le glisser-déposer HTML5 au dépôt de fichiers.

import { useLayoutEffect, useRef, useState } from "react";
import { type Columns, fitColumns } from "./columns";
import { t } from "./i18n";
import type { ColumnWidths } from "./useViewOptions";

interface Props {
  /** Colonne réglée : la pile (à gauche du séparateur) ou le fil (à sa droite). */
  side: "pile" | "notes";
  label: string;
  /** Largeurs actuelles de la table, de la pile et du fil, lues au début d'un geste. */
  measure: () => Columns & { table: number };
  /** Largeurs bornées : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
  onResize: (cols: Columns, done: boolean) => void;
  /** Double-clic : largeur par défaut. */
  onReset: () => void;
}

/** Pas d'une flèche du clavier, en px. */
const STEP = 16;

interface Drag {
  pointer: number;
  startX: number;
  start: Columns & { table: number };
  /** +1 si aller vers la droite élargit la colonne, -1 sinon. */
  sign: number;
  /** Dernières largeurs montrées. */
  last: Columns;
}

export default function ColumnDivider({ side, label, measure, onResize, onReset }: Props) {
  const drag = useRef<Drag | null>(null);
  const [active, setActive] = useState(false);

  /** +1 si aller vers la droite élargit la colonne ; la table s'inverse de droite à gauche. */
  const signOf = (el: HTMLElement) => (side === "pile" ? 1 : -1) * (getComputedStyle(el).direction === "rtl" ? -1 : 1);

  /** Largeurs bornées après un déplacement de `dx` px (vers la droite) depuis `start`. */
  const resized = (start: Columns & { table: number }, dx: number) =>
    side === "pile"
      ? fitColumns(start.table, start.pile + dx, start.notes)
      : fitColumns(start.table, start.pile, start.notes + dx);

  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const start = measure();
    drag.current = { pointer: e.pointerId, startX: e.clientX, start, sign: signOf(e.currentTarget), last: start };
    setActive(true);
  };

  const move = (e: React.PointerEvent<HTMLDivElement>, done: boolean) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    d.last = resized(d.start, d.sign * (e.clientX - d.startX));
    if (done) end();
    onResize(d.last, done);
  };

  /** Geste interrompu : la largeur reste là où il s'est arrêté (le pointeur n'a plus de position sûre). */
  const cancel = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointer !== e.pointerId) return;
    end();
    onResize({ pile: d.last.pile, notes: d.last.notes }, true);
  };

  const end = () => {
    drag.current = null;
    setActive(false);
  };

  const key = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const dir = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!dir) return;
    e.preventDefault();
    onResize(resized(measure(), signOf(e.currentTarget) * dir * STEP), true);
  };

  return (
    <div
      // Le côté en attribut, pas en classe : `.pile` et `.notes` sont déjà les colonnes.
      className={`col-divider${active ? " active" : ""}`}
      data-side={side}
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

const SIDES = ["pile", "notes"] as const;

/**
 * Les deux séparateurs de la table qui les contient, et les largeurs posées sur elle.
 * Pendant un glisser, la largeur suit le pointeur sur la table seule ; elle n'est
 * retenue (`onChange`) qu'au relâcher.
 */
export function ColumnDividers({ columns, onChange }: { columns: ColumnWidths; onChange: (columns: ColumnWidths) => void }) {
  const box = useRef<HTMLDivElement>(null);
  const tableEl = () => box.current?.parentElement ?? null;

  /** Pose une largeur sur la table ; `null` : celle de la feuille de style. */
  const show = (side: (typeof SIDES)[number], width: number | null) => {
    const el = tableEl();
    if (!el) return;
    if (width === null) el.style.removeProperty(`--${side}-w`);
    else el.style.setProperty(`--${side}-w`, `${width}px`);
  };

  useLayoutEffect(() => {
    for (const side of SIDES) show(side, columns[side]);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `show` ne lit que la table
  }, [columns.pile, columns.notes]);

  /** Largeurs effectives de la grille : colonnes 1 (pile) et 3 (fil). */
  const measure = () => {
    const el = tableEl();
    const cols = el ? getComputedStyle(el).gridTemplateColumns.split(" ").map(parseFloat) : [];
    return { table: el?.clientWidth ?? 0, pile: cols[0] || 0, notes: cols[2] || 0 };
  };

  return (
    // Sans boîte : les séparateurs sont des cases de la grille de la table.
    <div ref={box} style={{ display: "contents" }}>
      {SIDES.map((side) => (
        <ColumnDivider
          key={side}
          side={side}
          label={t(side === "pile" ? "ui-divider-pile" : "ui-divider-notes")}
          measure={measure}
          onResize={(cols, done) => (done ? onChange({ ...columns, [side]: cols[side] }) : show(side, cols[side]))}
          onReset={() => onChange({ ...columns, [side]: null })}
        />
      ))}
    </div>
  );
}
