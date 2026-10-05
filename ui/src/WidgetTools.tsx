/** Barre d'outils d'un sous-onglet de widget : l'historique de son éditeur et son code. */
import CodeSwitch from "./CodeSwitch";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { EditorHistory } from "./TypstEditor";
import type { ViewOptions } from "./useViewOptions";

interface Props {
  /** Id du sous-onglet actif (`widget-code-<id>`). */
  tab: number | null;
  history: EditorHistory;
  onUndo: () => void;
  onRedo: () => void;
  options: Pick<ViewOptions, "code" | "setCode">;
}

export default function WidgetTools({ tab, history, onUndo, onRedo, options }: Props) {
  const { code, setCode } = options;
  return (
    <>
      <button className="token ghost" aria-label={t("ui-toolbar-undo")} title={t("ui-toolbar-undo")} disabled={history.undo === 0} onClick={onUndo}>{Icon.undo}</button>
      <button className="token ghost" aria-label={t("ui-toolbar-redo")} title={t("ui-toolbar-redo")} disabled={history.redo === 0} onClick={onRedo}>{Icon.redo}</button>
      <span className="tools-sep" aria-hidden />
      <CodeSwitch checked={code} controls={`widget-code-${tab}`} onChange={setCode} />
    </>
  );
}
