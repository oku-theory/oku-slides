// Composants de données de la surface `host-data` (hôte seul) : `ChartData` et `TableData`.
// Rendus par les vues des graphiques et des tableaux ; un geste écrit sa modification dans
// `edit` (modèle de données), puis envoie l'action, que la session applique.

import { useContext } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { CommonSchemas } from "@a2ui/web_core/v0_9";
import { ChartDataView } from "../chart/ChartPanel";
import type { DataEdit } from "../chart/dataEdit";
import { TableDataView } from "../table/TablePanel";
import { ActionOutcomeContext, sendWithOutcome } from "./actionOutcome";
import { Hex } from "./schemas";

/** Une cellule de `dataset.source` : nombre, texte, case à cocher ou vide. */
const Cell = z.union([z.number(), z.string(), z.boolean(), z.null()]);
const Source = z.array(z.array(Cell));

/** Schéma de `ChartData` : l'hôte envoie toujours des littéraux, sauf `edit` (`/edit`). */
export const ChartDataApi = {
  name: "ChartData",
  schema: z
    .object({
      kind: z.string().optional(),
      source: Source,
      checkbox_columns: z.array(z.number().int().nonnegative()),
      stacked: z.boolean().optional(),
      series: z.array(z.object({ name: z.string(), color: Hex.optional() }).strict()),
      palette: z.array(Hex).readonly(),
      edit: CommonSchemas.DynamicValue,
      action: CommonSchemas.Action,
    })
    .strict(),
};

/** Schéma de `TableData`, comme `ChartData`. */
export const TableDataApi = {
  name: "TableData",
  schema: z
    .object({
      source: Source,
      header_row: z.boolean(),
      header_column: z.boolean(),
      striped: z.boolean(),
      align: z.array(z.string()).readonly(),
      edit: CommonSchemas.DynamicValue,
      action: CommonSchemas.Action,
    })
    .strict(),
};

/** Écrit la modification dans `edit`, puis envoie l'action ; son issue revient à la vue. */
function useSendEdit(props: { setEdit: unknown; action: () => void }) {
  const outcome = useContext(ActionOutcomeContext);
  return (edit: DataEdit) => {
    // `DynamicValue` est typé sans objet ; le modèle de données en garde un tel quel.
    (props.setEdit as (v: unknown) => void)(edit);
    return sendWithOutcome(outcome, () => props.action());
  };
}

/** Les données d'un graphique et ses réglages. */
export const ChartData = createComponentImplementation(ChartDataApi, ({ props }) => {
  const onEdit = useSendEdit(props);
  return (
    <ChartDataView
      kind={props.kind}
      source={props.source}
      checkboxColumns={props.checkbox_columns}
      stacked={props.stacked}
      series={props.series}
      palette={props.palette}
      onEdit={(e) => void onEdit(e)}
    />
  );
});

/** Les données d'un tableau et ses options d'affichage. */
export const TableData = createComponentImplementation(TableDataApi, ({ props }) => {
  const onEdit = useSendEdit(props);
  return (
    <TableDataView
      source={props.source}
      headerRow={props.header_row}
      headerColumn={props.header_column}
      striped={props.striped}
      align={props.align}
      onEdit={(e) => void onEdit(e)}
    />
  );
});
