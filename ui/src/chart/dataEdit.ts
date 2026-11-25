// Edits of a chart's or a table's data, sent by the `host-data` surface (mirror of `DataEdit`,
// oku-slides-session): the session turns each into an undoable op.

import type { ColumnAlign, TableFlag } from "../table/tableDeck";
import type { View } from "../table/ShowAsButton";
import type { ChartKind } from "./chartDeck";
import type { Cell } from "./chartTypes";
import type { TableOp } from "../slideTypes";

/** One edit, named by `edit`. */
export type DataEdit =
  | { edit: "cells"; change: TableOp }
  /** A block pasted at a cell, by row and column id; `names`: the columns it adds. */
  | { edit: "paste"; row: string; column: string; cells: Cell[][]; names: string[] }
  /** An empty row added after the data's last one, past the rows the pane shows. */
  | { edit: "add_row" }
  | { edit: "kind"; kind: ChartKind }
  | { edit: "stack"; stacked: boolean }
  | { edit: "color"; index: number; color: string | null }
  | { edit: "flag"; flag: TableFlag; on: boolean }
  | { edit: "align"; column: number; align: ColumnAlign }
  | { edit: "show_as"; view: View }
  /** The view switched to another data of the deck, by id. */
  | { edit: "bind"; data: string };
