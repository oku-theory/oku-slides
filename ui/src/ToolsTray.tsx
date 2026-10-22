// La barre d'outils sous la feuille. On y ouvre la grille d'ajout, montre les paramètres,
// ouvre le menu, annule et rétablit ; sur un sous-onglet widget, elle porte ses propres outils.

import type { ReactNode } from "react";
import type { SlideView, Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconToggle } from "./primitives";
import { canAdd } from "./slideTools";
import type { KeptUi } from "./useKeptUi";
import type { PackWidgetEditor } from "./usePackWidgetEditor";
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
  /** Les sous-onglets widget du mode pack. */
  widgets: PackWidgetEditor;
  /** Les réglages d'interface gardés par l'hôte. */
  keptUi: KeptUi;
  /** Les paramètres de contenu sont masqués. */
  paramsHidden: boolean;
  /** Le menu principal, posé dans la barre. */
  mainMenu: ReactNode;
  /** Applique une requête qui rend un snapshot. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  /** Ouvre la grille d'ajout. */
  onAdd: () => void;
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
  mainMenu,
  run,
  onAdd,
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
      {/* Un seul bouton d'ajout : il ouvre la grille des éléments natifs et des widgets du pack.
          Actif dès qu'une carte autre que le texte s'ajoute (« widget » : toute sorte non textuelle). */}
      <button
        className="token ghost add-button"
        aria-label={t("ui-toolbar-add-label")}
        title={t("ui-toolbar-add-label")}
        aria-haspopup="dialog"
        hidden={widgetActive}
        disabled={!canAdd(view?.slide.kind, "widget")}
        onClick={onAdd}
      >
        {Icon.widget}
      </button>
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
          <IconToggle
            className="token ghost"
            pressed={!paramsHidden}
            label={paramsLabel}
            onToggle={() => keptUi.setFolded("params", !paramsHidden)}
            icon={Icon.panelBottom}
          />
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
