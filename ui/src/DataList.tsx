// The deck's data, listed in the side panel's Data tab (`deck-data` surface): each data with its
// size and the views that show it, renamed or removed in place; a refusal shows on its row.

import type { DataPanelAction } from "./dataActions";
import { t } from "./i18n";
import { Icon } from "./icons";
import { Button, IconButton } from "./primitives";

/** A view that shows a data: its element, and how the list names it. */
export interface DataUser {
  slide: string;
  element: string;
  label: string;
}

/** A data of the deck, as the list shows it (`DataRow`, oku-slides-a2ui). */
export interface DataRow {
  id: string;
  name: string;
  label?: string;
  kind: string;
  columns: number;
  rows: number;
  users: readonly DataUser[];
  /** The last action's refusal on this data, until the next edit. */
  error?: string;
}

/** The list and its Add button; each gesture is sent as a panel action (`onAction`). */
export function DataListView({ rows, onAction }: { rows: readonly DataRow[]; onAction: (a: DataPanelAction) => void }) {
  return (
    <div className="data-list">
      <Button ghost className="data-list-add" onClick={() => onAction({ action: "add" })}>
        {Icon.add} {t("ui-data-add")}
      </Button>
      <ul className="data-rows">
        {rows.map((row) => <DataListRow key={row.id} row={row} onAction={onAction} />)}
      </ul>
    </div>
  );
}

function DataListRow({ row, onAction }: { row: DataRow; onAction: (a: DataPanelAction) => void }) {
  // Only the blur sends: Enter blurs, so the pair sends once.
  const rename = (name: string) => {
    const wanted = name.trim();
    if (wanted && wanted !== row.name) onAction({ action: "rename", data: row.id, name: wanted });
  };
  return (
    <li className="data-row" data-testid={`data-row-${row.name}`}>
      <div className="data-row-head">
        <button type="button" className="data-row-name" title={t("ui-data-open")} onClick={() => onAction({ action: "select", data: row.id })}>
          {row.label ?? row.name}
        </button>
        <IconButton className="token ghost data-row-remove" data-testid="data-row-remove" label={t("ui-data-remove")} icon={Icon.trash}
          onClick={() => onAction({ action: "remove", data: row.id })} />
      </div>
      <input
        // A new name from the session resets the field.
        key={row.name}
        className="data-row-rename"
        defaultValue={row.name}
        aria-label={t("ui-data-rename")}
        onKeyDown={(e) => {
          if (e.key === "Escape") e.currentTarget.value = row.name;
          if (e.key === "Enter" || e.key === "Escape") e.currentTarget.blur();
        }}
        onBlur={(e) => rename(e.currentTarget.value)}
      />
      <p className="data-row-size a2ui-hint">{t("ui-data-size", { columns: row.columns, rows: row.rows })}</p>
      {row.users.length > 0 ? (
        <div className="data-row-users">
          <span className="data-row-views a2ui-hint" data-testid="data-row-views">{t("ui-data-views", { count: row.users.length })}</span>
          {row.users.map((u) => (
            <button key={`${u.slide}/${u.element}`} type="button" className="data-row-user"
              onClick={() => onAction({ action: "open-element", slide: u.slide, element: u.element })}>
              {u.label}
            </button>
          ))}
        </div>
      ) : (
        <span className="data-row-unused a2ui-hint">{t("ui-data-unused")}</span>
      )}
      {row.error && <p className="data-row-error" role="alert" data-testid="data-row-error">{row.error}</p>}
    </li>
  );
}
