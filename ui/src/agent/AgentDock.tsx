// The dock: the agent's live questions, above the mic.

import type { ReactNode } from "react";
import { QuickHintsContext } from "../a2ui/quickHints";
import type { Surface } from "../api";
import { t } from "../i18n";
import { isQuestion } from "./surfaces";

/**
 * The live questions; nothing visible (0 px) without them, but the announced region stays
 * mounted: a question that arrives is read.
 */
export function AgentDock({
  surfaces,
  view,
  keysTo,
}: {
  surfaces: Surface[];
  view: (s: Surface) => ReactNode;
  /** La surface qui reçoit les touches rapides (`quickTarget`) ; par défaut la dernière question, au plus près du micro. */
  keysTo?: string | null;
}) {
  const keyed = keysTo === undefined ? (surfaces.filter(isQuestion).at(-1)?.id ?? null) : keysTo;
  return (
    <section
      className={`agent-dock agent-keys${surfaces.length === 0 ? " vacant" : ""}`}
      aria-label={t("ui-agent-dock")}
      aria-live="polite"
      aria-relevant="additions"
    >
      {surfaces.map((s) => (
        <div key={s.id} className={`agent-dock-item${isQuestion(s) ? " ask" : ""}`} data-surface-id={s.id}>
          {/* Les chiffres 1–4 seulement sur la question qui reçoit les touches. */}
          <QuickHintsContext.Provider value={s.id === keyed}>{view(s)}</QuickHintsContext.Provider>
        </div>
      ))}
    </section>
  );
}
