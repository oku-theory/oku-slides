// Panneau d'un tableau sélectionné : options d'affichage, alignement des colonnes
// et grille de ses données (celle des graphiques). Chaque modification part en op.

import type { ReactNode } from "react";

import type { Element, Op } from "../api";
import { ChartDataGrid } from "../chart/ChartDataGrid";
import { t } from "../i18n";
import { stroke } from "../icons";
import { ShowAsButton, type View } from "./ShowAsButton";
import { COLUMN_ALIGNS, type TableFlag, alignPatch, asColumnAlign, columnAlign, flag, tableOption } from "./tableDeck";

const FLAGS: { key: TableFlag; label: string; icon: ReactNode }[] = [
  { key: "header-row", label: "ui-table-header-row",
    icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 10h16" strokeWidth="3" /></>) },
  { key: "header-column", label: "ui-table-header-column",
    icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M9 5v14" strokeWidth="3" /></>) },
  { key: "striped", label: "ui-table-striped",
    icon: stroke(<><rect x="4" y="5" width="16" height="14" rx="1" /><path d="M4 9.5h16M4 14.5h16" /></>) },
];

const tableColumnName = (n: number) => t("ui-table-column", { n: n + 1 });

export function TablePanel({ slide, element, numberLang, onApply, onShowAs }: {
  slide: string;
  element: Element;
  /** Langue du deck : écriture des nombres de la grille. */
  numberLang?: string | null;
  onApply: (label: string, ops: Op[]) => void;
  /** Affiche le tableau en graphique (requête `show_as`). */
  onShowAs: (view: View) => void;
}) {
  const option = tableOption(element);
  if (!option) return <p className="a2ui-hint">{t("ui-table-no-data")}</p>;
  const target = { slide, element: element.id };
  const patch = (label: string, p: Record<string, unknown>) =>
    onApply(label, [{ op: "set_table_option", target, param: "data", patch: p }]);
  const header = option.dataset.source[0] ?? [];
  return (
    <section className="table-panel chart-panel" aria-label={t("ui-table-panel")}>
      <div className="table-options chart-options" role="group" aria-label={t("ui-table-options")}>
        {FLAGS.map(({ key, label, icon }) => {
          const pressed = flag(option, key);
          return (
            <button key={key} aria-label={t(label)} title={t(label)} aria-pressed={pressed} className={pressed ? "on" : ""}
              onClick={() => patch(t("ui-undo-table-option"), { [key]: !pressed })}>
              {icon}
            </button>
          );
        })}
      </div>
      <ShowAsButton view="chart" onShowAs={onShowAs} />
      <div className="table-align" role="group" aria-label={t("ui-table-alignment")}>
        {header.map((name, i) => {
          const shown = String(name ?? "").trim() || String(i + 1);
          return (
            <label key={i}>
              <span>{shown}</span>
              <select aria-label={t("ui-table-align-of", { name: shown })} value={columnAlign(option, i)}
                onChange={(e) => patch(t("ui-undo-table-align"), alignPatch(option, i, asColumnAlign(e.target.value)))}>
                {COLUMN_ALIGNS.map((a) => <option key={a} value={a}>{t("ui-table-align", { align: a })}</option>)}
              </select>
            </label>
          );
        })}
      </div>
      <ChartDataGrid
        source={option.dataset.source}
        firstColumnIsLabel={flag(option, "header-column")}
        newColumnName={tableColumnName}
        numberLang={numberLang}
        onOp={(change) => onApply(t("ui-undo-table-data"), [{ op: "set_data", target, param: "data", change }])}
      />
    </section>
  );
}
