// La barre d'outils sous la feuille. On y ouvre la grille d'ajout et le panneau des packs du deck,
// montre ou cache les miniatures, ouvre le menu, annule et rétablit ; sur un sous-onglet widget,
// elle porte ses propres outils.

import type { ReactNode } from "react";
import type { SlideView, Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconToggle } from "./primitives";
import { acceptsElements } from "./slideTools";
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
  /** Le menu principal, posé dans la barre. */
  mainMenu: ReactNode;
  /** Applique une requête qui rend un snapshot. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  /** Ouvre la grille d'ajout. */
  onAdd: () => void;
  /** Ouvre le panneau des packs du deck (surface `host-packs` de l'agent). */
  onPacks: () => void;
  /** La bande des miniatures sous la scène (montrée, et sa bascule) ; `null` : pas de bande (mode pack). */
  thumbs: { open: boolean; onToggle: () => void } | null;
  /** A theme tab: nothing to add, no deck packs. */
  themeTab: boolean;
}

/** Les outils posés sous la feuille. */
export function ToolsTray({
  api,
  snap,
  view,
  widgetActive,
  widgets,
  keptUi,
  mainMenu,
  run,
  onAdd,
  onPacks,
  thumbs,
  themeTab,
}: Props) {
  const undoLabel = snap.undo_label
    ? t("ui-toolbar-undo-label", { label: snap.undo_label })
    : t("ui-toolbar-undo");
  const redoLabel = snap.redo_label
    ? t("ui-toolbar-redo-label", { label: snap.redo_label })
    : t("ui-toolbar-redo");
  // The packs button: in deck mode only, neither in help, history nor a theme tab. A dot
  // says a pin is older than my current version.
  const packsShown = !snap.editing && !themeTab && !snap.read_only && !snap.history;
  const packsUpdate = snap.pack_updates > 0;
  // Avec une mise à jour, le nom accessible et l'infobulle la disent aussi (une seule phrase Fluent).
  const packsLabel = t(packsUpdate ? "ui-toolbar-packs-update" : "ui-toolbar-packs");
  // Ce qu'on ajoute à la slide, le menu de ce qui sert rarement, puis
  // annuler / rétablir : posés sous la feuille.
  return (
    <div className="object theme-card tools-tray">
      {/* Un seul bouton d'ajout : il ouvre la grille des widgets des packs. Actif dès que la slide
          a un slot libre ; chaque carte dit ensuite si ce slot l'accepte. */}
      <button
        className="token ghost add-button"
        aria-label={t("ui-toolbar-add-label")}
        title={t("ui-toolbar-add-label")}
        aria-haspopup="dialog"
        hidden={widgetActive || themeTab}
        disabled={!acceptsElements(view?.slide.kind, snap.free_slots)}
        onClick={onAdd}
      >
        {Icon.widget}
      </button>
      {packsShown && (
        <button
          className={`token ghost packs-button${packsUpdate ? " has-update" : ""}`}
          aria-label={packsLabel}
          title={packsLabel}
          onClick={onPacks}
        >
          {Icon.packs}
        </button>
      )}
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
          {/* La bande des miniatures sous la slide : montrée ou cachée (gardé par l'hôte) ; cachée, elle rend toute sa place. */}
          {thumbs && (
            <IconToggle
              className="token ghost thumbs-toggle"
              pressed={thumbs.open}
              label={t(thumbs.open ? "ui-toolbar-thumbs-hide" : "ui-toolbar-thumbs-show")}
              onToggle={thumbs.onToggle}
              icon={Icon.strip}
            />
          )}
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
