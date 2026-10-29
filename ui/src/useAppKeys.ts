// Les raccourcis clavier d'un onglet, écoutés sur la fenêtre et réabonnés à chaque rendu.

import { useEffect } from "react";
import type { Op, SlideView, Snapshot, TabApi } from "./api";
import { escapeAction } from "./agent/keys";
import type { AgentStatusView } from "./agent/AgentPanel";
import { appKeyScope, themeTabIgnores } from "./appKeyScope";
import { findElement, gridPlacement, placeOnGridOps } from "./deck";
import { moveCell, trackSize } from "./grid";
import type { HistoryCursor } from "./HistoryBar";
import { t } from "./i18n";
import type { PresentMode } from "./present";
import { presentShortcut } from "./present";
import { slideTarget } from "./slideKeys";
import type { FieldKey } from "./stageFields";
import { fieldKey, primaryFieldOf } from "./stageFields";
import type { useFileActions } from "./useFileActions";
import type { KeptUi } from "./useKeptUi";

/** Ce que les raccourcis lisent et font dans l'onglet. */
interface AppKeysContext {
  api: TabApi;
  snap: Snapshot | null;
  view: SlideView | null;
  current: string | null;
  selected: string | null;
  editing: FieldKey | null;
  presenting: PresentMode | null;
  agentStatus: AgentStatusView;
  dialogOpen: boolean;
  /** A pack tab: no stage keys; ⌘Z undoes the session unless the item editor has the focus. */
  packTab: boolean;
  /** A theme tab: no save keys, no slide or element keys. */
  themeTab: boolean;
  cutMark: { slide: string; element: string } | null;
  keptUi: KeptUi;
  cursor: HistoryCursor;
  autosave: ReturnType<typeof useFileActions>["autosave"];
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  apply: (label: string, ops: Op[]) => Promise<boolean>;
  present: (mode: PresentMode) => void;
  goToSlide: (id: string) => void;
  moveSlide: (id: string, slot: number) => void;
  dropCut: () => void;
  open: () => void;
  saveCopy: () => void;
  cancelAgent: () => void;
  setEditing: (key: FieldKey | null) => void;
  setSelected: (id: string | null) => void;
}

/** Les raccourcis de l'onglet, sauf ceux qu'un composant a déjà pris. */
export function useAppKeys({
  api, snap, view, current, selected, editing, presenting, agentStatus, dialogOpen, packTab, themeTab,
  cutMark, keptUi, cursor, autosave, run, apply, present, goToSlide, moveSlide, dropCut, open,
  saveCopy, cancelAgent, setEditing, setSelected,
}: AppKeysContext) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Already taken by a component (the pile…): never two actions for one key.
      if (e.defaultPrevented) return;
      // La présentation prend ses touches (`Presenter`) ; derrière elle, rien ne réagit.
      if (presenting) return;
      // Dans une saisie de texte, ou (touches seules) une surface A2UI ou le dock, les touches sont les leurs ; ⌘Z passe depuis un bouton.
      // Seuls ⌘' et ⌘⇧G, qu'aucun éditeur ne lie, y passent (t-364). Une fenêtre de l'agent ou de l'app ouverte : rien derrière elle ne réagit.
      const scope = appKeyScope({ modal: agentStatus.modal, dialog: dialogOpen, editing: !!editing }, e);
      if (scope === "none") return;
      // L'aide : `HelpBar` a ses touches ; rien ne s'édite ni ne se bascule.
      if (snap?.read_only) return;
      if (scope === "display") {
        keptUi.handleKey(e);
        return;
      }
      if (themeTab && themeTabIgnores(e)) return;
      const mod = e.metaKey || e.ctrlKey;
      const presentMode = presentShortcut(e);
      if (presentMode && !snap?.editing && !themeTab && !snap?.history) {
        e.preventDefault();
        present(presentMode);
        return;
      }
      // Grille et fantômes se basculent partout, historique compris.
      if (keptUi.handleKey(e)) return;
      if (snap?.history) {
        // Lecture seule : ← / → (⇧ : par session) parcourent les versions ;
        // les raccourcis d'édition, qui prennent aussi les flèches, sont inertes.
        if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && !mod && !e.altKey) {
          e.preventDefault();
          cursor.arrow(e.key, e.shiftKey);
        }
        return;
      }
      // Slide précédente / suivante : ← → ↑ ↓ sans sélection (sinon elles déplacent l'élément), Page↑ Page↓, Début, Fin.
      if (snap && !packTab) {
        const at = snap.slides.findIndex((s) => s.id === current);
        const to = slideTarget(e, at, snap.slides.length, { space: false, arrows: selected === null });
        if (to !== null) {
          e.preventDefault();
          goToSlide(snap.slides[to].id);
          return;
        }
      }
      // In a pack tab, ⌘Z inside the item editor is CodeMirror's (its toolbar included); elsewhere, the session's.
      if (mod && e.key.toLowerCase() === "z") {
        if (packTab && (e.target as HTMLElement | null)?.closest?.(".typst-editor")) return;
        e.preventDefault();
        dropCut();
        run(e.shiftKey ? api.redo() : api.undo());
      } else if (mod && e.key.toLowerCase() === "s") {
        // Tout s'enregistre de soi-même : ⌘S n'avance que l'échéance, ⌘⇧S fait une copie.
        e.preventDefault();
        if (e.shiftKey && !snap?.editing) saveCopy();
        else void autosave.flush();
      } else if (mod && e.key.toLowerCase() === "o") {
        e.preventDefault();
        open();
      } else if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown") && snap && current && !packTab) {
        e.preventDefault();
        const i = snap.slides.findIndex((s) => s.id === current);
        const slot = e.key === "ArrowUp" ? i - 1 : i + 2;
        if (slot >= 0 && slot <= snap.slides.length) moveSlide(current, slot);
      } else if (e.key === "Escape" && cutMark) {
        // Échap abandonne d'abord la coupe ; l'appui suivant reprend son rôle.
        dropCut();
      } else if (e.key === "Escape") {
        const action = escapeAction({ selected: selected !== null, agentBusy: agentStatus.cancellable });
        if (action === "deselect") setSelected(null);
        else if (action === "agent-cancel") {
          e.preventDefault();
          cancelAgent();
        }
      } else if (!view || !selected || packTab) {
        // A pack tab shows no stage: no stage keys (Enter, Delete, arrows).
        return;
      } else if (e.key === "Enter") {
        const f = primaryFieldOf(view, selected);
        if (f) {
          e.preventDefault();
          setEditing(fieldKey(f));
        }
      } else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        apply(t("ui-undo-remove"), [{ op: "remove_element", slide: view.slide.id, element: selected }]);
        setSelected(null);
      } else if (e.key.startsWith("Arrow")) {
        const dx = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : 0;
        const dy = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
        // Sur la grille : une case à la fois. Un bloc fait passer la slide sur
        // la grille ; un élément libre s'y accroche s'il ne l'était pas.
        const place = gridPlacement(view, selected);
        if (place) {
          e.preventDefault();
          const { zone, grid } = place;
          const { cw, rh } = trackSize(zone, grid);
          const cell = moveCell(zone, grid, place.cell, dx * (cw + grid.gap), dy * (rh + grid.gap));
          apply(t(place.block ? "ui-undo-place-on-grid" : "ui-undo-move-on-grid"), placeOnGridOps(view, selected, cell));
          return;
        }
        const el = findElement(view.slide, selected);
        if (!el?.frame) return;
        e.preventDefault();
        const step = e.shiftKey ? 10 : 1;
        const f = { ...el.frame };
        if (e.key === "ArrowLeft") f.x -= step;
        if (e.key === "ArrowRight") f.x += step;
        if (e.key === "ArrowUp") f.y -= step;
        if (e.key === "ArrowDown") f.y += step;
        apply(t("ui-undo-move"), [{ op: "set_frame", slide: view.slide.id, element: selected, frame: f }]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
}
