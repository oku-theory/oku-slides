// Le panneau des agents vu de la page (fenêtre principale ou de l'agent) : la dernière surface
// reçue de l'hôte (`onPicker` d'`AgentPanel`), la demander, choisir un agent, télécharger le
// modèle de l'agent local.

import { useCallback, useRef, useState } from "react";
import { errorText, type Surface } from "../api";
import type { AgentApi } from "./AgentPanel";

export interface AgentPickerState {
  /** La dernière surface `agent-picker` reçue, sinon `null`. */
  surface: Surface | null;
  /** À donner à `AgentPanel` : une surface reçue de l'hôte. */
  onPicker: (surface: Surface) => void;
  /** Demande le panneau à l'hôte (il le renvoie à jour). */
  open: () => void;
  /** Choisit un agent ; un refus est dit, et le panneau redemandé. */
  select: (id: string) => void;
  /** Télécharge le modèle de l'agent local ; un refus est dit. */
  download: () => void;
  /** Arrête le téléchargement du modèle ; un refus est dit. */
  cancelDownload: () => void;
}

export function useAgentPicker(agent: Pick<AgentApi, "picker" | "selectAgent" | "downloadModel" | "cancelDownload">, onError: (message: string) => void): AgentPickerState {
  const [surface, setSurface] = useState<Surface | null>(null);
  // Rappels stables : l'agent et le rapport d'erreur sont lus à l'appel.
  const latest = useRef({ agent, onError });
  latest.current = { agent, onError };
  const open = useCallback(() => {
    const { agent: a, onError: report } = latest.current;
    void a.picker().catch((e) => report(errorText(e)));
  }, []);
  // Un refus laisse la puce surlignée d'avance : l'hôte renvoie le panneau, elle revient au choix enregistré.
  const select = useCallback(
    (id: string) => {
      const { agent: a, onError: report } = latest.current;
      void a.selectAgent(id).catch((e) => {
        report(errorText(e));
        open();
      });
    },
    [open],
  );
  const download = useCallback(() => {
    const { agent: a, onError: report } = latest.current;
    void a.downloadModel().catch((e) => report(errorText(e)));
  }, []);
  const cancelDownload = useCallback(() => {
    const { agent: a, onError: report } = latest.current;
    void a.cancelDownload().catch((e) => report(errorText(e)));
  }, []);
  return { surface, onPicker: setSurface, open, select, download, cancelDownload };
}
