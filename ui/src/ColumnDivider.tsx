// Séparateurs entre les colonnes de la table : on les glisse pour régler la
// largeur de la pile ou du fil, on les double-clique pour la rendre par défaut.
// Les largeurs sont posées sur la table (--pile-w, --notes-w), que lit sa grille.

import { useLayoutEffect, useRef } from "react";
import { type Columns, columnBounds } from "./columns";
import Divider from "./Divider";
import { t } from "./i18n";
import type { ColumnWidths } from "./useKeptUi";

interface Props {
  /** Colonne réglée : la pile (à gauche du séparateur) ou le fil (à sa droite). */
  side: keyof Columns;
  label: string;
  /** Largeurs actuelles de la table, de la pile et du fil, lues au début d'un geste. */
  measure: () => Columns & { table: number };
  /** Largeur bornée de la colonne : en direct pendant le glisser, puis `done` au relâcher ou au clavier. */
  onResize: (size: number, done: boolean) => void;
  /** Double-clic : largeur par défaut. */
  onReset: () => void;
}

/** Séparateur d'une colonne ; aller vers la droite élargit la pile et rétrécit le fil. */
export default function ColumnDivider({ side, label, measure, onResize, onReset }: Props) {
  return (
    <Divider
      // Le côté en attribut, pas en classe : `.pile` et `.notes` sont déjà les colonnes.
      className="col-divider"
      side={side}
      label={label}
      axis="x"
      grow={side === "pile" ? 1 : -1}
      measure={() => {
        const m = measure();
        return { size: m[side], bounds: columnBounds(side, m.table, m.pile) };
      }}
      onResize={onResize}
      onReset={onReset}
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
          onResize={(size, done) => (done ? onChange({ ...columns, [side]: size }) : show(side, size))}
          onReset={() => onChange({ ...columns, [side]: null })}
        />
      ))}
    </div>
  );
}
