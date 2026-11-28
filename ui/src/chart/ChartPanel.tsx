// View of a chart's data (`ChartData` component of the `host-data` surface): its type, stacking,
// series colors and the grid of its data. Each gesture sends an edit (`DataEdit`) the session
// applies, then the surface is read again. Over a query, the grid is its result, read-only.

import type { ReactNode } from "react";

import { type QueryView, QueryInView } from "../DataSheet";
import { t } from "../i18n";
import { Icon, stroke } from "../icons";
import { IconToggle } from "../primitives";
import { ShowAsButton } from "../table/ShowAsButton";
import { ChartDataGrid } from "./ChartDataGrid";
import { CHART_KINDS, type ChartKind, asChartKind } from "./chartDeck";
import type { Cell, DataChoice, DataRef } from "./chartTypes";
import type { DataEdit } from "./dataEdit";
import { DataPick } from "./DataPick";
import { SeriesColors } from "./SeriesColors";

const kindLabel = (kind: ChartKind) => t("ui-chart-kind", { kind });

/** Each kind's icon; the name stays as `aria-label` and tooltip. */
const KIND_ICON: Record<ChartKind, ReactNode> = {
  bar: stroke(<><path d="M4 20h16" /><path d="M7 20v-6M12 20V6M17 20v-9" /></>),
  line: stroke(<><path d="M4 20h16" /><path d="M4 15l5-5 4 3 7-7" /></>),
  pie: stroke(<><circle cx="12" cy="12" r="8" /><path d="M12 4v8l6 5" /></>),
  scatter: stroke(<><path d="M4 20h16M4 20V4" /><circle cx="9" cy="14" r="1.2" /><circle cx="13" cy="9" r="1.2" /><circle cx="17" cy="12" r="1.2" /><circle cx="16" cy="6" r="1.2" /></>),
  radar: stroke(<><path d="M12 3l8.5 6.2-3.3 10H6.8L3.5 9.2z" /><path d="M12 7.5l4.3 3.2-2 5H9.6l-2-5z" /></>),
  waterfall: stroke(<><path d="M4 20h16" /><path d="M6 20v-8M10 12V7M14 7v5M18 12v8" /><path d="M6 12h4M10 7h4M14 12h4" /></>),
};

/** A series one can color: its name and its own color (absent: the palette). */
export type SeriesSwatch = { name: string; color?: string };

/** A chart's data and settings; each gesture is sent as an edit (`onEdit`). */
export function ChartDataView({ kind, source, rowIds, columnIds, checkboxColumns, stacked, series, palette, data, choices, query, onEdit, onOpen }: {
  /** Type of the first series; absent without data. */
  kind?: string;
  /** dataset.source: row 0 = headers; empty without data. */
  source: Cell[][];
  /** Id of each data row, after the header. */
  rowIds: readonly string[];
  /** Id of each column. */
  columnIds: readonly string[];
  /** Checkbox columns (the Total of a waterfall). */
  checkboxColumns: readonly number[];
  /** Stacked series; absent when stacking makes no sense. */
  stacked?: boolean;
  series: readonly SeriesSwatch[];
  /** The theme's chart palette, as `#rrggbb`. */
  palette: readonly string[];
  /** The data the chart shows. */
  data: DataRef;
  /** The data it can switch to, `data` included. */
  choices: readonly DataChoice[];
  /** Over a query data: its SQL, state, failure and inputs; `source` is its result. */
  query?: QueryView;
  onEdit: (edit: DataEdit) => void;
  /** Opens the view's data in the data pane. */
  onOpen: (data: string) => void;
}) {
  const current = asChartKind(kind);
  const hasData = source.length > 0;
  // Converting the chart reads its rows, which the session refuses for a query's (`op-data-derived`).
  const convertible = hasData && !query;
  return (
    <section className="chart-panel" aria-label={t("ui-chart-panel")}>
      <DataPick data={data} choices={choices} onEdit={onEdit} onOpen={onOpen} />
      <div className="panel-toolbar">
        <div className="chart-kinds" role="group" aria-label={t("ui-chart-kinds")}>
          {CHART_KINDS.map((k) => (
            <IconToggle key={k} label={kindLabel(k)} pressed={current === k} disabled={!convertible}
              onToggle={() => k !== current && onEdit({ edit: "kind", kind: k })} icon={KIND_ICON[k]} />
          ))}
        </div>
        {stacked !== undefined && (
          <div className="chart-stack chart-options" role="group" aria-label={t("ui-chart-options")}>
            <IconToggle label={t("ui-chart-stack")} pressed={stacked} onToggle={() => onEdit({ edit: "stack", stacked: !stacked })} icon={Icon.stack} />
          </div>
        )}
        <ShowAsButton view="table" onShowAs={(view) => onEdit({ edit: "show_as", view })} disabled={!convertible} />
      </div>
      <SeriesColors series={series} colors={palette} onColor={(index, color) => onEdit({ edit: "color", index, color })} />
      {current === "waterfall" && <p className="a2ui-hint">{t("ui-chart-waterfall-hint")}</p>}
      {query ? (
        <QueryInView data={data.id} query={query} source={source} onOpen={onOpen} />
      ) : hasData ? (
        <ChartDataGrid source={source} rowIds={rowIds} columnIds={columnIds} checkboxColumns={checkboxColumns} onEdit={onEdit} />
      ) : (
        <p className="a2ui-hint">{t("ui-chart-no-data")}</p>
      )}
    </section>
  );
}
