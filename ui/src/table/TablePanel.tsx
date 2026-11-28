// View of a table's data (`TableData` component of the `host-data` surface): display options,
// column alignment and the grid of its data (the charts' one). Each gesture sends an edit
// (`DataEdit`) the session applies. Over a query, the grid is its result, read-only.

import type { ReactNode } from "react";

import { ChartDataGrid } from "../chart/ChartDataGrid";
import type { Cell, DataChoice, DataRef } from "../chart/chartTypes";
import type { DataEdit } from "../chart/dataEdit";
import { DataPick } from "../chart/DataPick";
import { type QueryView, QueryInView } from "../DataSheet";
import { t, tPlain } from "../i18n";
import { stroke } from "../icons";
import { IconToggle } from "../primitives";
import { ShowAsButton } from "./ShowAsButton";
import { COLUMN_ALIGNS, type TableFlag, asColumnAlign } from "./tableDeck";

const FLAGS: { key: TableFlag; label: string; icon: ReactNode }[] = [
  { key: "header-row", label: "ui-table-header-row",
    icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 10h16" strokeWidth="3" /></>) },
  { key: "header-column", label: "ui-table-header-column",
    icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M9 5v14" strokeWidth="3" /></>) },
  { key: "striped", label: "ui-table-striped",
    icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 9.5h16M4 14.5h16" /></>) },
];

/** Default column header written into the deck, so without isolation marks (t-1380). */
const tableColumnName = (n: number) => tPlain("ui-table-column", { n: n + 1 });

/** A table's data and options; each gesture is sent as an edit (`onEdit`). */
export function TableDataView({ source, rowIds, columnIds, headerRow, headerColumn, striped, align, data, choices, query, onEdit, onOpen }: {
  /** dataset.source: row 0 = column names; empty without data. */
  source: Cell[][];
  /** Id of each data row, after the header. */
  rowIds: readonly string[];
  /** Id of each column. */
  columnIds: readonly string[];
  headerRow: boolean;
  headerColumn: boolean;
  striped: boolean;
  /** Each column's alignment; no entry: `auto`. */
  align: readonly string[];
  /** The data the table shows. */
  data: DataRef;
  /** The data it can switch to, `data` included. */
  choices: readonly DataChoice[];
  /** Over a query data: its SQL, state, failure and inputs; `source` is its result. */
  query?: QueryView;
  onEdit: (edit: DataEdit) => void;
  /** Opens the view's data in the data pane. */
  onOpen: (data: string) => void;
}) {
  const pick = <DataPick data={data} choices={choices} onEdit={onEdit} onOpen={onOpen} />;
  if (source.length === 0 && !query) {
    return (
      <section className="table-panel chart-panel" aria-label={t("ui-table-panel")}>
        {pick}
        <p className="a2ui-hint">{t("ui-table-no-data")}</p>
      </section>
    );
  }
  const flags: Record<TableFlag, boolean> = { "header-row": headerRow, "header-column": headerColumn, striped };
  const header = source[0] ?? [];
  return (
    <section className="table-panel chart-panel" aria-label={t("ui-table-panel")}>
      {pick}
      <div className="panel-toolbar">
        <div className="table-options chart-options" role="group" aria-label={t("ui-table-options")}>
          {FLAGS.map(({ key, label, icon }) => (
            <IconToggle key={key} label={t(label)} pressed={flags[key]} icon={icon}
              onToggle={() => onEdit({ edit: "flag", flag: key, on: !flags[key] })} />
          ))}
        </div>
        {/* Converting the table reads its rows, which the session refuses for a query's. */}
        <ShowAsButton view="chart" onShowAs={(view) => onEdit({ edit: "show_as", view })} disabled={!!query} />
      </div>
      <div className="table-align" role="group" aria-label={t("ui-table-alignment")}>
        {header.map((name, i) => {
          const shown = String(name ?? "").trim() || String(i + 1);
          return (
            <label key={i}>
              <span>{shown}</span>
              <select aria-label={t("ui-table-align-of", { name: shown })} value={asColumnAlign(align[i])}
                onChange={(e) => onEdit({ edit: "align", column: i, align: asColumnAlign(e.target.value) })}>
                {COLUMN_ALIGNS.map((a) => <option key={a} value={a}>{t("ui-table-align", { align: a })}</option>)}
              </select>
            </label>
          );
        })}
      </div>
      {query ? (
        <QueryInView data={data.id} query={query} source={source} onOpen={onOpen} />
      ) : (
        <ChartDataGrid
          source={source}
          rowIds={rowIds}
          columnIds={columnIds}
          firstColumnIsLabel={headerColumn}
          newColumnName={tableColumnName}
          onEdit={onEdit}
        />
      )}
    </section>
  );
}
