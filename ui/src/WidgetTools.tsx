/** Barre d'outils d'un sous-onglet de widget : l'historique de son éditeur, son code, sa place, la planche des cas. */
import BoardSwitch from "./BoardSwitch";
import CodePlaceSwitch from "./CodePlaceSwitch";
import CodeSwitch from "./CodeSwitch";
import { t } from "./i18n";
import { Icon } from "./icons";
import type { EditorHistory } from "./TypstEditor";
import type { ViewOptions } from "./useViewOptions";
import { COMMON } from "./widgetList";

interface Props {
  /** Id du sous-onglet actif (`widget-code-<id>`, `widget-panel-<id>`). */
  tab: number | null;
  /** Fichier du sous-onglet actif. */
  widget: string | null;
  history: EditorHistory;
  onUndo: () => void;
  onRedo: () => void;
  options: Pick<ViewOptions, "code" | "setCode" | "codeLayout" | "setCodeLayout" | "board" | "setBoard">;
}

export default function WidgetTools({ tab, widget, history, onUndo, onRedo, options }: Props) {
  const { code, setCode, codeLayout, setCodeLayout, board, setBoard } = options;
  return (
    <>
      <button className="token ghost" aria-label={t("ui-toolbar-undo")} title={t("ui-toolbar-undo")} disabled={history.undo === 0} onClick={onUndo}>{Icon.undo}</button>
      <button className="token ghost" aria-label={t("ui-toolbar-redo")} title={t("ui-toolbar-redo")} disabled={history.redo === 0} onClick={onRedo}>{Icon.redo}</button>
      <span className="tools-sep" aria-hidden />
      <CodeSwitch checked={code} controls={`widget-code-${tab}`} onChange={setCode} />
      {code && (
        <CodePlaceSwitch
          checked={codeLayout.code_place === "right"}
          controls={`widget-code-${tab}`}
          onChange={(right) => setCodeLayout({ ...codeLayout, code_place: right ? "right" : "below" })}
        />
      )}
      {/* Scène / Planche des cas : pas pour Commun. */}
      {widget !== COMMON && <BoardSwitch checked={board} controls={`widget-panel-${tab}`} onChange={setBoard} />}
    </>
  );
}
