/** Les sous-onglets ouverts, montés ; seul l'actif est visible. */
import type { ReactNode } from "react";
import type { TabApi } from "./api";
import type { TemplateWidgets } from "./useTemplateWidgets";
import type { CodeLayout } from "./useViewOptions";
import WidgetTab from "./WidgetTab";

interface Props {
  api: TabApi;
  widgets: TemplateWidgets;
  /** Révision des essais (commune au pack) : l'aperçu se relance quand elle change. */
  trials: number;
  /** Empreinte du style du deck : les aperçus se relancent quand elle change. */
  look: string;
  /** Code source affiché, commun à tous les sous-onglets. */
  code: boolean;
  /** Place du code et tailles choisies, communes à tous les sous-onglets. */
  layout: CodeLayout;
  onLayout: (layout: CodeLayout) => void;
  /** Réglages de l'hôte du widget actif, posés sous son aperçu. */
  settings?: ReactNode;
}

export default function WidgetPane({ api, widgets, trials, look, code, layout, onLayout, settings = null }: Props) {
  return (
    <div className="widget-pane" hidden={widgets.tabs.active === null}>
      {widgets.tabs.tabs.map((tab) => (
        <WidgetTab
          key={tab.id}
          api={api}
          files={widgets.files}
          id={tab.id}
          name={tab.name}
          revision={widgets.revision}
          trials={trials}
          look={look}
          code={code}
          layout={layout}
          onLayout={onLayout}
          settings={tab.id === widgets.tabs.active ? settings : null}
          active={tab.id === widgets.tabs.active}
          onOpen={widgets.open}
          onCommands={widgets.commandsOf(tab.id)}
          onHistory={widgets.historyOf(tab.id)}
        />
      ))}
    </div>
  );
}
