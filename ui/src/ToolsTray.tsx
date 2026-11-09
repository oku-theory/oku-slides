// The toolbar under the sheet. It opens the add grid and the theme picker, shows or hides
// the thumbnails, opens the menu, undoes and redoes; in a pack tab, it carries the selected
// item's tools instead, the session's undo among them while no code editor shows.

import type { ReactNode } from "react";
import type { Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconButton, IconToggle } from "./primitives";
import { isDeckTab } from "./tabKind";
import type { KeptUi } from "./useKeptUi";
import type { PackItem } from "./usePackItem";
import WidgetTools from "./WidgetTools";
import { COMMON } from "./widgetList";

/** Ce que la barre d'outils montre et ce qu'elle déclenche. */
interface Props {
  /** Requêtes de l'onglet : annuler et rétablir. */
  api: TabApi;
  /** Le snapshot, pour l'état d'annulation. */
  snap: Snapshot;
  /** A pack tab's selected item: the bar shows its editor tools; `null` elsewhere. */
  item: PackItem | null;
  /** Les réglages d'interface gardés par l'hôte. */
  keptUi: KeptUi;
  /** Le menu principal, posé dans la barre. */
  mainMenu: ReactNode;
  /** Applique une requête qui rend un snapshot. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  /** Opens the add grid; `null` when the slide takes no element: the button is then disabled. */
  onAdd: (() => void) | null;
  /** Opens the theme picker on the slide shown (surface `host-theme-picker`). */
  onThemes: () => void;
  /** Opens the deck's history, as the agent's `history` tool does. */
  onHistory: () => void;
  /** La bande des miniatures sous la scène (montrée, et sa bascule) ; `null` : pas de bande (mode pack). */
  thumbs: { open: boolean; onToggle: () => void } | null;
  /** A theme tab: nothing to add, no deck packs. */
  themeTab: boolean;
}

/** Les outils posés sous la feuille. */
export function ToolsTray({
  api,
  snap,
  item,
  keptUi,
  mainMenu,
  run,
  onAdd,
  onThemes,
  onHistory,
  thumbs,
  themeTab,
}: Props) {
  const undoLabel = snap.undo_label
    ? t("ui-toolbar-undo-label", { label: snap.undo_label })
    : t("ui-toolbar-undo");
  const redoLabel = snap.redo_label
    ? t("ui-toolbar-redo-label", { label: snap.redo_label })
    : t("ui-toolbar-redo");
  // The themes and history buttons: in deck mode only, not in pack mode, help, history or a
  // theme tab. The deck's packs and the inspector are in the side panel (`SidePanel`).
  const deckMode = isDeckTab(snap);
  const sessionHistory = (
    <>
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
  );
  // Ce qu'on ajoute à la slide, le menu de ce qui sert rarement, puis
  // annuler / rétablir : posés sous la feuille.
  return (
    <div className="object theme-card tools-tray">
      {/* One add button: it opens the grid of the packs' widgets. Enabled when App gives an opener
          (the slide has a free slot); each card then says whether that slot takes it. */}
      <button
        className="token ghost add-button"
        aria-label={t("ui-toolbar-add-label")}
        title={t("ui-toolbar-add-label")}
        aria-haspopup="dialog"
        hidden={item !== null || themeTab}
        disabled={onAdd === null}
        onClick={onAdd ?? undefined}
      >
        {Icon.widget}
      </button>
      {deckMode && (
        <IconButton
          className="token ghost themes-button"
          label={t("ui-themes")}
          icon={Icon.palette}
          aria-haspopup="dialog"
          data-popover-anchor="host-theme-picker"
          onClick={onThemes}
        />
      )}
      <span className="tools-sep" aria-hidden hidden={item !== null} />
      {item ? (
        <>
          {mainMenu}
          <span className="tools-sep" aria-hidden />
          <WidgetTools
            editor={item.selected !== null && (keptUi.code || item.selected === COMMON)}
            history={item.history}
            onUndo={item.undo}
            onRedo={item.redo}
            session={sessionHistory}
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
          {deckMode && (
            <IconButton
              className="token ghost history-button"
              label={t("ui-toolbar-history")}
              icon={Icon.history}
              onClick={onHistory}
            />
          )}
          {sessionHistory}
        </>
      )}
    </div>
  );
}
