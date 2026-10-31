// The toolbar under the sheet. It opens the add grid and the deck's packs panel, shows or hides
// the thumbnails, opens the menu, undoes and redoes; in a pack tab, it carries the selected
// item's tools instead, the session's undo among them while no code editor shows.

import type { ReactNode } from "react";
import type { SlideView, Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconButton, IconToggle } from "./primitives";
import { acceptsElements } from "./slideTools";
import type { KeptUi } from "./useKeptUi";
import type { PackItem } from "./usePackItem";
import WidgetTools from "./WidgetTools";

/** Ce que la barre d'outils montre et ce qu'elle déclenche. */
interface Props {
  /** Requêtes de l'onglet : annuler et rétablir. */
  api: TabApi;
  /** Le snapshot, pour l'état d'annulation. */
  snap: Snapshot;
  /** La slide montrée, qui dit ce qu'on peut y ajouter. */
  view: SlideView | null;
  /** A pack tab's selected item: the bar shows its editor tools; `null` elsewhere. */
  item: PackItem | null;
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
  view,
  item,
  keptUi,
  mainMenu,
  run,
  onAdd,
  onPacks,
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
  // The packs and history buttons: in deck mode only, not in pack mode, help, history or a
  // theme tab. A dot on packs says a pin is older than my current version.
  const deckMode = !snap.editing && !themeTab && !snap.read_only && !snap.history;
  const packsUpdate = snap.pack_updates > 0;
  // Avec une mise à jour, le nom accessible et l'infobulle la disent aussi (une seule phrase Fluent).
  const packsLabel = t(packsUpdate ? "ui-toolbar-packs-update" : "ui-toolbar-packs");
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
      {/* Un seul bouton d'ajout : il ouvre la grille des widgets des packs. Actif dès que la slide
          a un slot libre ; chaque carte dit ensuite si ce slot l'accepte. */}
      <button
        className="token ghost add-button"
        aria-label={t("ui-toolbar-add-label")}
        title={t("ui-toolbar-add-label")}
        aria-haspopup="dialog"
        hidden={item !== null || themeTab}
        disabled={!acceptsElements(view?.slide.kind, snap.free_slots)}
        onClick={onAdd}
      >
        {Icon.widget}
      </button>
      {deckMode && (
        <button
          className={`token ghost packs-button${packsUpdate ? " has-update" : ""}`}
          aria-label={packsLabel}
          title={packsLabel}
          onClick={onPacks}
        >
          {Icon.packs}
        </button>
      )}
      <span className="tools-sep" aria-hidden hidden={item !== null} />
      {item ? (
        <>
          {mainMenu}
          <span className="tools-sep" aria-hidden />
          <WidgetTools
            selected={item.selected !== null}
            history={item.history}
            onUndo={item.undo}
            onRedo={item.redo}
            session={sessionHistory}
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
