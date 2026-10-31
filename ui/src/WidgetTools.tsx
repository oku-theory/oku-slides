/**
 * The toolbar of a pack tab's selected item: an undo and redo, and the code switch.
 *
 * Undo and redo drive the item's editor while its code is shown, else the session's history
 * (pile deletes, renames, title, example values, agent writes), as ⌘Z outside the editor does.
 */
import type { ReactNode } from "react";
import CodeSwitch from "./CodeSwitch";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { EditorHistory } from "./TypstEditor";
import type { KeptUi } from "./useKeptUi";

/** The selected item's editor state and the shared code switch. */
interface Props {
  /** An item is selected: the switch controls its editor (`widget-code`). */
  selected: boolean;
  history: EditorHistory;
  onUndo: () => void;
  onRedo: () => void;
  /** The session's undo and redo buttons, shown while no editor is. */
  session: ReactNode;
  options: Pick<KeptUi, "code" | "setCode">;
}

/** Undo, redo (the editor's or the session's) and the code switch. */
export default function WidgetTools({ selected, history, onUndo, onRedo, session, options }: Props) {
  const { code, setCode } = options;
  return (
    <>
      {selected && code ? (
        <>
          <button className="token ghost" aria-label={t("ui-toolbar-undo")} title={t("ui-toolbar-undo")} disabled={history.undo === 0} onClick={onUndo}>{Icon.undo}</button>
          <button className="token ghost" aria-label={t("ui-toolbar-redo")} title={t("ui-toolbar-redo")} disabled={history.redo === 0} onClick={onRedo}>{Icon.redo}</button>
        </>
      ) : (
        session
      )}
      <span className="tools-sep" aria-hidden />
      <CodeSwitch checked={code} controls={selected ? "widget-code" : undefined} onChange={setCode} />
    </>
  );
}
