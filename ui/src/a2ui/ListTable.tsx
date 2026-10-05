// Tableau d'une liste au dock : une ligne par item, colonnes du schéma.
// Contrôlé : il émet un ListOp (ou un remplacement au collage) ; le cœur
// applique, et les lignes reviennent par `rows`. Rien n'est gardé localement
// hors du brouillon d'une cellule, du popover ouvert et des cellules signalées.
//
// TanStack Table v9 (`useTable` + `tableFeatures`), comme ChartDataGrid ; la
// poignée et « × » sont hors des colonnes du schéma.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ClipboardEvent, type KeyboardEvent, type PointerEvent, type RefObject } from "react";
import { createColumnHelper, tableFeatures, useTable } from "@tanstack/react-table";
import type { ListOp, Paragraph } from "../api";
import { newId, textToParagraphs } from "../deck";
import { locale, t } from "../i18n";
import { coerce, parseTsv, type ColumnKind } from "../table/cells";
import { RichTextField } from "./RichTextField";
import "./ListTable.css";

/** Une colonne du tableau (miroir de `ListColumn` côté Rust). */
export type ListColumn = {
  key: string;
  label: string;
  kind: ColumnKind;
  options: readonly { id: string; label: string }[];
};
export type ListRow = { id: string } & Record<string, unknown>;
export type ListChange = ListOp | { op: "replace"; items: ListRow[] };

export type ListTableProps = {
  label: string;
  rows: ListRow[];
  columns: readonly ListColumn[];
  min: number;
  max: number | null;
  itemLabel: string | null;
  palette: string[];
  /** Markdown d'édition des cellules `rich`, par id d'item puis colonne. */
  markdown: Record<string, Record<string, string>>;
  /** Émet un changement ; rend `true` une fois accepté par la session, `false` si refusé. */
  onChange: (change: ListChange) => Promise<boolean>;
};

/** Durée du signalement d'une cellule refusée. */
const INVALID_MS = 2000;

const features = tableFeatures({});
const helper = createColumnHelper<typeof features, ListRow>();

/** Ce qu'une cellule reçoit de TanStack et lit ici. */
type CellContext = { row: { original: ListRow; index: number }; getValue: () => unknown };

/** Adresse d'une cellule : `id/colonne`. */
const at = (row: string, col: string) => `${row}/${col}`;

export function ListTable({ label, rows, columns, min, max, itemLabel, palette, markdown, onChange }: ListTableProps) {
  const item = itemLabel ?? t("ui-list-item");
  const full = max !== null && rows.length >= max;
  const [invalid, flagged] = useInvalid();
  const [rich, setRich] = useState<string | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const dragged = useRef<string | null>(null);

  const lt = useRef<HTMLDivElement>(null);

  const add = () => {
    if (!full) void onChange({ op: "insert", id: newId("i") });
  };
  const move = (id: string, to: number) => {
    if (to >= 0 && to < rows.length && rows[to].id !== id) void onChange({ op: "move", item: id, to });
  };
  const set = (row: ListRow, c: ListColumn, value: unknown) => onChange({ op: "set_field", item: row.id, field: c.key, value });

  /** Bloc collé à partir de la ligne `r`, colonne `c` : lignes ajoutées jusqu'à `max`. */
  const pasteAt = (r: number, c: number, grid: string[][]) => {
    const items = rows.map((row) => ({ ...row }));
    const bad: string[] = [];
    grid.forEach((cells, dr) => {
      if (r + dr >= items.length) {
        if (max !== null && items.length >= max) return;
        items.push({ id: newId("i") });
      }
      const target = items[r + dr];
      cells.forEach((raw, dc) => {
        const column = columns[c + dc];
        if (!column) return;
        const v = cellValue(column, raw);
        if (v.ok) target[column.key] = v.value;
        else bad.push(at(target.id, column.key));
      });
    });
    flagged(bad);
    void onChange({ op: "replace", items });
  };

  /** Le contenu d'une cellule ; lu par les colonnes, qui ne changent qu'avec le schéma. */
  const renderCell = (c: ListColumn, ci: number, ctx: CellContext) => {
    const row = ctx.row.original;
    const r = ctx.row.index;
    const value = ctx.getValue();
    switch (c.kind) {
      case "flag":
        return (
          <input type="checkbox" className="lt-check" aria-label={c.label} checked={value === true}
            onChange={(e) => void set(row, c, e.currentTarget.checked)} />
        );
      case "select": {
        const known = c.options.some((o) => o.id === value);
        return (
          <select className="lt-select" aria-label={c.label} value={known ? String(value) : ""}
            onChange={(e) => void set(row, c, e.currentTarget.value)}>
            {!known && <option value="" disabled />}
            {c.options.map((o, i) => <option key={`${i}-${o.id}`} value={o.id}>{o.label}</option>)}
          </select>
        );
      }
      case "rich": {
        const paragraphs = Array.isArray(value) ? (value as Paragraph[]) : [];
        const where = at(row.id, c.key);
        return (
          <button type="button" className="lt-rich" data-list-rich={where} aria-label={c.label} aria-expanded={rich === where}
            onClick={() => setRich(rich === where ? null : where)}>
            {paragraphs[0]?.runs.map((run) => run.t).join("") ?? ""}
          </button>
        );
      }
      case "asset": {
        const id = typeof value === "string" ? value : "";
        return <span className="lt-asset" title={id}>{id.length > 16 ? `${id.slice(0, 15)}…` : id}</span>;
      }
      default:
        return (
          <CellText
            value={value}
            column={c}
            onCommit={(v) => set(row, c, v)}
            onInvalid={() => flagged([at(row.id, c.key)])}
            onEnter={() => r === rows.length - 1 && add()}
            onPasteGrid={(grid) => pasteAt(r, ci, grid)}
          />
        );
    }
  };
  const renderRef = useRef(renderCell);
  renderRef.current = renderCell;
  // Colonnes stables, par contenu : une définition nouvelle remonterait les cellules (brouillon et focus perdus).
  const schemaKey = JSON.stringify(columns);
  const tableColumns = useMemo(
    () =>
      helper.columns(
        columns.map((c, ci) =>
          helper.accessor((row) => row[c.key], {
            id: c.key,
            header: () => c.label,
            cell: (ctx) => renderRef.current(c, ci, ctx),
          }),
        ),
      ),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `columns` suit `schemaKey`.
    [schemaKey],
  );
  const table = useTable({ features, columns: tableColumns, data: rows });

  const onHandleKey = (e: KeyboardEvent<HTMLButtonElement>, id: string, r: number) => {
    if (!e.altKey || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
    e.preventDefault();
    e.stopPropagation();
    move(id, e.key === "ArrowUp" ? r - 1 : r + 1);
  };
  /** La ligne sous le pointeur. */
  const rowAt = (e: PointerEvent) =>
    document.elementFromPoint?.(e.clientX, e.clientY)?.closest("tr[data-row]")?.getAttribute("data-row") ?? null;
  const endDrag = (e: PointerEvent) => {
    const id = dragged.current;
    dragged.current = null;
    setOver(null);
    const target = rowAt(e);
    if (id && target) move(id, rows.findIndex((row) => row.id === target));
  };

  const open = rich ? openCell(rich, rows, columns) : null;

  return (
    <div ref={lt} className="lt note-field">
      <span className="a2ui-hint">{label}</span>
      <div className="lt-scroll">
        <table className="lt-table">
          <thead>
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                <th className="lt-gutter" />
                {group.headers.map((h) => (
                  <th key={h.id}>
                    <table.FlexRender header={h} />
                  </th>
                ))}
                <th className="lt-gutter" />
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.original.id} data-row={row.original.id} data-over={over === row.original.id ? "" : undefined}>
                <td className="lt-gutter">
                  <button
                    type="button"
                    className="lt-handle"
                    data-list-handle
                    aria-label={t("ui-list-move", { item })}
                    title={t("ui-list-move", { item })}
                    onKeyDown={(e) => onHandleKey(e, row.original.id, row.index)}
                    onPointerDown={(e) => {
                      e.currentTarget.setPointerCapture?.(e.pointerId);
                      dragged.current = row.original.id;
                    }}
                    onPointerMove={(e) => dragged.current && setOver(rowAt(e))}
                    onPointerUp={endDrag}
                    onPointerCancel={() => {
                      dragged.current = null;
                      setOver(null);
                    }}
                  >
                    ⋮⋮
                  </button>
                </td>
                {row.getAllCells().map((cell) => (
                  <td key={cell.id} data-col={cell.column.id}
                    data-invalid={invalid.has(at(row.original.id, cell.column.id)) ? "" : undefined}>
                    <table.FlexRender cell={cell} />
                  </td>
                ))}
                <td className="lt-gutter">
                  <button
                    type="button"
                    className="lt-remove"
                    data-list-remove
                    aria-label={t("ui-list-remove", { item })}
                    title={t("ui-list-remove", { item })}
                    disabled={rows.length <= min}
                    onClick={() => void onChange({ op: "remove", item: row.original.id })}
                  >
                    ×
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {open && (
        <RichPopover
          key={rich}
          anchor={rich ?? ""}
          container={lt}
          label={open.column.label}
          value={markdown[open.row.id]?.[open.column.key] ?? plainText(open.row[open.column.key])}
          source={Array.isArray(open.row[open.column.key]) ? (open.row[open.column.key] as Paragraph[]) : []}
          palette={palette}
          onCommit={(paragraphs) => void set(open.row, open.column, paragraphs)}
          onClose={() => setRich(null)}
        />
      )}
      <button
        type="button"
        className="object btn lt-add"
        data-list-add
        disabled={full}
        title={full ? t("ui-list-full", { max: max ?? 0 }) : undefined}
        onClick={add}
      >
        {t("ui-list-add", { item })}
      </button>
    </div>
  );
}

/** La valeur d'une cellule pour une saisie ou un collage : un nombre vide efface, un texte riche devient paragraphes. */
function cellValue(c: ListColumn, raw: string): { ok: true; value: unknown } | { ok: false } {
  if (c.kind === "number" && raw.trim() === "") return { ok: true, value: null };
  const v = coerce(c.kind, raw, c.options.map((o) => o.id));
  if (v.ok && c.kind === "rich") return { ok: true, value: textToParagraphs(String(v.value)) };
  return v;
}

/** Cellules refusées, signalées `INVALID_MS` ; `flag` en ajoute. */
function useInvalid(): [Set<string>, (cells: string[]) => void] {
  const [invalid, setInvalid] = useState<Set<string>>(() => new Set());
  const timers = useRef(new Set<ReturnType<typeof setTimeout>>());
  useEffect(() => {
    const pending = timers.current;
    return () => pending.forEach(clearTimeout);
  }, []);
  const flag = (cells: string[]) => {
    if (cells.length === 0) return;
    setInvalid((prev) => new Set([...prev, ...cells]));
    const timer = setTimeout(() => {
      timers.current.delete(timer);
      setInvalid((prev) => new Set([...prev].filter((c) => !cells.includes(c))));
    }, INVALID_MS);
    timers.current.add(timer);
  };
  return [invalid, flag];
}

/** La cellule riche ouverte, si sa ligne et sa colonne existent encore. */
function openCell(where: string, rows: ListRow[], columns: readonly ListColumn[]) {
  const cut = where.indexOf("/");
  const row = rows.find((r) => r.id === where.slice(0, cut));
  const column = columns.find((c) => c.key === where.slice(cut + 1));
  return row && column ? { row, column } : null;
}

/** Place du popover dans le tableau `box` : sous la cellule, à son bord gauche, sans déborder à droite. */
export function popoverPlace(box: DOMRect, cell: DOMRect, width: number): { left: number; top: number } {
  return { left: Math.max(0, Math.min(cell.left - box.left, box.width - width)), top: cell.bottom - box.top };
}

/** Texte brut de paragraphes (repli sans markdown de la session). */
function plainText(v: unknown): string {
  return Array.isArray(v) ? (v as Paragraph[]).map((p) => p.runs.map((r) => r.t).join("")).join("\n") : "";
}

function display(v: unknown): string {
  if (v === null || v === undefined) return "";
  if (typeof v === "number") return v.toLocaleString(locale(), { maximumFractionDigits: 10, useGrouping: false });
  return String(v);
}

type CellTextProps = {
  value: unknown;
  column: ListColumn;
  /** Enregistre la valeur ; rend `false` si la session l'a refusée. */
  onCommit: (value: unknown) => Promise<boolean>;
  onInvalid: () => void;
  onEnter: () => void;
  onPasteGrid: (grid: string[][]) => void;
};

/**
 * Cellule texte ou nombre : validée à la sortie ou sur Entrée, Échap rétablit. Une saisie
 * refusée (invalide, ou par la session) revient à la valeur des lignes ; Entrée ne
 * passe à la suite (`onEnter`) qu'une fois la saisie enregistrée.
 */
function CellText({ value, column, onCommit, onInvalid, onEnter, onPasteGrid }: CellTextProps) {
  const shown = display(value);
  const number = column.kind === "number";
  // Entrée valide elle-même : la sortie qu'elle provoque ne revalide pas.
  const committing = useRef(false);
  const commit = (el: HTMLInputElement): Promise<boolean> => {
    const v = cellValue(column, el.value);
    if (!v.ok) {
      el.value = shown;
      onInvalid();
      return Promise.resolve(false);
    }
    if (display(v.value) === shown) return Promise.resolve(true);
    return onCommit(v.value).then((ok) => {
      // Refusé : les lignes n'ont pas changé, la clé non plus ; le brouillon est effacé ici.
      if (!ok) el.value = shown;
      return ok;
    });
  };
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    // Les raccourcis de l'app ne doivent pas voir la saisie.
    e.stopPropagation();
    if (e.key === "Enter") {
      const el = e.currentTarget;
      const saved = commit(el);
      committing.current = true;
      el.blur();
      committing.current = false;
      void saved.then((ok) => ok && onEnter());
    } else if (e.key === "Escape") {
      e.currentTarget.value = shown;
      e.currentTarget.blur();
    }
  };
  const onPaste = (e: ClipboardEvent<HTMLInputElement>) => {
    const text = e.clipboardData.getData("text/plain");
    if (!/[\t\n]/.test(text.trim())) return;
    e.preventDefault();
    onPasteGrid(parseTsv(text));
  };
  return (
    <input
      // La clé réinitialise le brouillon quand la valeur change côté Rust.
      key={shown}
      defaultValue={shown}
      aria-label={column.label}
      className={number ? "lt-input lt-number" : "lt-input"}
      inputMode={number ? "decimal" : "text"}
      onBlur={(e) => {
        if (!committing.current) void commit(e.currentTarget);
      }}
      onKeyDown={onKeyDown}
      onPaste={onPaste}
    />
  );
}

type RichPopoverProps = {
  /** Adresse de la cellule (`id/colonne`) sous laquelle il se place. */
  anchor: string;
  /** Le tableau, dont il ne déborde pas. */
  container: RefObject<HTMLDivElement | null>;
  label: string;
  value: string;
  source: Paragraph[];
  palette: string[];
  onCommit: (paragraphs: Paragraph[]) => void;
  onClose: () => void;
};

/**
 * Popover d'une cellule riche, sous sa cellule et dans le tableau ; un clic ailleurs le ferme
 * (le champ valide à sa sortie).
 */
function RichPopover({ anchor, container, label, value, source, palette, onCommit, onClose }: RichPopoverProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null);
  // Placé sous la cellule, borné au tableau ; replacé quand le tableau défile.
  useLayoutEffect(() => {
    const box = container.current;
    const scroller = box?.querySelector(".lt-scroll");
    const update = () => {
      const cell = box?.querySelector(`[data-list-rich="${CSS.escape(anchor)}"]`)?.closest("td");
      if (!box || !cell || !ref.current) return;
      setPlace(popoverPlace(box.getBoundingClientRect(), cell.getBoundingClientRect(), ref.current.offsetWidth));
    };
    update();
    scroller?.addEventListener("scroll", update);
    return () => scroller?.removeEventListener("scroll", update);
  }, [anchor, container]);
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const outside = (e: Event) => {
      const target = e.target as Element | null;
      if (ref.current?.contains(target) || target?.closest?.("[data-list-rich]")) return;
      close.current();
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, []);
  return (
    <div ref={ref} className="lt-popover" data-anchor={anchor} style={place ?? undefined}>
      <RichTextField label={label} value={value} source={source} palette={palette} onCommit={(_md, paragraphs) => onCommit(paragraphs)} />
    </div>
  );
}
