// The data a chart or table view shows: a picker over the data it can switch to, and how many
// views share it. Picking another sends a `bind` edit; its name opens it in the data pane.

import { t } from "../i18n";
import { Icon } from "../icons";
import { Field, IconButton, Select } from "../primitives";
import type { DataChoice, DataRef } from "./chartTypes";
import type { DataEdit } from "./dataEdit";

/** A data's shown name: its label, else its name. */
const shown = (d: { name: string; label?: string }) => d.label ?? d.name;

/** The view's data and the picker over `choices`; a pick other than `data` sends `bind`. */
export function DataPick({ data, choices, onEdit, onOpen }: {
  data: DataRef;
  /** The data the view can switch to, `data` included. */
  choices: readonly DataChoice[];
  onEdit: (edit: DataEdit) => void;
  /** Opens the view's data, by id, in the data pane. */
  onOpen: (data: string) => void;
}) {
  const options = choices.map((c) => ({ value: c.id, label: shown(c) }));
  return (
    <div className="data-pick">
      <Field variant="row" label={t("ui-data-pane")}>
        <Select value={data.id} options={options} disabled={choices.length < 2}
          onChange={(id) => id !== data.id && onEdit({ edit: "bind", data: id })} />
        <IconButton className="token ghost data-open" label={t("ui-data-open")} icon={Icon.table}
          onClick={() => onOpen(data.id)} />
      </Field>
      {data.users > 1 && <p className="data-shared-by a2ui-hint">{t("ui-data-shared-by", { count: data.users })}</p>}
    </div>
  );
}
