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

/** A query's result state (`DataState`, oku-slides-a2ui). */
export type DataState = "pending" | "ready" | "failed";

/** The message of each data kind. */
const KIND_LABEL = { manual: "ui-data-kind-manual", query: "ui-data-kind-query" } as const;

/** The message of each query state. */
export const STATE_LABEL: Record<DataState, string> = {
  pending: "ui-data-state-pending",
  ready: "ui-data-state-ready",
  failed: "ui-data-state-failed",
};

/** A data of the deck, as the list shows it (`DataRow`, oku-slides-a2ui). */
export interface DataRow {
  id: string;
  name: string;
  label?: string;
  kind: "manual" | "query";
  columns: number;
  rows: number;
  users: readonly DataUser[];
  /** A query's result state; absent for a data entered by hand. */
  state?: DataState;
  /** The last action's refusal on this data, until the next edit. */
  error?: string;
}

/** The list and its Add buttons; each gesture is sent as a panel action (`onAction`). */
export function DataListView({ rows, onAction }: { rows: readonly DataRow[]; onAction: (a: DataPanelAction) => void }) {
  return (
    <div className="data-list">
      <div className="data-list-adds">
        <Button ghost className="data-list-add" onClick={() => onAction({ action: "add" })}>
          {Icon.add} {t("ui-data-add")}
        </Button>
        <Button ghost className="data-list-add-query" onClick={() => onAction({ action: "add-query" })}>
          {Icon.add} {t("ui-data-add-query")}
        </Button>
      </div>
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
      <p className="data-row-size a2ui-hint">
        <span className="data-row-kind">{t(KIND_LABEL[row.kind])}</span>
        {row.state && <span className={`data-row-state is-${row.state}`}>{t(STATE_LABEL[row.state])}</span>}
        <span>{t("ui-data-size", { columns: row.columns, rows: row.rows })}</span>
      </p>
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
