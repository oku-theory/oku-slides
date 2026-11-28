// A page of one data's rows in the data pane (`data-sheet` surface): every column, typed from
// its header menu, edited like a view's data, and paged by 100 rows. A query data shows its SQL,
// applied on submit, the data it reads, its failure, and its result read-only. A view over a
// query shows the same, its SQL read-only (`QueryInView`).

import { useState } from "react";
import { useNumberLang } from "./a2ui/numberLang";
import { ChartDataGrid, displayCell, type GridEdit } from "./chart/ChartDataGrid";
import type { Cell, DataChoice, DataRef } from "./chart/chartTypes";
import type { DataPanelAction } from "./dataActions";
import { type DataState, STATE_LABEL } from "./DataList";
import { t } from "./i18n";
import { Icon } from "./icons";
import { Button, IconButton } from "./primitives";
import type { ColumnType } from "./slideTypes";

/** The rows of a page: the session sends at most this many. */
export const SHEET_ROWS = 100;

/** A query data's SQL, state, failure and inputs (`QueryView`, oku-slides-a2ui). */
export interface QueryView {
  sql: string;
  state: DataState;
  /** Why it failed, localized. */
  error?: string;
  inputs: readonly DataChoice[];
}

/** The page, its paging and its grid; each gesture is sent as a panel action (`onAction`). */
export function DataSheetView({ data, source, rowIds, columnIds, types, offset, total, query, error, onAction }: {
  data: DataRef;
  source: Cell[][];
  rowIds: readonly string[];
  columnIds: readonly string[];
  types: readonly ColumnType[];
  /** Index of the page's first row in the data. */
  offset: number;
  /** The data's number of rows. */
  total: number;
  /** A query data's view; absent for a data entered by hand. */
  query?: QueryView;
  /** The refusal of the panel's last action on this data. */
  error?: string;
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
      {error && <p className="data-sheet-refusal" role="alert">{error}</p>}
      {query ? (
        <>
          {/* A new SQL from the session resets the draft. */}
          <QueryEditor key={query.sql} data={data.id} query={query}
            onOpen={(input) => onAction({ action: "open-data", data: input })}
            onQuery={(sql) => onAction({ action: "set-query", data: data.id, sql })} />
          <ResultGrid source={source} />
        </>
      ) : (
        <ChartDataGrid
          source={source}
          rowIds={rowIds}
          columnIds={columnIds}
          columnTypes={types}
          onType={(column, type) => onAction({ action: "set-type", data: data.id, column, type })}
          onEdit={onEdit}
        />
      )}
      <div className="data-sheet-pages">
        <IconButton className="token ghost data-sheet-previous" label={t("ui-data-previous")} icon={Icon.previous}
          disabled={offset === 0} onClick={() => page(Math.max(0, offset - SHEET_ROWS))} />
        <span className="data-sheet-page a2ui-hint">
          {t("ui-data-page", { first: total === 0 ? 0 : offset + 1, last: offset + Math.max(0, shown), total })}
        </span>
        <IconButton className="token ghost data-sheet-next" label={t("ui-data-next")} icon={Icon.next}
          disabled={offset + shown >= total} onClick={() => page(offset + SHEET_ROWS)} />
      </div>
    </div>
  );
}

/**
 * A query's result under its SQL, all read-only, in the data pane of a view over query data
 * `data`: its SQL opens the data's sheet (`onOpen`), which edits it.
 */
export function QueryInView({ data, query, source, onOpen }: {
  data: string;
  query: QueryView;
  /** The ready result: row 0 names the columns; empty while pending or failed. */
  source: Cell[][];
  onOpen: (data: string) => void;
}) {
  return (
    <>
      <QueryEditor key={query.sql} data={data} query={query} onOpen={onOpen} />
      {source.length > 0 && <ResultGrid source={source} />}
    </>
  );
}

/**
 * A query's SQL, sent as one change (`onQuery`) on submit, its state, failure and inputs, which
 * open their data (`onOpen`); without `onQuery`, the SQL is read-only and opens `data`'s sheet.
 */
function QueryEditor({ data, query, onOpen, onQuery }: {
  data: string;
  query: QueryView;
  onOpen: (data: string) => void;
  onQuery?: (sql: string) => void;
}) {
  const [sql, setSql] = useState(query.sql);
  const submit = () => {
    if (sql.trim() && sql !== query.sql) onQuery?.(sql);
  };
  return (
    <form className="data-query" onSubmit={(e) => (e.preventDefault(), submit())}>
      <textarea
        className="data-query-sql"
        aria-label={t("ui-data-query-sql")}
        spellCheck={false}
        readOnly={!onQuery}
        value={sql}
        onChange={(e) => setSql(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) (e.preventDefault(), submit());
        }}
      />
      <div className="data-query-bar">
        {onQuery ? (
          <Button primary type="submit" className="data-query-apply" disabled={!sql.trim() || sql === query.sql}>
            {t("ui-data-query-apply")}
          </Button>
        ) : (
          <Button className="data-query-edit" onClick={() => onOpen(data)}>{t("ui-data-query-edit")}</Button>
        )}
        <span className={`data-row-state is-${query.state}`}>{t(STATE_LABEL[query.state])}</span>
      </div>
      {query.error && <p className="data-query-error" role="alert">{query.error}</p>}
      {query.inputs.length > 0 && (
        <div className="data-query-inputs">
          <span className="a2ui-hint">{t("ui-data-query-inputs")}</span>
          {query.inputs.map((input) => (
            <button key={input.id} type="button" className="data-row-user data-query-input"
              onClick={() => onOpen(input.id)}>
              {input.label ?? input.name}
            </button>
          ))}
        </div>
      )}
    </form>
  );
}

/** A query's result page, read-only: row 0 names the columns. */
function ResultGrid({ source }: { source: Cell[][] }) {
  const lang = useNumberLang();
  const [header = [], ...rows] = source;
  return (
    <div className="cdg-scroll data-query-result">
      <table className="cdg">
        <thead>
          <tr>{header.map((c, i) => <th key={i}>{displayCell(c, lang)}</th>)}</tr>
        </thead>
        <tbody>
          {rows.map((row, r) => (
            <tr key={r}>{row.map((c, i) => <td key={i} className={typeof c === "number" ? "is-number" : undefined}>{displayCell(c, lang)}</td>)}</tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
