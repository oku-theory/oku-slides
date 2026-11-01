/**
 * The toolbar of a pack tab's selected item: an undo and redo.
 *
 * They drive the item's editor while its code shows, else the session's history
 * (pile deletes, renames, title, example values, agent writes), as ⌘Z outside the editor does.
 */
import type { ReactNode } from "react";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { EditorHistory } from "./TypstEditor";

/** The selected item's editor state. */
interface Props {
  /** The selected item's code shows (its Code tab, or Commun). */
  editor: boolean;
  history: EditorHistory;
  onUndo: () => void;
  onRedo: () => void;
  /** The session's undo and redo buttons, shown while no editor is. */
  session: ReactNode;
}

/** Undo and redo: the editor's or the session's. */
export default function WidgetTools({ editor, history, onUndo, onRedo, session }: Props) {
  return (
    <>
      {editor ? (
        <>
          <button className="token ghost" aria-label={t("ui-toolbar-undo")} title={t("ui-toolbar-undo")} disabled={history.undo === 0} onClick={onUndo}>{Icon.undo}</button>
          <button className="token ghost" aria-label={t("ui-toolbar-redo")} title={t("ui-toolbar-redo")} disabled={history.redo === 0} onClick={onRedo}>{Icon.redo}</button>
        </>
      ) : (
        session
      )}
    </>
  );
}
