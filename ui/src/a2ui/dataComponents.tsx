// Data components (host only): `ChartData` and `TableData` of the `host-data` surface, `DataList`
// of `deck-data` and `DataSheet` of `data-sheet`. A gesture writes its edit or panel action into
// `edit` (data model), then sends the action, which the session or the UI applies.

import { useContext } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { CommonSchemas } from "@a2ui/web_core/v0_9";
import { ChartDataView } from "../chart/ChartPanel";
import { CellSchema, DataChoicesSchema, DataRefSchema } from "../chart/chartTypes";
import type { DataEdit } from "../chart/dataEdit";
import type { DataPanelAction } from "../dataActions";
import { DataListView } from "../DataList";
import { DataSheetView } from "../DataSheet";
import { TableDataView } from "../table/TablePanel";
import { ActionOutcomeContext, sendWithOutcome } from "./actionOutcome";
import { Hex } from "./schemas";

const Source = z.array(z.array(CellSchema));
/** Ids of the data's rows (after the header) or columns, which edits name. */
const Ids = z.array(z.string()).readonly();

/** `ChartData` schema: the host always sends literals, except `edit` (`/edit`). */
export const ChartDataApi = {
  name: "ChartData",
  schema: z
    .object({
      kind: z.string().optional(),
      source: Source,
      row_ids: Ids,
      column_ids: Ids,
      checkbox_columns: z.array(z.number().int().nonnegative()),
      stacked: z.boolean().optional(),
      series: z.array(z.object({ name: z.string(), color: Hex.optional() }).strict()),
      palette: z.array(Hex).readonly(),
      data: DataRefSchema,
      choices: DataChoicesSchema,
      edit: CommonSchemas.DynamicValue,
      action: CommonSchemas.Action,
    })
    .strict(),
};

/** `TableData` schema, like `ChartData`. */
export const TableDataApi = {
  name: "TableData",
  schema: z
    .object({
      source: Source,
      row_ids: Ids,
      column_ids: Ids,
      header_row: z.boolean(),
      header_column: z.boolean(),
      striped: z.boolean(),
      align: z.array(z.string()).readonly(),
      data: DataRefSchema,
      choices: DataChoicesSchema,
      edit: CommonSchemas.DynamicValue,
      action: CommonSchemas.Action,
    })
    .strict(),
};

/** Ids are unique, so an action names a single data. */
const uniqueIds = <T extends { id: string }>(items: readonly T[]) => new Set(items.map((i) => i.id)).size === items.length;

const DataUserSchema = z.object({ slide: z.string(), element: z.string(), label: z.string() }).strict();
const Count = z.number().int().nonnegative();

/** `DataList` schema: the deck's data, by unique id. */
export const DataListApi = {
  name: "DataList",
  schema: z
    .object({
      rows: z
        .array(
          z
            .object({
              id: z.string(),
              name: z.string(),
              label: z.string().optional(),
              kind: z.literal("manual"),
              columns: Count,
              rows: Count,
              users: z.array(DataUserSchema).readonly(),
              error: z.string().optional(),
            })
            .strict(),
        )
        .readonly()
        .refine(uniqueIds, { message: "data ids must be unique" }),
      edit: CommonSchemas.DynamicValue,
      action: CommonSchemas.Action,
    })
    .strict(),
};

const ColumnTypeSchema = z.enum(["text", "number", "integer", "bool", "date"]);

/** `DataSheet` schema: a page of one data's rows, every column typed. */
export const DataSheetApi = {
  name: "DataSheet",
  schema: z
    .object({
      data: DataRefSchema,
      source: Source,
      row_ids: Ids,
      column_ids: Ids,
      types: z.array(ColumnTypeSchema).readonly(),
      offset: Count,
      total: Count,
      edit: CommonSchemas.DynamicValue,
      action: CommonSchemas.Action,
    })
    .strict(),
};

/** Writes the edit into `edit`, then sends the action; its outcome comes back to the view. */
function useSendEdit(props: { setEdit: unknown; action: () => void }) {
  const outcome = useContext(ActionOutcomeContext);
  return (edit: DataEdit | DataPanelAction) => {
    // `DynamicValue` is typed without objects; the data model keeps one as is.
    (props.setEdit as (v: unknown) => void)(edit);
    return sendWithOutcome(outcome, () => props.action());
  };
}

/** A chart's data and settings. */
export const ChartData = createComponentImplementation(ChartDataApi, ({ props }) => {
  const onEdit = useSendEdit(props);
  return (
    <ChartDataView
      kind={props.kind}
      source={props.source}
      rowIds={props.row_ids}
      columnIds={props.column_ids}
      checkboxColumns={props.checkbox_columns}
      stacked={props.stacked}
      series={props.series}
      palette={props.palette}
      data={props.data}
      choices={props.choices}
      onEdit={(e) => void onEdit(e)}
      onOpen={(data) => void onEdit({ action: "open-data", data })}
    />
  );
});

/** A table's data and display options. */
export const TableData = createComponentImplementation(TableDataApi, ({ props }) => {
  const onEdit = useSendEdit(props);
  return (
    <TableDataView
      source={props.source}
      rowIds={props.row_ids}
      columnIds={props.column_ids}
      headerRow={props.header_row}
      headerColumn={props.header_column}
      striped={props.striped}
      align={props.align}
      data={props.data}
      choices={props.choices}
      onEdit={(e) => void onEdit(e)}
      onOpen={(data) => void onEdit({ action: "open-data", data })}
    />
  );
});

/** The deck's data, in the side panel's Data tab. */
export const DataList = createComponentImplementation(DataListApi, ({ props }) => {
  const send = useSendEdit(props);
  return <DataListView rows={props.rows} onAction={(a) => void send(a)} />;
});

/** A page of one data's rows, in the data pane. */
export const DataSheet = createComponentImplementation(DataSheetApi, ({ props }) => {
  const send = useSendEdit(props);
  return (
    <DataSheetView
      data={props.data}
      source={props.source}
      rowIds={props.row_ids}
      columnIds={props.column_ids}
      types={props.types}
      offset={props.offset}
      total={props.total}
      onAction={(a) => void send(a)}
    />
  );
});
