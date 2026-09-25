// Panneau d'un graphique sélectionné : son type et la grille de ses données.
// Chaque modification part en op vers le cœur, qui redessine le graphique.

import type { Element, Op } from "../api";
import { t } from "../i18n";
import { ChartDataGrid } from "./ChartDataGrid";
import { CHART_KINDS, type ChartKind, chartKind, chartSource, kindPatch } from "./chartDeck";

const kindLabel = (kind: ChartKind) => t("ui-chart-kind", { kind });

export function ChartPanel({ slide, element, onApply }: {
  slide: string;
  element: Element;
  onApply: (label: string, ops: Op[]) => void;
}) {
  const source = chartSource(element);
  const current = chartKind(element);
  const setKind = (kind: ChartKind) => {
    const patch = kindPatch(element, kind);
    if (patch) onApply(t("ui-undo-chart-kind"), [{ op: "set_chart_option", slide, element: element.id, patch }]);
  };
  return (
    <section className="chart-panel" aria-label={t("ui-chart-panel")}>
      <div className="chart-kinds" role="group" aria-label={t("ui-chart-kinds")}>
        {CHART_KINDS.map((k) => (
          <button key={k} aria-pressed={current === k} disabled={!source} className={current === k ? "on" : ""} onClick={() => setKind(k)}>
            {kindLabel(k)}
          </button>
        ))}
      </div>
      {source ? (
        <ChartDataGrid
          source={source}
          onOp={(change) => onApply(t("ui-undo-chart-data"), [{ op: "set_chart_data", slide, element: element.id, change }])}
        />
      ) : (
        <p className="postit-hint">{t("ui-chart-no-data")}</p>
      )}
    </section>
  );
}
