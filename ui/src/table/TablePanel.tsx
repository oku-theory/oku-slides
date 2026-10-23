// Vue des données d'un tableau (composant `TableData` de la surface `host-data`) : options
// d'affichage, alignement des colonnes et grille de ses données (celle des graphiques).
// Chaque geste part en une modification (`DataEdit`) que la session applique.

import type { ReactNode } from "react";

import { ChartDataGrid } from "../chart/ChartDataGrid";
import type { Cell } from "../chart/chartTypes";
import type { DataEdit } from "../chart/dataEdit";
import { t } from "../i18n";
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

const tableColumnName = (n: number) => t("ui-table-column", { n: n + 1 });

/** Les données d'un tableau et ses options ; chaque geste part en une modification (`onEdit`). */
export function TableDataView({ source, headerRow, headerColumn, striped, align, onEdit }: {
  /** dataset.source : ligne 0 = noms de colonnes ; vide sans données. */
  source: Cell[][];
  headerRow: boolean;
  headerColumn: boolean;
  striped: boolean;
  /** Alignement de chaque colonne ; sans entrée : `auto`. */
  align: readonly string[];
  onEdit: (edit: DataEdit) => void;
}) {
  if (source.length === 0) return <p className="a2ui-hint">{t("ui-table-no-data")}</p>;
  const flags: Record<TableFlag, boolean> = { "header-row": headerRow, "header-column": headerColumn, striped };
  const header = source[0] ?? [];
  return (
    <section className="table-panel chart-panel" aria-label={t("ui-table-panel")}>
      <div className="panel-toolbar">
        <div className="table-options chart-options" role="group" aria-label={t("ui-table-options")}>
          {FLAGS.map(({ key, label, icon }) => (
            <IconToggle key={key} label={t(label)} pressed={flags[key]} icon={icon}
              onToggle={() => onEdit({ edit: "flag", flag: key, on: !flags[key] })} />
          ))}
        </div>
        <ShowAsButton view="chart" onShowAs={(view) => onEdit({ edit: "show_as", view })} />
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
      <ChartDataGrid
        source={source}
        firstColumnIsLabel={headerColumn}
        newColumnName={tableColumnName}
        onOp={(change) => onEdit({ edit: "cells", change })}
      />
    </section>
  );
}
