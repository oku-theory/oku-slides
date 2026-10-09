// L'état d'interface gardé par l'hôte pour la reprise : slide courante, grille et fantômes de la scène,
// largeurs des colonnes, code des widgets (affiché, largeur), sections repliées, sous-onglets.

import { useEffect, useRef, useState } from "react";
import type { MenuEntry } from "./ActionMenu";
import type { PileSectionName, UiState } from "./api";
import { t } from "./i18n";
import type { KeptTabs } from "./widgetTabs";

/** Option d'affichage qu'une touche bascule : ⌘' la grille, ⌘⇧G les fantômes. */
export function viewToggle(e: { key: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }): "grid" | "ghosts" | null {
  if (!e.metaKey && !e.ctrlKey) return null;
  if (e.key === "'") return "grid";
  return e.shiftKey && e.key.toLowerCase() === "g" ? "ghosts" : null;
}

type KeyLike = Parameters<typeof viewToggle>[0] & { preventDefault(): void };

/** Largeurs choisies aux séparateurs, en px ; `null` : largeur par défaut. */
export type ColumnWidths = Pick<UiState, "pile" | "notes">;

/** L'état d'interface gardé : grille et fantômes (entrées de menu, raccourcis), colonnes, code des widgets, sections repliées. */
export interface KeptUi {
  grid: boolean;
  ghosts: boolean;
  columns: ColumnWidths;
  /** Retient de nouvelles largeurs (et les fait garder par l'hôte). */
  setColumns: (columns: ColumnWidths) => void;
  /** Code source des widgets déplié à droite de l'aperçu. */
  code: boolean;
  /** Déplie ou replie le code (et le fait garder par l'hôte). */
  setCode: (code: boolean) => void;
  /** Largeur du code choisie au séparateur, en px ; `null` : proportion par défaut. */
  codeWidth: number | null;
  /** Retient une nouvelle largeur du code (et la fait garder par l'hôte). */
  setCodeWidth: (width: number | null) => void;
  /** Sections repliées (pile, carte des paramètres). */
  folded: PileSectionName[];
  /** Replie ou rouvre une section (et le fait garder par l'hôte). */
  setFolded: (section: PileSectionName, folded: boolean) => void;
  entries: MenuEntry[];
  /** Prend ⌘' / ⌘⇧G (et empêche l'action par défaut) ; dit s'il a pris la touche. */
  handleKey: (e: KeyLike) => boolean;
}

/**
 * Grille, fantômes, colonnes, code des widgets et sections repliées, repris de l'état gardé `kept` au premier
 * snapshot, puis sauvés par `save` avec la slide `current` à chaque changement, une sauvegarde à la fois.
 * `tabs` : sous-onglets à garder avec le reste.
 * `save` doit rester la même fonction (`api.ui`) : un changement de sa seule
 * référence ne relance pas la sauvegarde.
 */
export function useKeptUi(
  kept: UiState | null,
  current: string | null,
  save: (ui: UiState) => Promise<unknown>,
  tabs: KeptTabs,
): KeptUi {
  const [grid, setGrid] = useState(true);
  const [ghosts, setGhosts] = useState(false);
  const [columns, setColumns] = useState<ColumnWidths>({ pile: null, notes: null });
  const [code, setCode] = useState(false);
  const [codeWidth, setCodeWidth] = useState<number | null>(null);
  const [folded, setFoldedList] = useState<PileSectionName[]>([]);
  // Reprise unique ; un état, pas une référence : la sauvegarde attend le rendu
  // où les choix repris sont en place, sans écraser d'abord l'état gardé par les défauts.
  const [restored, setRestored] = useState(false);
  useEffect(() => {
    if (!kept || restored) return;
    setGrid(kept.grid);
    setGhosts(kept.ghosts);
    setColumns({ pile: kept.pile, notes: kept.notes });
    setCode(kept.code);
    setCodeWidth(kept.code_width);
    setFoldedList(kept.folded);
    setRestored(true);
  }, [kept, restored]);

  // Une sauvegarde à la fois : des requêtes parallèles peuvent être traitées par l'hôte dans le
  // désordre, et une plus ancienne écraserait la dernière. Pendant l'attente, seul le dernier état compte.
  const queue = useRef<{ busy: boolean; next: UiState | null }>({ busy: false, next: null });
  const send = (ui: UiState) => {
    const q = queue.current;
    if (q.busy) {
      q.next = ui;
      return;
    }
    q.busy = true;
    save(ui)
      .catch(() => {
        /* reprise facultative : l'édition continue */
      })
      .finally(() => {
        const next = q.next;
        q.busy = false;
        q.next = null;
        if (next) send(next);
      });
  };

  useEffect(() => {
    if (!restored) return;
    send({ slide: current, grid, ghosts, ...columns, ...tabs, code, code_width: codeWidth, folded });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `save` est stable (voir plus haut), `send` ne lit que lui et la file
  }, [restored, current, grid, ghosts, columns, tabs.widgets.join("\n"), tabs.widget, code, codeWidth, folded]);

  // Même état : même tableau, pas de nouvelle sauvegarde.
  const setFolded = (section: PileSectionName, fold: boolean) =>
    setFoldedList((list) => (list.includes(section) === fold ? list : fold ? [...list, section] : list.filter((s) => s !== section)));
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
  return { grid, ghosts, columns, setColumns, code, setCode, codeWidth, setCodeWidth, folded, setFolded, entries, handleKey };
}
