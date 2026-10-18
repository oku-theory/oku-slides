// La barre d'outils sous la feuille. On y ajoute un élément ou un widget, montre les paramètres,
// ouvre le menu, annule et rétablit ; sur un sous-onglet widget, elle porte ses propres outils.

import type { ReactNode } from "react";
import { ActionMenu } from "./ActionMenu";
import type { AddWidget, SlideView, Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { AddKind } from "./slideTools";
import { ADD_KINDS, canAdd } from "./slideTools";
import type { KeptUi } from "./useKeptUi";
import type { TemplateWidgets } from "./useTemplateWidgets";
import { activeWidget } from "./widgetTabs";
import WidgetTools from "./WidgetTools";

/** Ce que la barre d'outils montre et ce qu'elle déclenche. */
interface Props {
  /** Requêtes de l'onglet : annuler et rétablir. */
  api: TabApi;
  /** Le snapshot, pour l'état d'annulation. */
  snap: Snapshot;
  /** La slide montrée, qui dit ce qu'on peut y ajouter. */
  view: SlideView | null;
  /** Un sous-onglet widget est actif : la barre montre ses outils. */
  widgetActive: boolean;
  /** Les sous-onglets widget du mode modèles. */
  widgets: TemplateWidgets;
  /** Les réglages d'interface gardés par l'hôte. */
  keptUi: KeptUi;
  /** Les paramètres de contenu sont masqués. */
  paramsHidden: boolean;
  /** Les widgets du pack qu'on peut ajouter. */
  addWidgets: AddWidget[];
  /** Le menu principal, posé dans la barre. */
  mainMenu: ReactNode;
  /** Applique une requête qui rend un snapshot. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  /** Ajoute un élément du genre donné. */
  addElement: (kind: AddKind) => void;
  /** Ajoute un widget du pack. */
  addPackWidget: (w: AddWidget) => void;
}

/** Les outils posés sous la feuille. */
export function ToolsTray({
  api,
  snap,
  view,
  widgetActive,
  widgets,
  keptUi,
  paramsHidden,
  addWidgets,
  mainMenu,
  run,
  addElement,
  addPackWidget,
}: Props) {
  const paramsLabel = t(paramsHidden ? "ui-toolbar-params-show" : "ui-toolbar-params-hide");
  const undoLabel = snap.undo_label
    ? t("ui-toolbar-undo-label", { label: snap.undo_label })
    : t("ui-toolbar-undo");
  const redoLabel = snap.redo_label
    ? t("ui-toolbar-redo-label", { label: snap.redo_label })
    : t("ui-toolbar-redo");
  // Ce qu'on ajoute à la slide, les paramètres et le menu de ce qui sert rarement, puis
  // annuler / rétablir : posés sous la feuille.
  return (
    <div className="object theme-card tools-tray">
      <div className="add-group" role="group" aria-label={t("ui-toolbar-add-label")} hidden={widgetActive}>
        {ADD_KINDS.map((k) => {
          const name = t("ui-add-kind", { kind: k });
          return (
            <button
              key={k}
              className="token ghost"
              aria-label={name}
              title={name}
              disabled={!canAdd(k, view?.slide.kind)}
              onClick={() => addElement(k)}
            >
              {Icon[k]}
            </button>
          );
        })}
        {addWidgets.length > 0 && (
          <ActionMenu
            label={t("ui-add-widget")}
            icon={Icon.widget}
            triggerClass="token ghost"
            disabled={!canAdd("widget", view?.slide.kind)}
            entries={addWidgets.map((w) => ({ kind: "item", label: w.label ?? w.name, onSelect: () => addPackWidget(w) }))}
          />
        )}
      </div>
      <span className="tools-sep" aria-hidden hidden={widgetActive} />
      {widgetActive ? (
        <>
          {mainMenu}
          <span className="tools-sep" aria-hidden />
          <WidgetTools
            tab={activeWidget(widgets.tabs)}
            history={widgets.history}
            onUndo={widgets.undo}
            onRedo={widgets.redo}
            options={keptUi}
          />
        </>
      ) : (
        <>
          {/* Les paramètres de contenu sous la slide : affichés ou masqués (gardé par l'hôte). */}
          <button
            className="token ghost"
            aria-pressed={!paramsHidden}
            aria-label={paramsLabel}
            title={paramsLabel}
            onClick={() => keptUi.setFolded("params", !paramsHidden)}
          >
            {Icon.panelBottom}
          </button>
          {mainMenu}
          <span className="tools-sep" aria-hidden />
          <button
            className="token ghost"
            aria-label={undoLabel}
            title={snap.undo_label ?? t("ui-toolbar-undo")}
            disabled={!snap.can_undo}
            onClick={() => run(api.undo())}
          >
            {Icon.undo}
          </button>
          <button
            className="token ghost"
            aria-label={redoLabel}
            title={snap.redo_label ?? t("ui-toolbar-redo")}
            disabled={!snap.can_redo}
            onClick={() => run(api.redo())}
          >
            {Icon.redo}
          </button>
        </>
      )}
    </div>
  );
}
