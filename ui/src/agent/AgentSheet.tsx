// Fiche d'une question ancrée sous l'élément visé (choix, reformulation).

import { t } from "../i18n";
import type { SurfaceProps } from "./AgentSurfaceBox";
import { AgentSurfaceBox } from "./AgentSurfaceBox";

/** La fiche : la surface de la question et le rappel des touches. */
export function AgentSheet(props: SurfaceProps) {
  return <AgentSurfaceBox {...props} className="agent-sheet-box" role="dialog" label={t("ui-agent-sheet")} caption={t("ui-agent-sheet-keys")} />;
}
