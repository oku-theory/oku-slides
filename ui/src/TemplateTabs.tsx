/** Barre des sous-onglets du mode modèles : « Modèles », « Thème », puis un par widget ouvert. */
import { useRef, type KeyboardEvent } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";
import { THEME_TAB, type ActiveTab, type OpenWidget } from "./widgetTabs";
import { widgetLabel } from "./widgetList";

export interface TemplateTabsProps {
  tabs: OpenWidget[];
  active: ActiveTab;
  unsaved: ReadonlySet<string>;
  onActivate: (id: ActiveTab) => void;
  onClose: (id: number) => void;
}

/** Clé d'un onglet dans l'ordre clavier : 0 pour « Modèles », `THEME_TAB`, ou l'id du widget. */
type Key = number | typeof THEME_TAB;

export default function TemplateTabs({ tabs, active, unsaved, onActivate, onClose }: TemplateTabsProps) {
  const els = useRef(new Map<Key, HTMLButtonElement>());
  const order: Key[] = [0, THEME_TAB, ...tabs.map((tab) => tab.id)];
  const rtl = typeof document !== "undefined" && document.dir === "rtl";
  const go = (key: Key) => {
    els.current.get(key)?.focus();
    onActivate(key === 0 ? null : key);
  };
  const onKey = (e: KeyboardEvent, key: Key) => {
    const i = order.indexOf(key);
    const step = { ArrowRight: rtl ? -1 : 1, ArrowLeft: rtl ? 1 : -1 }[e.key];
    if (step !== undefined) go(order[(i + step + order.length) % order.length]);
    else if (e.key === "Home") go(order[0]);
    else if (e.key === "End") go(order[order.length - 1]);
    else if (e.key === "Delete" || e.key === "Backspace") {
      // Suppr, ou ⌫ (Mac). Sur « Modèles » et « Thème », rien, mais l'élément choisi de la scène ne doit pas être supprimé.
      if (typeof key === "number" && key !== 0) onClose(key);
    } else return;
    // Les raccourcis de la fenêtre (scène : flèches, Suppr) ne voient pas la touche.
    e.preventDefault();
    e.stopPropagation();
  };
  const tabProps = (key: Key, selected: boolean) => ({
    role: "tab" as const,
    id: `widget-tab-${key}`,
    "aria-selected": selected,
    tabIndex: selected ? 0 : -1,
    className: "template-tab",
    ref: (el: HTMLButtonElement | null) => {
      if (el) els.current.set(key, el);
      else els.current.delete(key);
    },
    onKeyDown: (e: KeyboardEvent) => onKey(e, key),
  });

  return (
    <div role="tablist" aria-label={t("ui-widgets-tabs")} className="template-tabs">
      <button {...tabProps(0, active === null)} aria-controls="templates-panel" onClick={() => onActivate(null)}>
        {t("ui-widgets-templates-tab")}
      </button>
      <button {...tabProps(THEME_TAB, active === THEME_TAB)} aria-controls="theme-panel" onClick={() => onActivate(THEME_TAB)}>
        {t("ui-theme-tab")}
      </button>
      {tabs.map((tab) => {
        const label = widgetLabel(tab.name);
        // L'enveloppe sort de l'arbre d'accessibilité ; le ×, caché aux
        // lecteurs d'écran et hors tabulation, laisse la liste aux seuls onglets
        // (au clavier : Suppr).
        return (
          <div key={tab.id} role="presentation" className="template-tab-slot">
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
