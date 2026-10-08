// Miroir des types Rust de oku-slides-chart. Dans le dépôt, ces types viennent de
// bindings.ts (tauri-specta) ; ce fichier sert tant qu'ils n'y sont pas.

/** Cellule de dataset.source ; un booléen est une case à cocher. */
export type Cell = number | string | boolean | null;

/** Opération `set_data` (enum Rust DataOp) : données d'un graphique ou d'un tableau. */
export type DataOp =
  | { op: "set_cell"; row: number; col: number; value: Cell }
  | { op: "insert_row"; at: number; cells: Cell[] }
  | { op: "remove_row"; at: number }
  | { op: "insert_column"; at: number; cells: Cell[] }
  | { op: "remove_column"; at: number }
  | { op: "replace"; source: Cell[][] };
