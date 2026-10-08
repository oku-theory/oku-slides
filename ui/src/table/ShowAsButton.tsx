// Bouton « Afficher en tableau » / « Afficher en graphique » : la session convertit
// (requête `show_as`), en une étape d'annulation.

import { t } from "../i18n";
import { Icon } from "../icons";

export type View = "chart" | "table";

export function ShowAsButton({ view, onShowAs, disabled }: { view: View; onShowAs: (view: View) => void; disabled?: boolean }) {
  const label = t(view === "table" ? "ui-show-as-table" : "ui-show-as-chart");
  return (
    <div className="show-as chart-options" role="group" aria-label={t("ui-show-as")}>
      <button aria-label={label} title={label} disabled={disabled} onClick={() => onShowAs(view)}>
        {view === "table" ? Icon.table : Icon.chart}
      </button>
    </div>
  );
}
