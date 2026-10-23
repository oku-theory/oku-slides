/**
 * Barre des sous-onglets du mode pack : « Layouts » (la scène, pack de layouts seulement),
 * « Thème », puis un par widget ouvert.
 */
import { useRef, type KeyboardEvent } from "react";
import type { PackKind } from "./packTypes";
import { t } from "./i18n";
import { Icon } from "./icons";
import { THEME_TAB, type ActiveTab, type OpenWidget } from "./widgetTabs";
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

/** Clé d'un onglet dans l'ordre clavier : `STAGE_TAB` (« Layouts »), `THEME_TAB`, ou l'id du widget. */
type Key = number | typeof THEME_TAB;

/** Clé de l'onglet « Layouts » : la scène (onglet actif `null`) ; les ids des widgets partent de 1. */
const STAGE_TAB = 0;

/**
 * Les sous-onglets du pack édité. Un pack de widgets n'a pas de scène à montrer : sans onglet
 * actif (aucun widget ouvert), « Thème » reste l'entrée de la barre au clavier.
 */
export default function PackTabs({ kind, tabs, active, unsaved, onActivate, onClose }: PackTabsProps) {
  const els = useRef(new Map<Key, HTMLButtonElement>());
  const stage = kind === "layouts";
  const order: Key[] = [...(stage ? [STAGE_TAB] : []), THEME_TAB, ...tabs.map((tab) => tab.id)];
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
      // Suppr, ou ⌫ (Mac). Sur « Layouts » et « Thème », rien, mais l'élément choisi de la scène ne doit pas être supprimé.
      if (typeof key === "number" && key !== STAGE_TAB) onClose(key);
    } else return;
    // Les raccourcis de la fenêtre (scène : flèches, Suppr) ne voient pas la touche.
    e.preventDefault();
    e.stopPropagation();
  };
  // Aucun onglet actif : « Layouts », sinon « Thème » prend la tabulation, la barre reste joignable.
  const tabbable = active ?? (stage ? STAGE_TAB : THEME_TAB);
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

  return (
    <div role="tablist" aria-label={t("ui-pack-tabs")} className="pack-tabs">
      {stage && (
        <button {...tabProps(STAGE_TAB, active === null)} aria-controls="stage-panel" onClick={() => onActivate(null)}>
          {t("ui-pack-layouts-tab")}
        </button>
      )}
      <button {...tabProps(THEME_TAB, active === THEME_TAB)} aria-controls="theme-panel" onClick={() => onActivate(THEME_TAB)}>
        {t("ui-theme-tab")}
      </button>
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
