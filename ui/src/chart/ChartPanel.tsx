// Vue des données d'un graphique (composant `ChartData` de la surface `host-data`) : son type,
// l'empilement, la couleur des séries et la grille de ses données. Chaque geste part en une
// modification (`DataEdit`) que la session applique, puis la surface est relue.

import type { ReactNode } from "react";

import { t } from "../i18n";
import { Icon, stroke } from "../icons";
import { IconToggle } from "../primitives";
import { ShowAsButton } from "../table/ShowAsButton";
import { ChartDataGrid } from "./ChartDataGrid";
import { CHART_KINDS, type ChartKind, asChartKind } from "./chartDeck";
import type { Cell } from "./chartTypes";
import type { DataEdit } from "./dataEdit";
import { SeriesColors } from "./SeriesColors";

const kindLabel = (kind: ChartKind) => t("ui-chart-kind", { kind });

/** Pictogramme de chaque type ; le nom reste en `aria-label` et en infobulle. */
const KIND_ICON: Record<ChartKind, ReactNode> = {
  bar: stroke(<><path d="M4 20h16" /><path d="M7 20v-6M12 20V6M17 20v-9" /></>),
  line: stroke(<><path d="M4 20h16" /><path d="M4 15l5-5 4 3 7-7" /></>),
  pie: stroke(<><circle cx="12" cy="12" r="8" /><path d="M12 4v8l6 5" /></>),
  scatter: stroke(<><path d="M4 20h16M4 20V4" /><circle cx="9" cy="14" r="1.2" /><circle cx="13" cy="9" r="1.2" /><circle cx="17" cy="12" r="1.2" /><circle cx="16" cy="6" r="1.2" /></>),
  radar: stroke(<><path d="M12 3l8.5 6.2-3.3 10H6.8L3.5 9.2z" /><path d="M12 7.5l4.3 3.2-2 5H9.6l-2-5z" /></>),
  waterfall: stroke(<><path d="M4 20h16" /><path d="M6 20v-8M10 12V7M14 7v5M18 12v8" /><path d="M6 12h4M10 7h4M14 12h4" /></>),
};

/** Une série qu'on peut colorer : son nom et sa couleur propre (absente : la palette). */
export type SeriesSwatch = { name: string; color?: string };

/** Les données d'un graphique et ses réglages ; chaque geste part en une modification (`onEdit`). */
export function ChartDataView({ kind, source, checkboxColumns, stacked, series, palette, onEdit }: {
  /** Type de la première série ; absent sans données. */
  kind?: string;
  /** dataset.source : ligne 0 = en-têtes ; vide sans données. */
  source: Cell[][];
  /** Colonnes de cases à cocher (le Total d'une cascade). */
  checkboxColumns: readonly number[];
  /** Séries empilées ; absent quand l'empilement n'a pas de sens. */
  stacked?: boolean;
  series: readonly SeriesSwatch[];
  /** Palette des graphiques du thème, en `#rrggbb`. */
  palette: readonly string[];
  onEdit: (edit: DataEdit) => void;
}) {
  const current = asChartKind(kind);
  const hasData = source.length > 0;
  return (
    <section className="chart-panel" aria-label={t("ui-chart-panel")}>
      <div className="panel-toolbar">
        <div className="chart-kinds" role="group" aria-label={t("ui-chart-kinds")}>
          {CHART_KINDS.map((k) => (
            <IconToggle key={k} label={kindLabel(k)} pressed={current === k} disabled={!hasData}
              onToggle={() => k !== current && onEdit({ edit: "kind", kind: k })} icon={KIND_ICON[k]} />
          ))}
        </div>
        {stacked !== undefined && (
          <div className="chart-stack chart-options" role="group" aria-label={t("ui-chart-options")}>
            <IconToggle label={t("ui-chart-stack")} pressed={stacked} onToggle={() => onEdit({ edit: "stack", stacked: !stacked })} icon={Icon.stack} />
          </div>
        )}
        <ShowAsButton view="table" onShowAs={(view) => onEdit({ edit: "show_as", view })} disabled={!hasData} />
      </div>
      <SeriesColors series={series} colors={palette} onColor={(index, color) => onEdit({ edit: "color", index, color })} />
      {current === "waterfall" && <p className="a2ui-hint">{t("ui-chart-waterfall-hint")}</p>}
      {hasData ? (
        <ChartDataGrid source={source} checkboxColumns={checkboxColumns} onOp={(change) => onEdit({ edit: "cells", change })} />
      ) : (
        <p className="a2ui-hint">{t("ui-chart-no-data")}</p>
      )}
    </section>
  );
}
