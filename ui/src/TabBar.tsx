// Barre d'onglets d'une fenêtre : cliquer, glisser, menu contextuel.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { createPortal } from "react-dom";
import { listenTauri, type TabAt, tabPath, type WindowView, type WorkspaceView } from "./api";
import { t } from "./i18n";
import { arrivalIndex, coalesce, cycle, finalIndex, menuAbove, outside, slotAt } from "./tabs";

interface Props {
  view: WorkspaceView;
  win: WindowView;
  /** Plusieurs fenêtres possibles (Tauri) : actions de fenêtre visibles. */
  windows: boolean;
  onActivate: (tab: string) => void;
  onClose: (tab: string) => void;
  onCloseOthers: (tab: string) => void;
  onNew: () => void;
  onReorder: (tab: string, index: number) => void;
  /** Vers `window` ; `null` : une nouvelle fenêtre. */
  onMove: (tab: string, window: string | null) => void;
  /** Hors de la fenêtre, l'onglet survole peut-être une autre fenêtre (l'hôte lit le curseur). */
  onDragOver: (tab: string) => Promise<unknown>;
  /** Revenu dans la fenêtre, ou glisser annulé : plus de survol ailleurs. */
  onDragLeave: () => Promise<unknown>;
  /** Lâché hors de la fenêtre, au curseur. */
  onDropOutside: (tab: string) => void;
}

/** Onglet glissé depuis cette barre ; `x`, `y` : le pointeur, `grab` : sa prise dans l'onglet. */
interface Drag {
  id: string;
  slot: number;
  x: number;
  y: number;
  grabX: number;
  grabY: number;
  /** Hors de la fenêtre : le fantôme est chez la fenêtre survolée, s'il y en a une. */
  out: boolean;
}

/** Barre d'onglets d'une fenêtre. */
export default function TabBar(props: Props) {
  const { view, win, windows, onActivate, onClose, onCloseOthers, onNew, onReorder, onMove, onDragOver, onDragLeave, onDropOutside } = props;
  const els = useRef(new Map<string, HTMLElement>());
  const winRef = useRef(win);
  winRef.current = win;
  const reorderRef = useRef(onReorder);
  reorderRef.current = onReorder;
  const [drag, setDrag] = useState<Drag | null>(null);
  // Onglet d'une autre fenêtre sous le pointeur (prévenu par l'hôte).
  const [incoming, setIncoming] = useState<TabAt | null>(null);
  // `placed` : position corrigée après mesure du menu rendu.
  const [menu, setMenu] = useState<{ id: string; x: number; y: number; placed?: boolean } | null>(null);
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

  // Le menu mesuré une fois affiché, avant d'être peint : ouvert vers le haut
  // (la barre est en bas) et gardé dans la fenêtre, plutôt qu'estimé à l'avance.
  useLayoutEffect(() => {
    if (!menu || menu.placed || !menuRef.current) return;
    const rect = menuRef.current.getBoundingClientRect();
    const at = menuAbove(menu.x, menu.y, rect.width, rect.height, window.innerWidth, window.innerHeight);
    setMenu({ id: menu.id, ...at, placed: true });
  }, [menu]);

  /** Milieu de chaque onglet (sauf `except`), à l'instant (les onglets ont pu changer). */
  const mids = (except?: string) =>
    winRef.current.tabs.filter((tab) => tab.id !== except).map((tab) => {
      const r = els.current.get(tab.id)?.getBoundingClientRect();
      return r ? r.left + r.width / 2 : 0;
    });

  // Un onglet d'une autre fenêtre : l'hôte prévient au survol (fantôme, place
  // de dépôt), puis, l'onglet déjà arrivé en fin de barre, au lâcher : sa
  // place se calcule ici, d'après nos autres onglets.
  useEffect(() => {
    if (!windows) return;
    return listenTauri(async () => {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const w = getCurrentWebviewWindow();
      const stops = await Promise.all([
        w.listen<TabAt | null>("tab-drag", (e) => setIncoming(e.payload)),
        w.listen<TabAt>("tab-drop", (e) => {
          setIncoming(null);
          const index = arrivalIndex(mids(e.payload.tab), e.payload.x, rtl);
          if (index !== null) reorderRef.current(e.payload.tab, index);
        }),
      ]);
      return () => stops.forEach((stop) => stop());
    });
  }, [windows]);

  // Pointeur plutôt que le glisser-déposer HTML5 : un lâcher hors fenêtre doit
  // ouvrir une fenêtre au point de l'écran via l'hôte, ce que DnD ne permet pas.
  const onPointerDown = (e: ReactPointerEvent<HTMLElement>, id: string) => {
    if (e.button !== 0) return;
    if (id !== winRef.current.active) onActivate(id);
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const [startX, startY] = [e.clientX, e.clientY];
    const box = el.getBoundingClientRect();
    const [grabX, grabY] = [startX - box.left, startY - box.top];
    let moved = false;
    let out = false;
    // Le survol suit le pointeur sans empiler les requêtes ; le lâcher attend le dernier.
    const over = coalesce(() => onDragOver(id));
    const move = (ev: PointerEvent) => {
      // Dans les deux axes : tirer un onglet vers le bas le détache aussi.
      if (!moved && Math.hypot(ev.clientX - startX, ev.clientY - startY) < 4) return;
      moved = true;
      const nowOut = windows && outside(ev.clientX, ev.clientY, window.innerWidth, window.innerHeight);
      if (nowOut) over.run();
      else if (out) void over.idle().then(onDragLeave);
      out = nowOut;
      const slot = nowOut ? -1 : slotAt(mids(), ev.clientX, rtl);
      setDrag({ id, slot, x: ev.clientX, y: ev.clientY, grabX, grabY, out: nowOut });
    };
    const end = (ev: PointerEvent) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      setDrag(null);
      if (!moved) return;
      if (ev.type === "pointercancel") {
        if (out) void over.idle().then(onDragLeave);
        return;
      }
      if (windows && outside(ev.clientX, ev.clientY, window.innerWidth, window.innerHeight)) {
        void over.idle().then(() => onDropOutside(id));
        return;
      }
      if (out) void over.idle().then(onDragLeave);
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

  // Place de dépôt montrée : celle de notre glisser, ou celle d'un onglet venu d'ailleurs.
  const slot = drag ? drag.slot : incoming ? slotAt(mids(), incoming.x, rtl) : -1;
  // Fantôme : notre onglet tant qu'il est chez nous, ou celui d'une autre fenêtre.
  const title = (id: string) => view.windows.flatMap((w) => w.tabs).find((tab) => tab.id === id)?.title ?? "";
  const ghost =
    drag && !drag.out
      ? { title: title(drag.id), left: drag.x - drag.grabX, top: drag.y - drag.grabY }
      : incoming
        ? { title: title(incoming.tab), left: incoming.x - 24, top: incoming.y - 15 }
        : null;

  return (
    <div className={`tabbar${slot === win.tabs.length ? " drop-end" : ""}${incoming ? " drop-target" : ""}`}>
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
            className={`tab${tab.id === win.active ? " active" : ""}${drag?.id === tab.id ? " dragging" : ""}${slot === i ? " drop-before" : ""}`}
            title={tabPath(tab) ?? tab.title}
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
              // Position du pointeur : placée après mesure du menu rendu (voir l'effet ci-dessus).
              setMenu({ id: tab.id, x: e.clientX, y: e.clientY });
            }}
          >
            {/* Pack or theme, until lot 6's borders; a theme tab never edits a pack. */}
            {(tab.editing_pack || tab.document?.kind === "theme") && (
              <span className="tab-badge">{t(tab.editing_pack ? "ui-tab-pack" : "ui-tab-theme")}</span>
            )}
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
      {/* Fantôme et menu dans `body` : la vitre de la barre (`backdrop-filter`)
          deviendrait leur bloc conteneur, et leur `position: fixed` y serait relative. */}
      {ghost &&
        createPortal(
          <div className="tab active tab-ghost" aria-hidden style={{ left: ghost.left, top: ghost.top }}>
            <span className="tab-title">{ghost.title}</span>
          </div>,
          document.body,
        )}
      {menu &&
        createPortal(
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
          </div>,
          document.body,
        )}
    </div>
  );
}
