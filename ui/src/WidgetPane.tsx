/** Les sous-onglets ouverts, montés ; seul l'actif est visible. */
import type { TabApi } from "./api";
import type { TemplateWidgets } from "./useTemplateWidgets";
import WidgetTab from "./WidgetTab";

export default function WidgetPane({ api, widgets }: { api: TabApi; widgets: TemplateWidgets }) {
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
          active={tab.id === widgets.tabs.active}
          onOpen={widgets.open}
          onCommands={widgets.commandsOf(tab.id)}
          onHistory={widgets.historyOf(tab.id)}
        />
      ))}
    </div>
  );
}
