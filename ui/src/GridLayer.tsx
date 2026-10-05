// La grille de la scène : un point sur chaque nœud, la place visée pendant un glisser.

import type { Cell, GridSpec, ZoneView } from "./grid";
import { cellRect, tracks } from "./grid";

/** La grille : un petit point sur chaque nœud (coins des cases), et la place visée. */
export default function GridLayer({ zone, grid, scale, target }: { zone: ZoneView; grid: GridSpec; scale: number; target: Cell | null }) {
  const t = tracks(zone, grid);
  const px = (v: number) => v * scale;
  const hot = target ? cellRect(zone, grid, target) : null;
  // Nœuds : là où les bords des colonnes croisent ceux des lignes ; c'est là que tout s'accroche.
  const xs = t.cols.flatMap((c) => [c.start, c.end]).map(px);
  const ys = t.rows.flatMap((r) => [r.start, r.end]).map(px);
  return (
    <svg className="grid-layer" aria-hidden width={px(zone.w)} height={px(zone.h)} style={{ left: px(zone.x), top: px(zone.y) }}>
      {hot && <rect className="grid-hot" x={px(hot.x)} y={px(hot.y)} width={px(hot.w)} height={px(hot.h)} rx={4} />}
      {ys.map((y, i) => xs.map((x, j) => <circle key={`${i}-${j}`} className="grid-node" cx={x} cy={y} r={1.6} />))}
    </svg>
  );
}
