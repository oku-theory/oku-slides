// Grille d'édition des données d'un graphique (dataset.source).
//
// Composant contrôlé : il affiche `source` et émet des DataOp ; c'est le
// cœur Rust qui applique l'opération, régénère le SVG et renvoie la nouvelle
// source. Aucune donnée n'est modifiée localement, donc l'annulation et la
// dictée restent cohérentes avec la grille.
//
// TanStack Table v9 (`useTable` + `tableFeatures`), sans feature optionnelle.

import { useMemo, type ClipboardEvent, type KeyboardEvent } from "react";
import { createColumnHelper, tableFeatures, useTable } from "@tanstack/react-table";
import { locale, t } from "../i18n";
import { Icon } from "../icons";
import { parseCheck, parseTsv } from "../table/cells";
import { formatNumber, parseNumber } from "../table/numbers";
import type { Cell, DataOp } from "./chartTypes";
import "./ChartDataGrid.css";

type Row = { index: number; cells: Cell[] };

const features = tableFeatures({});
const helper = createColumnHelper<typeof features, Row>();

export type ChartDataGridProps = {
  /** dataset.source : ligne 0 = en-têtes. */
  source: Cell[][];
  onOp: (op: DataOp) => void;
  /** Colonne 0 en texte (catégories) ; les autres attendent des nombres. */
  firstColumnIsLabel?: boolean;
  /** Colonnes de cases à cocher (booléens), comme le Total d'une cascade. */
  checkboxColumns?: readonly number[];
  /** Nom d'une colonne ajoutée ou collée sans en-tête (`n` : son index) ; par défaut « Série n ». */
  newColumnName?: (n: number) => string;
  /** Langue du deck (`params.lang`) : les nombres s'écrivent comme sur la slide ; absente, celle de l'interface. */
  numberLang?: string | null;
};

const seriesName = (n: number) => t("ui-chart-series", { n });

const NO_CHECKBOXES: readonly number[] = [];

export function ChartDataGrid({
  source,
  onOp,
  firstColumnIsLabel = true,
  checkboxColumns = NO_CHECKBOXES,
  newColumnName = seriesName,
  numberLang,
}: ChartDataGridProps) {
  const lang = numberLang || locale();
  const header = source[0] ?? [];
  const width = header.length;
  const data = useMemo<Row[]>(
    () => source.slice(1).map((cells, i) => ({ index: i + 1, cells })),
    [source],
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
                  onCommit={(value) => onOp({ op: "set_cell", row: 0, col, value })}
                />
                {width > 1 && (
                  <button
                    className="cdg-remove"
                    title={t("ui-chart-remove-column")}
                    onClick={() => onOp({ op: "remove_column", at: col })}
                  >
                    ×
                  </button>
                )}
              </div>
            ),
            cell: (ctx) =>
              checkboxColumns.includes(col) ? (
                <CellCheck
                  value={ctx.getValue()}
                  ariaLabel={t("ui-chart-cell", { row: ctx.row.original.index, col: col + 1 })}
                  onCommit={(value) =>
                    onOp({ op: "set_cell", row: ctx.row.original.index, col, value })
                  }
                />
              ) : (
                <CellInput
                  value={ctx.getValue()}
                  label={firstColumnIsLabel && col === 0}
                  lang={lang}
                  ariaLabel={t("ui-chart-cell", { row: ctx.row.original.index, col: col + 1 })}
                  onCommit={(value) =>
                    onOp({ op: "set_cell", row: ctx.row.original.index, col, value })
                  }
                  onPasteGrid={(grid) =>
                    onOp({
                      op: "replace",
                      source: pasteInto(
                        source,
                        grid,
                        ctx.row.original.index,
                        col,
                        firstColumnIsLabel,
                        checkboxColumns,
                        newColumnName,
                        lang,
                      ),
                    })
                  }
                />
              ),
          }),
        ),
      ),
    // `source` est nécessaire au collage ; les colonnes changent peu.
    [source, header, width, onOp, firstColumnIsLabel, checkboxColumns, newColumnName, lang],
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
                    onClick={() => onOp({ op: "remove_row", at: row.original.index })}
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
        <button aria-label={t("ui-chart-add-row")} title={t("ui-chart-add-row")} onClick={() => onOp({ op: "insert_row", at: source.length, cells: [] })}>
          {Icon.addRow}
        </button>
        <button aria-label={t("ui-chart-add-column")} title={t("ui-chart-add-column")} onClick={() => onOp({ op: "insert_column", at: width, cells: [newColumnName(width)] })}>
          {Icon.addColumn}
        </button>
      </div>
    </div>
  );
}

type CellInputProps = {
  value: Cell;
  label?: boolean;
  /** Langue d'écriture des nombres. */
  lang: string;
  ariaLabel: string;
  onCommit: (value: Cell) => void;
  onPasteGrid?: (grid: string[][]) => void;
};

function CellInput({ value, label, lang, ariaLabel, onCommit, onPasteGrid }: CellInputProps) {
  const shown = display(value, lang);
  const commit = (raw: string) => {
    const next = label ? raw : parseNumber(raw, lang);
    if (display(next, lang) !== shown) onCommit(next);
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
      // La clé réinitialise le brouillon quand la valeur change côté Rust.
      key={shown}
      defaultValue={shown}
      aria-label={ariaLabel}
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

function display(c: Cell, lang: string): string {
  if (c === null) return "";
  if (typeof c === "boolean") return String(c);
  if (typeof c === "number") return formatNumber(c, lang);
  return c;
}

/** Colle un bloc à partir de (row, col), en agrandissant le tableau si besoin. */
export function pasteInto(
  source: Cell[][],
  grid: string[][],
  row: number,
  col: number,
  firstColumnIsLabel: boolean,
  checkboxColumns: readonly number[] = [],
  newColumnName: (n: number) => string = seriesName,
  /** Langue d'écriture des nombres (milliers « , » / « . ») ; vide : `parseCell` seul. */
  lang = "",
): Cell[][] {
  const out = source.map((r) => [...r]);
  const width = Math.max(out[0]?.length ?? 0, col + Math.max(...grid.map((r) => r.length)));
  grid.forEach((cells, dr) => {
    const r = row + dr;
    while (out.length <= r) out.push([]);
    cells.forEach((raw, dc) => {
      const c = col + dc;
      out[r][c] =
        firstColumnIsLabel && c === 0
          ? raw
          : checkboxColumns.includes(c)
            ? parseCheck(raw)
            : parseNumber(raw, lang);
    });
  });
  for (const r of out) while (r.length < width) r.push(null);
  for (let c = 0; c < width; c++) if (out[0][c] === null) out[0][c] = newColumnName(c);
  return out;
}
