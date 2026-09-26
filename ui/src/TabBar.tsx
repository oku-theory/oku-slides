// Barre d'onglets d'une fenêtre : cliquer, glisser, menu contextuel.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import type { WindowView, WorkspaceView } from "./api";
import { t } from "./i18n";
import { cycle, finalIndex, outside, slotAt } from "./tabs";

interface Props {
  view: WorkspaceView;
  win: WindowView;
  /** Barre de titre intégrée (macOS) : place pour les boutons de fenêtre. */
  overlay: boolean;
  /** Plusieurs fenêtres possibles (Tauri) : actions de fenêtre visibles. */
  windows: boolean;
  onActivate: (tab: string) => void;
  onClose: (tab: string) => void;
  onCloseOthers: (tab: string) => void;
  onNew: () => void;
  onReorder: (tab: string, index: number) => void;
  /** Vers `window` ; `null` : une nouvelle fenêtre. */
  onMove: (tab: string, window: string | null) => void;
  /** Lâché hors de la fenêtre, au point écran (`screenX`, `screenY`). */
  onDropOutside: (tab: string, x: number, y: number) => void;
}

/** Barre d'onglets d'une fenêtre. */
export default function TabBar({ view, win, overlay, windows, onActivate, onClose, onCloseOthers, onNew, onReorder, onMove, onDropOutside }: Props) {
  const els = useRef(new Map<string, HTMLElement>());
  const winRef = useRef(win);
  winRef.current = win;
  const [drag, setDrag] = useState<{ id: string; slot: number } | null>(null);
  const [menu, setMenu] = useState<{ id: string; x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Repère de droite à gauche (arabe) : la place de dépôt s'y compte à l'envers.
  const rtl = typeof document !== "undefined" && document.dir === "rtl";

  // Le menu se ferme au premier clic ailleurs, ou avec Échap.
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu]);

  // Trop d'onglets pour la largeur : l'onglet actif reste visible, même hors champ.
  useEffect(() => {
    els.current.get(win.active)?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [win.active]);

  // Le menu mesuré une fois affiché : clampé dans la fenêtre plutôt qu'estimé à l'avance.
  useLayoutEffect(() => {
    if (!menu || !menuRef.current) return;
    const rect = menuRef.current.getBoundingClientRect();
    const x = Math.min(menu.x, window.innerWidth - rect.width - 8);
    const y = Math.min(menu.y, window.innerHeight - rect.height - 8);
    if (x !== menu.x || y !== menu.y) setMenu({ id: menu.id, x, y });
  }, [menu?.id]);

  /** Milieu de chaque onglet, à l'instant (les onglets ont pu changer). */
  const mids = () =>
    winRef.current.tabs.map((tab) => {
      const r = els.current.get(tab.id)?.getBoundingClientRect();
      return r ? r.left + r.width / 2 : 0;
    });

  // Pointeur plutôt que le glisser-déposer HTML5 : un lâcher hors fenêtre doit
  // ouvrir une fenêtre au point de l'écran via l'hôte, ce que DnD ne permet pas.
  const onPointerDown = (e: ReactPointerEvent<HTMLElement>, id: string) => {
    if (e.button !== 0) return;
    if (id !== winRef.current.active) onActivate(id);
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const [startX, startY] = [e.clientX, e.clientY];
    let moved = false;
    let slot = -1;
    const move = (ev: PointerEvent) => {
      // Dans les deux axes : tirer un onglet vers le bas le détache aussi.
      if (!moved && Math.hypot(ev.clientX - startX, ev.clientY - startY) < 4) return;
      moved = true;
      const next = slotAt(mids(), ev.clientX, rtl);
      if (next !== slot) {
        slot = next;
        setDrag({ id, slot });
      }
    };
    const end = (ev: PointerEvent) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      setDrag(null);
      if (!moved || ev.type === "pointercancel") return;
      if (outside(ev.clientX, ev.clientY, window.innerWidth, window.innerHeight)) {
        if (windows) onDropOutside(id, ev.screenX, ev.screenY);
        return;
      }
      const from = winRef.current.tabs.findIndex((tab) => tab.id === id);
      const to = finalIndex(from, slotAt(mids(), ev.clientX, rtl));
      if (from >= 0 && to !== from) onReorder(id, to);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  };

  const pick = (action: () => void) => () => {
    action();
    setMenu(null);
  };
  // Dans le navigateur, le dernier onglet reste : sans lui, plus de fenêtre.
  const lastStays = !windows && win.tabs.length === 1;

  return (
    <div className={`tabbar${overlay ? " tabbar-overlay" : ""}${drag?.slot === win.tabs.length ? " drop-end" : ""}`} data-tauri-drag-region>
      <div className="tabs" role="tablist" aria-label={t("ui-tabs")}>
        {win.tabs.map((tab, i) => (
          <div
            key={tab.id}
            ref={(el) => {
              if (el) els.current.set(tab.id, el);
              else els.current.delete(tab.id);
            }}
            role="tab"
            tabIndex={tab.id === win.active ? 0 : -1}
            aria-selected={tab.id === win.active}
            className={`tab${tab.id === win.active ? " active" : ""}${drag?.id === tab.id ? " dragging" : ""}${drag?.slot === i ? " drop-before" : ""}`}
            title={tab.path ?? tab.title}
            onPointerDown={(e) => onPointerDown(e, tab.id)}
            onKeyDown={(e) => {
              // Motif WAI-ARIA des onglets : Entrée / Espace activent, les flèches déplacent.
              const ids = win.tabs.map((x) => x.id);
              const next =
                e.key === "ArrowRight" ? cycle(ids, tab.id, 1) : e.key === "ArrowLeft" ? cycle(ids, tab.id, -1) : undefined;
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onActivate(tab.id);
              } else if (next) {
                e.preventDefault();
                onActivate(next);
                els.current.get(next)?.focus();
              }
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              // Position brute : clampée après mesure du menu rendu (voir l'effet ci-dessus).
              setMenu({ id: tab.id, x: e.clientX, y: e.clientY });
            }}
          >
            {tab.editing_templates && <span className="tab-badge">{t("ui-tab-templates")}</span>}
            <span className="tab-title">{tab.title}</span>
            {tab.dirty && <span className="tab-dirty" role="img" aria-label={t("ui-tab-unsaved")} />}
            <button
              className="tab-close"
              aria-label={t("ui-tab-close")}
              title={t("ui-tab-close")}
              disabled={lastStays}
              onPointerDown={(e) => e.stopPropagation()}
              onClick={() => onClose(tab.id)}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <button className="tab-new" aria-label={t("ui-tab-new")} title={t("ui-tab-new")} onClick={onNew}>
        +
      </button>
      {menu && (
        <div
          ref={menuRef}
          className="tab-menu object"
          role="menu"
          style={{ left: menu.x, top: menu.y }}
          onPointerDown={(e) => e.stopPropagation()}
        >
          {windows && <button role="menuitem" onClick={pick(() => onMove(menu.id, null))}>{t("ui-tab-move-new-window")}</button>}
          {windows &&
            view.windows.map((w, i) =>
              w.label === win.label ? null : (
                <button key={w.label} role="menuitem" onClick={pick(() => onMove(menu.id, w.label))}>
                  {t("ui-tab-move-to", { n: i + 1 })}
                </button>
              ),
            )}
          <button role="menuitem" disabled={win.tabs.length < 2} onClick={pick(() => onCloseOthers(menu.id))}>{t("ui-tab-close-others")}</button>
          <button role="menuitem" disabled={lastStays} onClick={pick(() => onClose(menu.id))}>{t("ui-tab-close")}</button>
        </div>
      )}
    </div>
  );
}
