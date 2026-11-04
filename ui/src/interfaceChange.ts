// What the agent changes in the interface (`set_interface` tool, t-988, t-1001), applied through
// the app's own commands: scheme and language for every window, guides and layout for the tab.

import type { InterfaceChange, PileSectionName } from "./api";
import { fitIn } from "./bounds";
import { columnBounds, measureColumns } from "./columns";
import type { SchemeChoice } from "./scheme";
import type { ColumnWidths, KeptUi } from "./useKeptUi";

/** The commands a change goes through; injected for the tests. */
export interface InterfaceCommands {
  changeScheme: (choice: SchemeChoice) => Promise<void>;
  changeLocale: (code: string | null) => Promise<void>;
  setGhosts: (on: boolean) => void;
  /** Folds or opens a pile section (kept by the host). */
  setFolded: (section: PileSectionName, folded: boolean) => void;
  /** Opens a pack item's Code or Settings tab (kept by the host). */
  setCode: (code: boolean) => void;
  /** The widths chosen so far; `null`: the default. */
  columns: ColumnWidths;
  /** Keeps new widths (and has the host keep them). */
  setColumns: (columns: ColumnWidths) => void;
  /** The live widths of the table and its pile, in px. */
  measure: () => { table: number; pile: number };
  /** Reports a command's failure. */
  fail: (err: unknown) => void;
}

/** The layout commands of `kept`, the table measured in the page. */
export function layoutCommands(kept: KeptUi): Pick<InterfaceCommands, "setGhosts" | "setFolded" | "setCode" | "columns" | "setColumns" | "measure"> {
  return {
    setGhosts: kept.setGhosts,
    setFolded: kept.setFolded,
    setCode: kept.setCode,
    columns: kept.columns,
    setColumns: kept.setColumns,
    measure: () => measureColumns(document.querySelector<HTMLElement>(".table")),
  };
}

/**
 * Applies `c`: `system` means the system's (`null`), an absent field changes nothing. Widths are
 * clamped to the live bounds, as at the dividers: the notes against the pile once it is set.
 * Without a measured table (0 px, still loading) the asked widths are kept as they are: Rust
 * already checked the static bounds, and the grid's CSS clamps what it shows.
 */
export function applyInterface(c: InterfaceChange, cmd: InterfaceCommands): void {
  if (c.scheme != null) cmd.changeScheme(c.scheme === "system" ? null : c.scheme).catch(cmd.fail);
  if (c.locale != null) cmd.changeLocale(c.locale === "system" ? null : c.locale).catch(cmd.fail);
  if (c.guides != null) cmd.setGhosts(c.guides);
  if (c.folded?.slides != null) cmd.setFolded("slides", c.folded.slides);
  if (c.folded?.layouts != null) cmd.setFolded("layouts", c.folded.layouts);
  if (c.code != null) cmd.setCode(c.code);
  if (c.pile != null || c.notes != null) {
    const { table, pile: shown } = cmd.measure();
    const fit = (side: "pile" | "notes", size: number, pile: number) =>
      table > 0 ? fitIn(columnBounds(side, table, pile), size) : size;
    const pile = c.pile != null ? fit("pile", c.pile, shown) : cmd.columns.pile;
    const notes = c.notes != null ? fit("notes", c.notes, pile ?? shown) : cmd.columns.notes;
    cmd.setColumns({ pile, notes });
  }
}
