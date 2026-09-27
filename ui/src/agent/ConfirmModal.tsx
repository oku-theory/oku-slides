// Fenêtre de confirmation (ask_confirm, outils non annulables, permissions de l'agent).

import { t } from "../i18n";
import type { SurfaceProps } from "./AgentSurfaceBox";
import { AgentSurfaceBox } from "./AgentSurfaceBox";

/** La fenêtre, sur un voile qui couvre l'app ; répondre demande un clic (Échap annule). */
export function ConfirmModal(props: SurfaceProps) {
  return (
    <div className="agent-veil">
      <AgentSurfaceBox {...props} className="agent-modal" role="alertdialog" label={t("ui-agent-confirm")} modal />
    </div>
  );
}
