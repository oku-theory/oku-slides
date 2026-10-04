/** Les sous-onglets ouverts, montés ; seul l'actif est visible. */
import type { ReactNode } from "react";
import type { Snapshot, TabApi } from "./api";
import { EXAMPLE_DEFAULT } from "./ExamplePills";
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
  /** Noms des cas d'exemple de chaque widget (`Snapshot.examples`). */
  examples: Record<string, string[]>;
  /** Cas courant gardé de chaque widget ; absent ou inconnu : le premier. */
  cases: Record<string, string>;
  onCase: (widget: string, name: string) => void;
  /** Planche (tous les cas) plutôt que scène, commune à tous les sous-onglets. */
  board: boolean;
  onBoard: (board: boolean) => void;
  /** Schéma du deck : celui de la planche au départ. */
  scheme?: "light" | "dark";
  /** Écritures des cas : refus en toast, snapshot mis à jour. */
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  /** Couleurs des pastilles dans l'éditeur d'un champ de l'aperçu (palette des graphiques du thème). */
  palette: string[];
}

export default function WidgetPane({ api, widgets, trials, look, code, layout, onLayout, settings = null, examples, cases, onCase, board, onBoard, scheme, run, palette }: Props) {
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
          // Un widget tout juste créé n'est pas encore au snapshot : son cas implicite.
          examples={examples[tab.name] ?? [EXAMPLE_DEFAULT]}
          example={cases[tab.name] ?? null}
          onExample={(name) => onCase(tab.name, name)}
          board={board}
          onBoard={onBoard}
          scheme={scheme}
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
