// L'état d'interface gardé par l'hôte pour la reprise : slide courante, grille et fantômes de la scène,
// largeurs des colonnes, code des widgets (affiché, largeur), sections repliées, sous-onglets.

import { useEffect, useState } from "react";
import type { MenuEntry } from "./ActionMenu";
import type { PileSectionName, UiOrder, UiState } from "./api";
import { REAL_TIMERS, type Timers } from "./debounce";
import { t } from "./i18n";
import { randomId } from "./randomId";
import type { KeptTabs } from "./widgetTabs";

// Ordre des sauvegardes : chacune part tout de suite (rien n'attend côté page), et l'hôte ignore celle
// qui arrive après une plus récente. À la fermeture (⌘Q, dernière fenêtre), l'hôte attend les requêtes
// déjà parties avant de s'arrêter : une sauvegarde partie est donc gardée ; seul un changement qui n'a pas
// encore atteint `save` (rendu en cours) peut manquer. `page` est tiré au chargement : une page rechargée
// repart à zéro sans être ignorée. `seq` est celui du module, pas du composant : il survit au remontage
// d'`App` d'un onglet à l'autre.

// L'UUID du navigateur lève hors contexte sûr (`serve` sur une adresse réseau en HTTP) et ce n'est pas un secret :
// `randomId` suffit (t-695).
const page = randomId("page");
let seq = 0;
const nextOrder = (): UiOrder => ({ page, seq: ++seq });

/** Délai avant de renvoyer une sauvegarde échouée, en ms : assez court pour que l'hôte ne garde pas
 *  longtemps l'ancien état, assez long pour laisser passer un à-coup (requête perdue, réponse en erreur). */
export const UI_RESEND_DELAY_MS = 300;

/**
 * Envoie `ui` ; à l'échec, le renvoie une fois après `UI_RESEND_DELAY_MS`, avec le même ordre (t-749) :
 * l'hôte ignore un `seq` déjà vu, un envoi appliqué dont seule la réponse s'est perdue ne double donc rien.
 * Pas de renvoi si une sauvegarde plus récente est partie entre-temps : elle porte un état plus neuf.
 * Un renvoi échoué n'est pas renvoyé : la reprise est facultative, l'édition continue.
 */
function send(save: (ui: UiState, order: UiOrder) => Promise<unknown>, ui: UiState, timers: Timers) {
  const order = nextOrder();
  save(ui, order).catch(() => {
    timers.set(() => {
      if (seq === order.seq) save(ui, order).catch(() => {});
    }, UI_RESEND_DELAY_MS);
  });
}

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
 * snapshot, puis sauvés par `save` avec la slide `current` à chaque changement, tout de suite, numérotés
 * (`UiOrder`) pour que l'hôte garde la dernière quel que soit l'ordre d'arrivée.
 * `tabs` : sous-onglets à garder avec le reste. Une sauvegarde échouée est renvoyée une fois (`timers` : tests).
 * `save` doit rester la même fonction (`api.ui`) : un changement de sa seule
 * référence ne relance pas la sauvegarde.
 */
export function useKeptUi(
  kept: UiState | null,
  current: string | null,
  save: (ui: UiState, order: UiOrder) => Promise<unknown>,
  tabs: KeptTabs,
  timers: Timers = REAL_TIMERS,
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

  useEffect(() => {
    if (!restored) return;
    send(save, { slide: current, grid, ghosts, ...columns, ...tabs, code, code_width: codeWidth, folded }, timers);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `save` et `timers` sont stables (voir plus haut)
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
