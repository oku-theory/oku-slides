// Panneau d'un graphique sélectionné : son type et la grille de ses données.
// Chaque modification part en op vers le cœur, qui redessine le graphique.

import type React from "react";

import type { Element, Op } from "../api";
import { t } from "../i18n";
import { ChartDataGrid } from "./ChartDataGrid";
import { CHART_KINDS, type ChartKind, chartKind, chartSource, checkboxColumns, colorPatch, isStacked, paletteOf, stackPatch, stackable } from "./chartDeck";
import { SeriesColors } from "./SeriesColors";

const kindLabel = (kind: ChartKind) => t("ui-chart-kind", { kind });

const svg = (children: React.ReactNode) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
    {children}
  </svg>
);

/** Pictogramme de chaque type ; le nom reste en `aria-label` et en infobulle. */
const KIND_ICON: Record<ChartKind, React.ReactNode> = {
  bar: svg(<><path d="M4 20h16" /><path d="M7 20v-6M12 20V6M17 20v-9" /></>),
  line: svg(<><path d="M4 20h16" /><path d="M4 15l5-5 4 3 7-7" /></>),
  pie: svg(<><circle cx="12" cy="12" r="8" /><path d="M12 4v8l6 5" /></>),
  scatter: svg(<><path d="M4 20h16M4 20V4" /><circle cx="9" cy="14" r="1.2" /><circle cx="13" cy="9" r="1.2" /><circle cx="17" cy="12" r="1.2" /><circle cx="16" cy="6" r="1.2" /></>),
  radar: svg(<><path d="M12 3l8.5 6.2-3.3 10H6.8L3.5 9.2z" /><path d="M12 7.5l4.3 3.2-2 5H9.6l-2-5z" /></>),
  waterfall: svg(<><path d="M4 20h16" /><path d="M6 20v-8M10 12V7M14 7v5M18 12v8" /><path d="M6 12h4M10 7h4M14 12h4" /></>),
};

export function ChartPanel({ slide, element, palette, onApply, onKind }: {
  slide: string;
  element: Element;
  /** Palette des graphiques du thème (apparence). */
  palette: readonly string[];
  onApply: (label: string, ops: Op[]) => void;
  /** Change le type : le cœur convertit les données (requête `chart_kind`). */
  onKind: (kind: ChartKind) => void;
}) {
  const source = chartSource(element);
  const current = chartKind(element);
  const setKind = (kind: ChartKind) => {
    if (kind !== current) onKind(kind);
  };
  const stacked = isStacked(element);
  const toggleStack = () =>
    onApply(t("ui-undo-chart-stack", { stacked: stacked ? "no" : "yes" }), [
      { op: "set_chart_option", slide, element: element.id, patch: stackPatch(element, !stacked) },
    ]);
  return (
    <section className="chart-panel" aria-label={t("ui-chart-panel")}>
      <div className="chart-kinds" role="group" aria-label={t("ui-chart-kinds")}>
        {CHART_KINDS.map((k) => (
          <button key={k} aria-label={kindLabel(k)} title={kindLabel(k)} aria-pressed={current === k} disabled={!source} className={current === k ? "on" : ""} onClick={() => setKind(k)}>
            {KIND_ICON[k]}
          </button>
        ))}
      </div>
      {stackable(element) && (
        <div className="chart-options" role="group" aria-label={t("ui-chart-options")}>
          <button aria-pressed={stacked} className={stacked ? "on" : ""} onClick={toggleStack}>
            {t("ui-chart-stack")}
          </button>
        </div>
      )}
      <SeriesColors
        element={element}
        colors={paletteOf(palette)}
        onColor={(index, color) =>
          onApply(t("ui-undo-chart-color"), [{ op: "set_chart_option", slide, element: element.id, patch: colorPatch(element, index, color) }])
        }
      />
      {current === "waterfall" && <p className="postit-hint">{t("ui-chart-waterfall-hint")}</p>}
      {source ? (
        <ChartDataGrid
          source={source}
          checkboxColumns={checkboxColumns(element)}
          onOp={(change) => onApply(t("ui-undo-chart-data"), [{ op: "set_chart_data", slide, element: element.id, change }])}
        />
      ) : (
        <p className="postit-hint">{t("ui-chart-no-data")}</p>
      )}
    </section>
  );
}
