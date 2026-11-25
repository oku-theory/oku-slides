// A page of one data's rows in the data pane (`data-sheet` surface): every column, typed from
// its header menu, edited like a view's data, and paged by 100 rows.

import { ChartDataGrid, type GridEdit } from "./chart/ChartDataGrid";
import type { Cell, DataRef } from "./chart/chartTypes";
import type { DataPanelAction } from "./dataActions";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconButton } from "./primitives";
import type { ColumnType } from "./slideTypes";

/** The rows of a page: the session sends at most this many. */
export const SHEET_ROWS = 100;

/** The page, its paging and its grid; each gesture is sent as a panel action (`onAction`). */
export function DataSheetView({ data, source, rowIds, columnIds, types, offset, total, onAction }: {
  data: DataRef;
  source: Cell[][];
  rowIds: readonly string[];
  columnIds: readonly string[];
  types: readonly ColumnType[];
  /** Index of the page's first row in the data. */
  offset: number;
  /** The data's number of rows. */
  total: number;
  onAction: (a: DataPanelAction) => void;
}) {
  const shown = source.length - 1;
  const onEdit = (edit: GridEdit) => {
    if (edit.edit === "cells") onAction({ action: "cells", data: data.id, change: edit.change });
    else if (edit.edit === "add_row") onAction({ action: "add-row", data: data.id });
    else {
      const { row, column, cells, names } = edit;
      onAction({ action: "paste", data: data.id, row, column, cells, names });
    }
  };
  const page = (to: number) => onAction({ action: "page", data: data.id, offset: to });
  return (
    <div className="data-sheet">
      <ChartDataGrid
        source={source}
        rowIds={rowIds}
        columnIds={columnIds}
        columnTypes={types}
        onType={(column, type) => onAction({ action: "set-type", data: data.id, column, type })}
        onEdit={onEdit}
      />
      <div className="data-sheet-pages">
        <IconButton className="token ghost data-sheet-previous" label={t("ui-data-previous")} icon={Icon.previous}
          disabled={offset === 0} onClick={() => page(Math.max(0, offset - SHEET_ROWS))} />
        <span className="data-sheet-page a2ui-hint">
          {t("ui-data-page", { first: total === 0 ? 0 : offset + 1, last: offset + shown, total })}
        </span>
        <IconButton className="token ghost data-sheet-next" label={t("ui-data-next")} icon={Icon.next}
          disabled={offset + shown >= total} onClick={() => page(offset + SHEET_ROWS)} />
      </div>
    </div>
  );
}
