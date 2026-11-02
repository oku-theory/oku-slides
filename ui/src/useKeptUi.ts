// The interface state the host keeps for the resume: current slide, the stage's ghosts,
// column widths, widget code (shown, width), folded sections, a pack tab's selected item and preview slot,
// and whether a deck tab's inspector and data pane are open.

import { useEffect, useState } from "react";
import type { PileSectionName, PreviewSpan, UiOrder, UiState } from "./api";
import { REAL_TIMERS, type Timers } from "./debounce";
import { randomId } from "./randomId";

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

/** Whether the key is ⌘⇧G, which toggles the ghosts; the grid shows only while dragging (t-1283). */
export function viewToggle(e: { key: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }): boolean {
  return (e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "g";
}

/** Whether the key is ⌘⇧E (Ctrl+Shift+E elsewhere), which shows or hides a deck tab's inspector. */
export function inspectorToggle(e: { key: string; metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }): boolean {
  return (e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "e";
}

type KeyLike = Parameters<typeof viewToggle>[0] & { preventDefault(): void };

/** Largeurs choisies aux séparateurs, en px ; `null` : largeur par défaut. */
export type ColumnWidths = Pick<UiState, "pile" | "notes">;

/** L'état d'interface gardé : fantômes (raccourci, agent), colonnes, code des widgets, sections repliées. */
export interface KeptUi {
  ghosts: boolean;
  columns: ColumnWidths;
  /** Retient de nouvelles largeurs (et les fait garder par l'hôte). */
  setColumns: (columns: ColumnWidths) => void;
  /** A pack item's Code tab is open, rather than its Settings tab. */
  code: boolean;
  /** Opens the Code or the Settings tab (and has the host keep it). */
  setCode: (code: boolean) => void;
  /** Folded sections (thumbnails). */
  folded: PileSectionName[];
  /** Replie ou rouvre une section (et le fait garder par l'hôte). */
  setFolded: (section: PileSectionName, folded: boolean) => void;
  /** A pack tab's preview slot for element widgets. */
  previewSpan: PreviewSpan;
  /** Picks the preview slot (and has the host keep it). */
  setPreviewSpan: (span: PreviewSpan) => void;
  /** Montre ou cache les repères des champs vides (demandé par l'agent). */
  setGhosts: (on: boolean) => void;
  /** A deck tab's inspector column is open. */
  inspector: boolean;
  /** Opens or closes the inspector (and has the host keep it). */
  setInspector: (open: boolean) => void;
  /** A deck tab's data pane under the stage is open. */
  dataPane: boolean;
  /** Opens or closes the data pane (and has the host keep it). */
  setDataPane: (open: boolean) => void;
  /**
   * Takes ⌘⇧G, and ⌘⇧E in a deck tab (`deck`), preventing the default; says whether it took the key.
   */
  handleKey: (e: KeyLike, deck?: boolean) => boolean;
}

/**
 * Fantômes, colonnes, code des widgets et sections repliées, repris de l'état gardé `kept` au premier
 * snapshot, puis sauvés par `save` avec la slide `current` à chaque changement, tout de suite, numérotés
 * (`UiOrder`) pour que l'hôte garde la dernière quel que soit l'ordre d'arrivée.
 * `item`: a pack tab's selected item, kept with the rest. Une sauvegarde échouée est renvoyée une fois (`timers` : tests).
 * `save` doit rester la même fonction (`api.ui`) : un changement de sa seule
 * référence ne relance pas la sauvegarde.
 */
export function useKeptUi(
  kept: UiState | null,
  current: string | null,
  save: (ui: UiState, order: UiOrder) => Promise<unknown>,
  item: Pick<UiState, "widget">,
  timers: Timers = REAL_TIMERS,
): KeptUi {
  const [ghosts, setGhosts] = useState(false);
  const [columns, setColumns] = useState<ColumnWidths>({ pile: null, notes: null });
  const [code, setCode] = useState(false);
  const [folded, setFoldedList] = useState<PileSectionName[]>([]);
  const [previewSpan, setPreviewSpan] = useState<PreviewSpan>("full");
  const [inspector, setInspector] = useState(false);
  const [dataPane, setDataPane] = useState(false);
  // Reprise unique ; un état, pas une référence : la sauvegarde attend le rendu
  // où les choix repris sont en place, sans écraser d'abord l'état gardé par les défauts.
  const [restored, setRestored] = useState(false);
  useEffect(() => {
    if (!kept || restored) return;
    setGhosts(kept.ghosts);
    setColumns({ pile: kept.pile, notes: kept.notes });
    setCode(kept.code);
    setFoldedList(kept.folded);
    setPreviewSpan(kept.preview_span);
    setInspector(kept.inspector);
    setDataPane(kept.data_pane);
    setRestored(true);
  }, [kept, restored]);

  useEffect(() => {
    if (!restored) return;
    const ui = { slide: current, ghosts, ...columns, widget: item.widget, code, folded, preview_span: previewSpan, inspector, data_pane: dataPane };
    send(save, ui, timers);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `save` et `timers` sont stables (voir plus haut)
  }, [restored, current, ghosts, columns, item.widget, code, folded, previewSpan, inspector, dataPane]);

  // Même état : même tableau, pas de nouvelle sauvegarde.
  const setFolded = (section: PileSectionName, fold: boolean) =>
    setFoldedList((list) => (list.includes(section) === fold ? list : fold ? [...list, section] : list.filter((s) => s !== section)));
  const handleKey = (e: KeyLike, deck = false) => {
    const toggle = viewToggle(e) ? setGhosts : deck && inspectorToggle(e) ? setInspector : null;
    if (!toggle) return false;
    e.preventDefault();
    toggle((v) => !v);
    return true;
  };
  return {
    ghosts, columns, setColumns, code, setCode, folded, setFolded, previewSpan, setPreviewSpan, setGhosts,
    inspector, setInspector, dataPane, setDataPane, handleKey,
  };
}
