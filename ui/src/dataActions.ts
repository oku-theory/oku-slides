// What the data panel asks: the `deck-data` list, the `data-sheet` page in the data pane and a
// view's data name. The UI opens the pane or selects an element itself; the rest goes to the
// session (mirror of `DataAction`, oku-slides-session).

import type { Cell } from "./chart/chartTypes";
import type { ColumnType, TableOp } from "./slideTypes";

/** An action of the session on the deck's data: one undo step each. */
export type DataAction =
  | { action: "add" }
  /** A query data reading no data yet; `name`: its name, else a free default one. */
  | { action: "add-query"; name?: string }
  /** A query data's new SQL. */
  | { action: "set-query"; data: string; sql: string }
  | { action: "rename"; data: string; name: string }
  | { action: "remove"; data: string }
  | { action: "set-type"; data: string; column: string; type: ColumnType }
  | { action: "cells"; data: string; change: TableOp }
  | { action: "add-row"; data: string }
  /** A block pasted at a cell, by row and column id; `names`: the columns it adds. */
  | { action: "paste"; data: string; row: string; column: string; cells: Cell[][]; names: string[] };

/** An action of the data panel, named by `action`. */
export type DataPanelAction =
  | DataAction
  /** Opens the data pane on a data (a row of the list). */
  | { action: "select"; data: string }
  /** Opens the data pane on a data (a view's data name). */
  | { action: "open-data"; data: string }
  /** Shows another page of the data's rows. */
  | { action: "page"; data: string; offset: number }
  /** Shows the slide and selects the element of a view. */
  | { action: "open-element"; slide: string; element: string };

/** Where an action goes: the data pane's sheet, the stage's selection, or the session. */
export type DataRoute =
  | { to: "sheet"; data: string; offset: number }
  | { to: "element"; slide: string; element: string }
  | { to: "session"; action: DataAction };

/** A panel action written in a surface's `edit`; the session validates the rest. */
export const isPanelAction = (v: unknown): v is DataPanelAction =>
  typeof v === "object" && v !== null && typeof (v as { action?: unknown }).action === "string";

/** Where `a` goes. */
export function routeDataAction(a: DataPanelAction): DataRoute {
  switch (a.action) {
    case "select":
    case "open-data":
      return { to: "sheet", data: a.data, offset: 0 };
    case "page":
      return { to: "sheet", data: a.data, offset: a.offset };
    case "open-element":
      return { to: "element", slide: a.slide, element: a.element };
    default:
      return { to: "session", action: a };
  }
}
