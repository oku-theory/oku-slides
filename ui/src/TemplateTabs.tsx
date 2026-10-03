/** Barre des sous-onglets du mode modèles : « Modèles », puis un par widget ouvert. */
import { useRef, type KeyboardEvent } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { OpenWidget } from "./widgetTabs";
import { widgetLabel } from "./widgetList";

export interface TemplateTabsProps {
  tabs: OpenWidget[];
  active: number | null;
  unsaved: ReadonlySet<string>;
  onActivate: (id: number | null) => void;
  onClose: (id: number) => void;
}

export default function TemplateTabs({ tabs, active, unsaved, onActivate, onClose }: TemplateTabsProps) {
  const els = useRef(new Map<number, HTMLButtonElement>());
  /** 0 : « Modèles » ; sinon l'id du sous-onglet. */
  const order = [0, ...tabs.map((tab) => tab.id)];
  const rtl = typeof document !== "undefined" && document.dir === "rtl";
  const go = (key: number) => {
    els.current.get(key)?.focus();
    onActivate(key === 0 ? null : key);
  };
  const onKey = (e: KeyboardEvent, key: number) => {
    const i = order.indexOf(key);
    const step = { ArrowRight: rtl ? -1 : 1, ArrowLeft: rtl ? 1 : -1 }[e.key];
    if (step !== undefined) go(order[(i + step + order.length) % order.length]);
    else if (e.key === "Home") go(order[0]);
    else if (e.key === "End") go(order[order.length - 1]);
    else if (e.key === "Delete" || e.key === "Backspace") {
      // Suppr, ou ⌫ (Mac). Sur « Modèles », rien, mais l'élément choisi de la scène ne doit pas être supprimé.
      if (key !== 0) onClose(key);
    } else return;
    // Les raccourcis de la fenêtre (scène : flèches, Suppr) ne voient pas la touche.
    e.preventDefault();
    e.stopPropagation();
  };
  const tabProps = (key: number, selected: boolean) => ({
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
