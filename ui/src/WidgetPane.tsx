/** Les sous-onglets ouverts, montés ; seul l'actif est visible. */
import type { TabApi } from "./api";
import type { TemplateWidgets } from "./useTemplateWidgets";
import WidgetTab from "./WidgetTab";

interface Props {
  api: TabApi;
  widgets: TemplateWidgets;
  /** Révision des essais (commune au pack) : l'aperçu se relance quand elle change. */
  trials: number;
  /** Code source affiché, commun à tous les sous-onglets. */
  code: boolean;
}

export default function WidgetPane({ api, widgets, trials, code }: Props) {
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
          code={code}
          active={tab.id === widgets.tabs.active}
          onOpen={widgets.open}
          onCommands={widgets.commandsOf(tab.id)}
          onHistory={widgets.historyOf(tab.id)}
        />
      ))}
    </div>
  );
}
