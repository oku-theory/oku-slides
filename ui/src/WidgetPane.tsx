/** Les sous-onglets ouverts, montés ; seul l'actif est visible. */
import type { ReactNode } from "react";
import type { Snapshot, TabApi } from "./api";
import type { TemplateWidgets } from "./useTemplateWidgets";
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
  /** Largeur du code choisie au séparateur, commune à tous les sous-onglets ; `null` : proportion par défaut. */
  codeWidth: number | null;
  onCodeWidth: (width: number | null) => void;
  /** Réglages de l'hôte du widget actif, posés sous son aperçu. */
  settings?: ReactNode;
  /** Écritures des valeurs d'exemple : refus en toast, snapshot mis à jour. */
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  /** Couleurs des pastilles dans l'éditeur d'un champ de l'aperçu (palette des graphiques du thème). */
  palette: string[];
}

export default function WidgetPane({ api, widgets, trials, look, code, codeWidth, onCodeWidth, settings = null, run, palette }: Props) {
  return (
    <div className="widget-pane" hidden={typeof widgets.tabs.active !== "number"}>
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
          codeWidth={codeWidth}
          onCodeWidth={onCodeWidth}
          settings={tab.id === widgets.tabs.active ? settings : null}
          run={run}
          palette={palette}
          active={tab.id === widgets.tabs.active}
          onOpen={widgets.open}
          onCommands={widgets.commandsOf(tab.id)}
          onHistory={widgets.historyOf(tab.id)}
        />
      ))}
    </div>
  );
}
