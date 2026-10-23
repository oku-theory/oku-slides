// Modification des données d'un graphique ou d'un tableau, envoyée par la surface `host-data`
// (miroir de `DataEdit`, oku-slides-session) : la session en fait une op annulable.

import type { ColumnAlign, TableFlag } from "../table/tableDeck";
import type { View } from "../table/ShowAsButton";
import type { ChartKind } from "./chartDeck";
import type { DataOp } from "./chartTypes";

/** Une modification, nommée par `edit`. */
export type DataEdit =
  | { edit: "cells"; change: DataOp }
  | { edit: "kind"; kind: ChartKind }
  | { edit: "stack"; stacked: boolean }
  | { edit: "color"; index: number; color: string | null }
  | { edit: "flag"; flag: TableFlag; on: boolean }
  | { edit: "align"; column: number; align: ColumnAlign }
  | { edit: "show_as"; view: View };
