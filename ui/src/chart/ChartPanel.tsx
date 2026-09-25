// Panneau d'un graphique sélectionné : son type et la grille de ses données.
// Chaque modification part en op vers le cœur, qui redessine le graphique.

import type { Element, Op } from "../api";
import { ChartDataGrid } from "./ChartDataGrid";
import { CHART_KINDS, type ChartKind, chartKind, chartSource, kindPatch } from "./chartDeck";

const KIND_LABELS: Record<ChartKind, string> = { bar: "Barres", line: "Lignes", pie: "Secteurs" };

export function ChartPanel({ slide, element, onApply }: {
  slide: string;
  element: Element;
  onApply: (label: string, ops: Op[]) => void;
}) {
  const source = chartSource(element);
  const current = chartKind(element);
  const setKind = (kind: ChartKind) => {
    const patch = kindPatch(element, kind);
    if (patch) onApply("Changer le type du graphique", [{ op: "set_chart_option", slide, element: element.id, patch }]);
  };
  return (
    <section className="chart-panel" aria-label="Graphique">
      <div className="chart-kinds" role="group" aria-label="Type de graphique">
        {CHART_KINDS.map((k) => (
          <button key={k} aria-pressed={current === k} disabled={!source} className={current === k ? "on" : ""} onClick={() => setKind(k)}>
            {KIND_LABELS[k]}
          </button>
        ))}
      </div>
      {source ? (
        <ChartDataGrid
          source={source}
          onOp={(change) => onApply("Modifier les données", [{ op: "set_chart_data", slide, element: element.id, change }])}
        />
      ) : (
        <p className="postit-hint">Ce graphique n'a pas de tableau de données.</p>
      )}
    </section>
  );
}
