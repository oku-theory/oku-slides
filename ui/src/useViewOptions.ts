// Options d'affichage : grille et fantômes de la scène, largeurs des colonnes, gardés par l'hôte pour la reprise.

import { useEffect, useState } from "react";
import type { MenuEntry } from "./ActionMenu";
import type { UiState } from "./api";
import { t } from "./i18n";

/** Option d'affichage qu'une touche bascule : ⌘' la grille, ⌘⇧G les fantômes. */
export function viewToggle(e: { key: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }): "grid" | "ghosts" | null {
  if (!e.metaKey && !e.ctrlKey) return null;
  if (e.key === "'") return "grid";
  return e.shiftKey && e.key.toLowerCase() === "g" ? "ghosts" : null;
}

type KeyLike = Parameters<typeof viewToggle>[0] & { preventDefault(): void };

/** Largeurs choisies aux séparateurs, en px ; `null` : largeur par défaut. */
export type ColumnWidths = Pick<UiState, "pile" | "notes">;

/** Grille et fantômes de la scène, leurs entrées de menu et leurs raccourcis, largeurs des colonnes. */
export interface ViewOptions {
  grid: boolean;
  ghosts: boolean;
  columns: ColumnWidths;
  /** Retient de nouvelles largeurs (et les fait garder par l'hôte). */
  setColumns: (columns: ColumnWidths) => void;
  entries: MenuEntry[];
  /** Prend ⌘' / ⌘⇧G (et empêche l'action par défaut) ; dit s'il a pris la touche. */
  handleKey: (e: KeyLike) => boolean;
}

/**
 * Grille, fantômes et colonnes, repris de l'état gardé `kept` au premier
 * snapshot, puis sauvés par `save` avec la slide `current` à chaque changement.
 * `save` doit rester la même fonction (`api.ui`) : un changement de sa seule
 * référence ne relance pas la sauvegarde.
 */
export function useViewOptions(
  kept: UiState | null,
  current: string | null,
  save: (ui: UiState) => Promise<unknown>,
): ViewOptions {
  const [grid, setGrid] = useState(true);
  const [ghosts, setGhosts] = useState(false);
  const [columns, setColumns] = useState<ColumnWidths>({ pile: null, notes: null });
  // Reprise unique ; un état, pas une référence : la sauvegarde attend le rendu
  // où les choix repris sont en place, sans écraser d'abord l'état gardé par les défauts.
  const [restored, setRestored] = useState(false);
  useEffect(() => {
    if (!kept || restored) return;
    setGrid(kept.grid);
    setGhosts(kept.ghosts);
    setColumns({ pile: kept.pile, notes: kept.notes });
    setRestored(true);
  }, [kept, restored]);

  useEffect(() => {
    if (!restored) return;
    save({ slide: current, grid, ghosts, ...columns }).catch(() => {
      /* reprise facultative : l'édition continue */
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `save` est stable (voir plus haut)
  }, [restored, current, grid, ghosts, columns]);

  const flip = (which: "grid" | "ghosts") => (which === "grid" ? setGrid : setGhosts)((v) => !v);
  const entries: MenuEntry[] = [
    { kind: "item", label: t(grid ? "ui-toolbar-grid-hide" : "ui-toolbar-grid-show"), shortcut: "⌘'", onSelect: () => flip("grid") },
    { kind: "item", label: t(ghosts ? "ui-toolbar-ghosts-hide" : "ui-toolbar-ghosts-show"), shortcut: "⌘⇧G", onSelect: () => flip("ghosts") },
  ];
  const handleKey = (e: KeyLike) => {
    const which = viewToggle(e);
    if (!which) return false;
    e.preventDefault();
    flip(which);
    return true;
  };
  return { grid, ghosts, columns, setColumns, entries, handleKey };
}
