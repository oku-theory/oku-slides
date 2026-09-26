// Panneau d'un graphique sélectionné : son type et la grille de ses données.
// Chaque modification part en op vers le cœur, qui redessine le graphique.

import type { Element, Op } from "../api";
import { t } from "../i18n";
import { ChartDataGrid } from "./ChartDataGrid";
import { CHART_KINDS, type ChartKind, chartKind, chartSource, isStacked, stackPatch, stackable } from "./chartDeck";

const kindLabel = (kind: ChartKind) => t("ui-chart-kind", { kind });

export function ChartPanel({ slide, element, onApply, onKind }: {
  slide: string;
  element: Element;
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
          <button key={k} aria-pressed={current === k} disabled={!source} className={current === k ? "on" : ""} onClick={() => setKind(k)}>
            {kindLabel(k)}
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
      {current === "waterfall" && <p className="postit-hint">{t("ui-chart-waterfall-hint")}</p>}
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
