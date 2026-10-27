/**
 * The pack mode's sub-tab bar: "Layouts" (the stage, layout packs only), then one per open
 * widget. A theme is edited in its own tab, never here.
 */
import { useRef, type KeyboardEvent } from "react";
import type { PackKind } from "./packTypes";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { ActiveTab, OpenWidget } from "./widgetTabs";
import { widgetLabel } from "./widgetList";

/** Les sous-onglets ouverts, l'actif et leurs gestes. */
export interface PackTabsProps {
  /** Sorte du pack édité : un pack de layouts a l'onglet de sa scène, un pack de widgets non. */
  kind: PackKind;
  tabs: OpenWidget[];
  active: ActiveTab;
  unsaved: ReadonlySet<string>;
  onActivate: (id: ActiveTab) => void;
  onClose: (id: number) => void;
}

/** A tab's key in keyboard order: `STAGE_TAB` ("Layouts") or the widget's id. */
type Key = number;

/** Clé de l'onglet « Layouts » : la scène (onglet actif `null`) ; les ids des widgets partent de 1. */
const STAGE_TAB = 0;

/**
 * The edited pack's sub-tabs. A widget pack has no stage to show: without an active tab its
 * first widget sub-tab is the bar's keyboard entry, and without any widget open there is no bar.
 */
export default function PackTabs({ kind, tabs, active, unsaved, onActivate, onClose }: PackTabsProps) {
  const els = useRef(new Map<Key, HTMLButtonElement>());
  const stage = kind === "layouts";
  const order: Key[] = [...(stage ? [STAGE_TAB] : []), ...tabs.map((tab) => tab.id)];
  const rtl = typeof document !== "undefined" && document.dir === "rtl";
  const go = (key: Key) => {
    els.current.get(key)?.focus();
    onActivate(key === STAGE_TAB ? null : key);
  };
  const onKey = (e: KeyboardEvent, key: Key) => {
    const i = order.indexOf(key);
    const step = { ArrowRight: rtl ? -1 : 1, ArrowLeft: rtl ? 1 : -1 }[e.key];
    if (step !== undefined) go(order[(i + step + order.length) % order.length]);
    else if (e.key === "Home") go(order[0]);
    else if (e.key === "End") go(order[order.length - 1]);
    else if (e.key === "Delete" || e.key === "Backspace") {
      // Delete, or ⌫ (Mac). On "Layouts", nothing, but the stage's selected element must not be deleted.
      if (key !== STAGE_TAB) onClose(key);
    } else return;
    // Les raccourcis de la fenêtre (scène : flèches, Suppr) ne voient pas la touche.
    e.preventDefault();
    e.stopPropagation();
  };
  // No active tab: "Layouts", else the first widget takes the tab key, so the bar stays reachable.
  const tabbable = active ?? order[0];
  const tabProps = (key: Key, selected: boolean) => ({
    role: "tab" as const,
    id: `widget-tab-${key}`,
    "aria-selected": selected,
    tabIndex: key === tabbable ? 0 : -1,
    className: "pack-tab",
    ref: (el: HTMLButtonElement | null) => {
      if (el) els.current.set(key, el);
      else els.current.delete(key);
    },
    onKeyDown: (e: KeyboardEvent) => onKey(e, key),
  });

  if (order.length === 0) return null;
  return (
    <div role="tablist" aria-label={t("ui-pack-tabs")} className="pack-tabs">
      {stage && (
        <button {...tabProps(STAGE_TAB, active === null)} aria-controls="stage-panel" onClick={() => onActivate(null)}>
          {t("ui-pack-layouts-tab")}
        </button>
      )}
      {tabs.map((tab) => {
        const label = widgetLabel(tab.name);
        // L'enveloppe sort de l'arbre d'accessibilité ; le ×, caché aux
        // lecteurs d'écran et hors tabulation, laisse la liste aux seuls onglets
        // (au clavier : Suppr).
        return (
          <div key={tab.id} role="presentation" className="pack-tab-slot">
            <button
              {...tabProps(tab.id, active === tab.id)}
              aria-controls={`widget-panel-${tab.id}`}
              onClick={() => onActivate(tab.id)}
              onAuxClick={(e) => {
                if (e.button !== 1) return;
                e.preventDefault();
                onClose(tab.id);
              }}
            >
              {label}
              {unsaved.has(tab.name) && <span className="unsaved-dot" role="img" aria-label={t("ui-widgets-unsaved")} />}
            </button>
            <button
              className="token ghost"
              tabIndex={-1}
              aria-hidden
              title={t("ui-widgets-close-tab", { name: label })}
              onClick={() => onClose(tab.id)}
            >
              {Icon.close}
            </button>
          </div>
        );
      })}
    </div>
  );
}
