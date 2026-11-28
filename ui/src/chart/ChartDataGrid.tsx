// Grid editing the data a chart or a table shows (its dataset.source).
//
// Controlled component: it shows `source` and emits edits naming rows and columns by id (a
// `TableOp`, a pasted block or an added row); the Rust core applies them, renders again and
// sends the new source back. No data changes
// locally, so undo and dictation stay consistent with the grid.
//
// TanStack Table v9 (`useTable` + `tableFeatures`), without optional features.

import { useCallback, useMemo, type ClipboardEvent, type KeyboardEvent } from "react";
import { createColumnHelper, tableFeatures, useTable } from "@tanstack/react-table";
import { useNumberLang } from "../a2ui/numberLang";
import { t, tPlain } from "../i18n";
import { Icon } from "../icons";
import { parseCheck, parseTsv } from "../table/cells";
import { formatNumber, parseNumber } from "../table/numbers";
import type { ColumnType, TableOp } from "../slideTypes";
import type { Cell } from "./chartTypes";
import type { DataEdit } from "./dataEdit";
import "./ChartDataGrid.css";

type Row = { index: number; id: string; cells: Cell[] };

/** The edits the grid sends: a cell, row or column change, a paste, or a row added at the end. */
export type GridEdit = Extract<DataEdit, { edit: "cells" | "paste" | "add_row" }>;

const features = tableFeatures({});
const helper = createColumnHelper<typeof features, Row>();

export type ChartDataGridProps = {
  /** dataset.source: row 0 holds the column names. */
  source: Cell[][];
  /** Id of each data row (`source[i + 1]`). */
  rowIds: readonly string[];
  /** Id of each column. */
  columnIds: readonly string[];
  onEdit: (edit: GridEdit) => void;
  /** Column 0 as text (categories); the others expect numbers. */
  firstColumnIsLabel?: boolean;
  /** Checkbox (boolean) columns, like a waterfall's Total. */
  checkboxColumns?: readonly number[];
  /** Name of a column added or pasted without a header (`n`: its index); "Series n" by default. */
  newColumnName?: (n: number) => string;
  /** Each column's type, which sets how its cells are typed in place of the two props above. */
  columnTypes?: readonly ColumnType[];
  /** Changes a column's type, from a menu in its header; no menu without it. */
  onType?: (column: string, type: ColumnType) => void;
};

/** Each column type, with its label's key. */
const TYPES: Readonly<Record<ColumnType, string>> = {
  text: "ui-data-type-text",
  number: "ui-data-type-number",
  integer: "ui-data-type-integer",
  bool: "ui-data-type-bool",
  date: "ui-data-type-date",
};

/** How a column's cells are typed in. */
export type CellKind = "label" | "number" | "check";

/** The cell kind of a column of type `ty`. */
const kindOfType = (ty: ColumnType | undefined): CellKind =>
  ty === "bool" ? "check" : ty === "number" || ty === "integer" ? "number" : "label";

/** Default series name written into the deck, so without isolation marks (t-1380). */
const seriesName = (n: number) => tPlain("ui-chart-series", { n });

const NO_CHECKBOXES: readonly number[] = [];

export function ChartDataGrid({
  source,
  rowIds,
  columnIds,
  onEdit,
  firstColumnIsLabel = true,
  checkboxColumns = NO_CHECKBOXES,
  newColumnName = seriesName,
  columnTypes,
  onType,
}: ChartDataGridProps) {
  const lang = useNumberLang();
  const onOp = useCallback((change: TableOp) => onEdit({ edit: "cells", change }), [onEdit]);
  const header = source[0] ?? [];
  const width = header.length;
  const data = useMemo<Row[]>(
    () => source.slice(1).map((cells, i) => ({ index: i + 1, id: rowIds[i] ?? "", cells })),
    [source, rowIds],
  );

  const kindOf = useCallback(
    (col: number): CellKind =>
      columnTypes
        // A column a paste adds has no type yet: its cells read as numbers when they are.
        ? col < columnTypes.length ? kindOfType(columnTypes[col]) : "number"
        : checkboxColumns.includes(col) ? "check" : firstColumnIsLabel && col === 0 ? "label" : "number",
    [columnTypes, checkboxColumns, firstColumnIsLabel],
  );

  const columns = useMemo(
    () =>
      helper.columns(
        header.map((h, col) =>
          helper.accessor((row) => row.cells[col] ?? null, {
            id: `c${col}`,
            header: () => (
              <div className="cdg-head">
                <CellInput
                  value={h}
                  label
                  lang={lang}
                  ariaLabel={t("ui-chart-column-name", { col: col + 1 })}
                  onCommit={(value) => onOp({ op: "rename_column", column: columnIds[col], name: displayCell(value, lang) })}
                />
                {onType && columnTypes?.[col] && (
                  <select
                    className="cdg-type"
                    aria-label={t("ui-data-column-type")}
                    value={columnTypes[col]}
                    onChange={(e) => onType(columnIds[col], e.currentTarget.value as ColumnType)}
                  >
                    {Object.entries(TYPES).map(([ty, key]) => <option key={ty} value={ty}>{t(key)}</option>)}
                  </select>
                )}
                {width > 1 && (
                  <button
                    className="cdg-remove"
                    title={t("ui-chart-remove-column")}
                    onClick={() => onOp({ op: "remove_column", column: columnIds[col] })}
                  >
                    ×
                  </button>
                )}
              </div>
            ),
            cell: (ctx) =>
              kindOf(col) === "check" ? (
                <CellCheck
                  value={ctx.getValue()}
                  ariaLabel={t("ui-chart-cell", { row: ctx.row.original.index, col: col + 1 })}
                  onCommit={(value) =>
                    onOp({ op: "set_cell", row: ctx.row.original.id, column: columnIds[col], value })
                  }
                />
              ) : (
                <CellInput
                  value={ctx.getValue()}
                  label={kindOf(col) === "label"}
                  lang={lang}
                  ariaLabel={t("ui-chart-cell", { row: ctx.row.original.index, col: col + 1 })}
                  testId={`cdg-cell-${ctx.row.original.index}-${col}`}
                  onCommit={(value) =>
                    onOp({ op: "set_cell", row: ctx.row.original.id, column: columnIds[col], value })
                  }
                  onPasteGrid={(grid) =>
                    onEdit({
                      edit: "paste",
                      row: ctx.row.original.id,
                      column: columnIds[col],
                      ...pasteCells(grid, col, width, kindOf, newColumnName, lang),
                    })
                  }
                />
              ),
          }),
        ),
      ),
    [header, width, columnIds, onOp, onEdit, newColumnName, lang, kindOf, columnTypes, onType],
  );

  const table = useTable({ features, columns, data });

  return (
    <div className="cdg">
      <div className="cdg-scroll">
        <table>
          <thead>
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                {group.headers.map((h) => (
                  <th key={h.id}>
                    <table.FlexRender header={h} />
                  </th>
                ))}
                <th className="cdg-gutter" />
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.id}>
                {row.getAllCells().map((cell) => (
                  <td key={cell.id}>
                    <table.FlexRender cell={cell} />
                  </td>
                ))}
                <td className="cdg-gutter">
                  <button
                    className="cdg-remove"
                    title={t("ui-chart-remove-row")}
                    onClick={() => onOp({ op: "remove_row", row: row.original.id })}
                  >
                    ×
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="cdg-actions">
        <button aria-label={t("ui-chart-add-row")} title={t("ui-chart-add-row")} onClick={() => onEdit({ edit: "add_row" })}>
          {Icon.addRow}
        </button>
        <button aria-label={t("ui-chart-add-column")} title={t("ui-chart-add-column")} onClick={() => onOp({ op: "insert_column", after: columnIds.at(-1), name: newColumnName(width) })}>
          {Icon.addColumn}
        </button>
      </div>
    </div>
  );
}

type CellInputProps = {
  value: Cell;
  label?: boolean;
  /** Language numbers are written in. */
  lang: string;
  ariaLabel: string;
  onCommit: (value: Cell) => void;
  onPasteGrid?: (grid: string[][]) => void;
  /** A data cell's `cdg-cell-<row>-<column>` (row 1 the first under the header, column from 0). */
  testId?: string;
};

function CellInput({ value, label, lang, ariaLabel, onCommit, onPasteGrid, testId }: CellInputProps) {
  const shown = displayCell(value, lang);
  const commit = (raw: string) => {
    const next = label ? raw : parseNumber(raw, lang);
    if (displayCell(next, lang) !== shown) onCommit(next);
  };
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") e.currentTarget.blur();
    if (e.key === "Escape") {
      e.currentTarget.value = shown;
      e.currentTarget.blur();
    }
  };
  const onPaste = (e: ClipboardEvent<HTMLInputElement>) => {
    const text = e.clipboardData.getData("text/plain");
    if (!onPasteGrid || !/[\t\n]/.test(text.trim())) return;
    e.preventDefault();
    onPasteGrid(parseTsv(text));
  };
  return (
    <input
      // The key resets the draft when the value changes on the Rust side.
      key={shown}
      defaultValue={shown}
      aria-label={ariaLabel}
      data-testid={testId}
      className={label ? "cdg-input" : "cdg-input cdg-number"}
      inputMode={label ? "text" : "decimal"}
      onBlur={(e) => commit(e.currentTarget.value)}
      onKeyDown={onKeyDown}
      onPaste={onPaste}
    />
  );
}

function CellCheck({ value, ariaLabel, onCommit }: {
  value: Cell;
  ariaLabel: string;
  onCommit: (value: boolean) => void;
}) {
  return (
    <input
      type="checkbox"
      className="cdg-check"
      checked={value === true}
      aria-label={ariaLabel}
      onChange={(e) => onCommit(e.currentTarget.checked)}
    />
  );
}

/** A cell as a grid shows it: numbers written in `lang`, empty for null. */
export function displayCell(c: Cell, lang: string): string {
  if (c === null) return "";
  if (typeof c === "boolean") return String(c);
  if (typeof c === "number") return formatNumber(c, lang);
  return c;
}

/**
 * A block pasted at view column `col` of a grid `width` columns wide, parsed as typed input
 * into each column's kind (`kindOf`), with the names of the columns it adds past the last one.
 */
export function pasteCells(
  grid: string[][],
  col: number,
  width: number,
  kindOf: (col: number) => CellKind,
  newColumnName: (n: number) => string = seriesName,
  /** Language numbers are written in (thousands "," or "."); empty: `parseCell` alone. */
  lang = "",
): { cells: Cell[][]; names: string[] } {
  const cells = grid.map((line) =>
    line.map((raw, dc) => {
      const kind = kindOf(col + dc);
      return kind === "label" ? raw : kind === "check" ? parseCheck(raw) : parseNumber(raw, lang);
    }),
  );
  const end = col + Math.max(0, ...grid.map((line) => line.length));
  const names = Array.from({ length: Math.max(0, end - width) }, (_, k) => newColumnName(width + k));
  return { cells, names };
}
