/** The toolbar of a pack tab's selected item: its editor's history and its code switch. */
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
  options: Pick<KeptUi, "code" | "setCode">;
}

/** Undo, redo and the code switch of the selected item's editor. */
export default function WidgetTools({ selected, history, onUndo, onRedo, options }: Props) {
  const { code, setCode } = options;
  return (
    <>
      <button className="token ghost" aria-label={t("ui-toolbar-undo")} title={t("ui-toolbar-undo")} disabled={history.undo === 0} onClick={onUndo}>{Icon.undo}</button>
      <button className="token ghost" aria-label={t("ui-toolbar-redo")} title={t("ui-toolbar-redo")} disabled={history.redo === 0} onClick={onRedo}>{Icon.redo}</button>
      <span className="tools-sep" aria-hidden />
      <CodeSwitch checked={code} controls={selected ? "widget-code" : undefined} onChange={setCode} />
    </>
  );
}
